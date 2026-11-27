use std::time::Duration;

use futures_util::future::Either;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use crate::{Error, Result};

use super::{
    GatewayConfig, GatewayConnection, GatewayEvent, RequestChannelInfo, RequestGuildMembers,
    RequestSoundboardSounds, UpdatePresence, UpdateVoiceState,
};

/// An outbound operation for an independently driven Gateway connection.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GatewayCommand {
    UpdatePresence(UpdatePresence),
    UpdateVoiceState(UpdateVoiceState),
    RequestGuildMembers(RequestGuildMembers),
    RequestSoundboardSounds(RequestSoundboardSounds),
    RequestChannelInfo(RequestChannelInfo),
}

type Command = (GatewayCommand, oneshot::Sender<Result<()>>);

/// Cloneable outbound handle; sending a command does not require the event receiver.
#[derive(Debug, Clone)]
pub struct GatewayCommandSender(mpsc::Sender<Command>);

impl GatewayCommandSender {
    /// Queues a command and awaits the actual write result.
    pub async fn send(&self, command: GatewayCommand) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.0
            .send((command, reply))
            .await
            .map_err(|_| stopped())?;
        response.await.map_err(|_| stopped())?
    }
}

/// Owns a background Gateway driver with a bounded event queue.
///
/// Heartbeats and recovery are driven independently of application polling.
/// If the event queue fills, the connection stops and reports an error after
/// queued events are drained rather than silently losing dispatches.
pub struct GatewayDriver {
    commands: GatewayCommandSender,
    events: mpsc::Receiver<GatewayEvent>,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<Result<()>>>,
}

impl std::fmt::Debug for GatewayDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayDriver").finish_non_exhaustive()
    }
}

impl GatewayDriver {
    /// Connects and starts the driver on the current Tokio runtime.
    pub async fn connect(config: GatewayConfig) -> Result<Self> {
        let connection = GatewayConnection::connect(config).await?;
        Ok(Self::spawn(connection))
    }

    /// Transfers an existing connection to the background driver.
    #[must_use]
    pub fn spawn(connection: GatewayConnection) -> Self {
        let (commands, command_rx) = mpsc::channel(64);
        let (event_tx, events) = mpsc::channel(256);
        let (stop, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run(connection, command_rx, event_tx, stop_rx));
        Self {
            commands: GatewayCommandSender(commands),
            events,
            stop,
            task: Some(task),
        }
    }

    /// Returns an independently usable outbound handle.
    #[must_use]
    pub fn commands(&self) -> GatewayCommandSender {
        self.commands.clone()
    }

    /// Receives a dispatch or lifecycle event; yields any terminal driver error once.
    pub async fn next_event(&mut self) -> Option<Result<GatewayEvent>> {
        if let Some(event) = self.events.recv().await {
            return Some(Ok(event));
        }
        let task = self.task.as_mut()?;
        let result = task.await;
        self.task.take();
        match result {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(Err(error)),
            Err(error) => Some(Err(error.into())),
        }
    }

    /// Requests shutdown and waits for the driver to exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.events.close();
        let _ = self.stop.send(true);
        if let Some(task) = self.task.as_mut() {
            match tokio::time::timeout(Duration::from_secs(10), &mut *task).await {
                Ok(result) => {
                    self.task.take();
                    result??;
                }
                Err(_) => {
                    task.abort();
                    return Err(Error::GatewayProtocol("Gateway driver shutdown timed out".into()));
                }
            }
        }
        Ok(())
    }
}

impl Drop for GatewayDriver {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

fn stopped() -> Error {
    Error::GatewayProtocol("Gateway driver stopped".into())
}

async fn run(
    mut connection: GatewayConnection,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<GatewayEvent>,
    mut stop: watch::Receiver<bool>,
) -> Result<()> {
    loop {
        let input = tokio::select! {
            _ = stop.changed() => break,
            result = connection.next_event_or(commands.recv()) => result?,
        };
        match input {
            Either::Left(event) => match events.try_send(event) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Closed(_)) => break,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    return Err(Error::GatewayProtocol("Gateway driver event queue overflow".into()));
                }
            },
            Either::Right(Some((command, reply))) => {
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    match command {
                        GatewayCommand::UpdatePresence(update) => connection.update_presence(&update).await,
                        GatewayCommand::UpdateVoiceState(update) => connection.update_voice_state(&update).await,
                        GatewayCommand::RequestGuildMembers(request) => connection.request_guild_members(&request).await,
                        GatewayCommand::RequestSoundboardSounds(request) => connection.request_soundboard_sounds(&request).await,
                        GatewayCommand::RequestChannelInfo(request) => connection.request_channel_info(&request).await,
                    }
                }).await;
                match result {
                    Ok(result) => { let _ = reply.send(result); }
                    Err(_) => {
                        let _ = reply.send(Err(Error::GatewayProtocol("Gateway command timed out".into())));
                        return Err(Error::GatewayProtocol("Gateway command timed out".into()));
                    }
                }
            }
            Either::Right(None) => break,
        }
    }
    connection.shutdown().await
}
