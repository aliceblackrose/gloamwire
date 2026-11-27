use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use super::{
    DaveProviderLifecycle, DaveVoiceSession, VoiceError, VoiceGatewayEvent, VoiceOpusFrame,
    VoiceOpusFrameDuration, VoiceReceivedOpus, VoiceResult, VoiceSpeakingFlags,
    session::VoiceInput,
};

/// Outbound operations for a managed voice session.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum VoiceCommand {
    SendOpus {
        payload: Vec<u8>,
        duration: VoiceOpusFrameDuration,
    },
    SetSpeaking(VoiceSpeakingFlags),
    FinishSpeaking,
}

type Command = (VoiceCommand, oneshot::Sender<VoiceResult<()>>);

/// Cloneable outbound voice handle.
#[derive(Debug, Clone)]
pub struct VoiceCommandSender(mpsc::Sender<Command>);

impl VoiceCommandSender {
    /// Queues an operation and waits for its actual result.
    pub async fn send(&self, command: VoiceCommand) -> VoiceResult<()> {
        let (reply, response) = oneshot::channel();
        self.0.send((command, reply)).await.map_err(|_| stopped())?;
        response.await.map_err(|_| stopped())?
    }
}

/// Independently drives the Voice Gateway, DAVE transitions and UDP media.
///
/// Control events are lossless until the bounded queue fills, at which point
/// the session stops. Inbound audio is best effort: malformed packets and
/// packets arriving when the audio queue is full are counted and discarded.
/// A terminal error is delivered through the control stream; reconnect by
/// establishing a fresh voice session after observing that error.
pub struct VoiceDriver {
    commands: VoiceCommandSender,
    events: mpsc::Receiver<VoiceGatewayEvent>,
    audio: Option<mpsc::Receiver<VoiceReceivedOpus>>,
    dropped: Arc<AtomicU64>,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<VoiceResult<()>>>,
}

impl std::fmt::Debug for VoiceDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceDriver").finish_non_exhaustive()
    }
}

impl VoiceDriver {
    /// Starts driving a negotiated voice session on the current Tokio runtime.
    #[must_use]
    pub fn spawn<P: DaveProviderLifecycle + Send + 'static>(session: DaveVoiceSession<P>) -> Self {
        let (commands, command_rx) = mpsc::channel(64);
        let (event_tx, events) = mpsc::channel(256);
        let (audio_tx, audio) = mpsc::channel(128);
        let (stop, stop_rx) = watch::channel(false);
        let dropped = Arc::new(AtomicU64::new(0));
        let task = tokio::spawn(run(
            session,
            command_rx,
            event_tx,
            audio_tx,
            Arc::clone(&dropped),
            stop_rx,
        ));
        Self {
            commands: VoiceCommandSender(commands),
            events,
            audio: Some(audio),
            dropped,
            stop,
            task: Some(task),
        }
    }

    /// Returns an independently usable outbound handle.
    #[must_use]
    pub fn commands(&self) -> VoiceCommandSender {
        self.commands.clone()
    }

    /// Receives control events and reports the terminal driver error once.
    pub async fn next_event(&mut self) -> Option<VoiceResult<VoiceGatewayEvent>> {
        if let Some(event) = self.events.recv().await {
            return Some(Ok(event));
        }
        let result = self.task.as_mut()?.await;
        self.task.take();
        match result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(Err(error)),
            Err(error) => Some(Err(VoiceError::Protocol(format!(
                "voice driver task failed: {error}"
            )))),
        }
    }

    /// Receives decoded Opus audio. Returns None when the driver stops.
    pub async fn next_audio(&mut self) -> Option<VoiceReceivedOpus> {
        self.audio.as_mut()?.recv().await
    }

    /// Transfers the inbound audio receiver to a separate application task.
    /// Subsequent calls to next_audio return None.
    pub fn take_audio_receiver(&mut self) -> Option<mpsc::Receiver<VoiceReceivedOpus>> {
        self.audio.take()
    }

    /// Total malformed, replayed or backpressured inbound audio packets dropped.
    #[must_use]
    pub fn dropped_audio_packets(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Requests shutdown and waits for the driver to exit.
    pub async fn shutdown(&mut self) -> VoiceResult<()> {
        self.events.close();
        if let Some(audio) = &mut self.audio {
            audio.close();
        }
        let _ = self.stop.send(true);
        if let Some(task) = self.task.as_mut() {
            match tokio::time::timeout(Duration::from_secs(10), &mut *task).await {
                Ok(result) => {
                    self.task.take();
                    result.map_err(|error| {
                        VoiceError::Protocol(format!("voice driver task failed: {error}"))
                    })??;
                }
                Err(_) => {
                    task.abort();
                    return Err(VoiceError::Timeout(Duration::from_secs(10)));
                }
            }
        }
        Ok(())
    }
}

impl Drop for VoiceDriver {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

fn stopped() -> VoiceError {
    VoiceError::Protocol("voice driver stopped".into())
}

async fn run<P: DaveProviderLifecycle + Send>(
    mut session: DaveVoiceSession<P>,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<VoiceGatewayEvent>,
    audio: mpsc::Sender<VoiceReceivedOpus>,
    dropped: Arc<AtomicU64>,
    mut stop: watch::Receiver<bool>,
) -> VoiceResult<()> {
    let mut buffer = vec![0; 65_536];
    let timeout = Duration::from_secs(5);
    loop {
        let input = tokio::select! {
            _ = stop.changed() => break,
            result = session.session_mut().next_input(commands.recv(), &mut buffer) => result?,
        };
        match input {
            VoiceInput::Gateway(event) => {
                tokio::time::timeout(timeout, session.apply_gateway_event(&event))
                    .await
                    .map_err(|_| VoiceError::Timeout(timeout))??;
                match events.try_send(event) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        return Err(VoiceError::Protocol(
                            "voice control event queue overflow".into(),
                        ));
                    }
                }
            }
            VoiceInput::Udp(received) => match session.decrypt_rtp_packet(&buffer[..received]) {
                Ok(frame) if !audio.is_closed() => {
                    if audio.try_send(frame).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
                _ => {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            },
            VoiceInput::External(Some((command, reply))) => {
                let result = tokio::time::timeout(timeout, async {
                    match command {
                        VoiceCommand::SendOpus { payload, duration } => session
                            .send_opus_frame(VoiceOpusFrame::new(&payload, duration)?)
                            .await
                            .map(|_| ()),
                        VoiceCommand::SetSpeaking(flags) => session.set_speaking(flags).await,
                        VoiceCommand::FinishSpeaking => session.finish_speaking().await,
                    }
                })
                .await;
                match result {
                    Ok(result) => {
                        let _ = reply.send(result);
                    }
                    Err(_) => {
                        let _ = reply.send(Err(VoiceError::Timeout(timeout)));
                        return Err(VoiceError::Timeout(timeout));
                    }
                }
            }
            VoiceInput::External(None) => break,
        }
    }
    session.shutdown().await
}
