use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    net::TcpStream,
    time::{Instant, Interval, MissedTickBehavior},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        Message,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};

use crate::error::{Error, Result};

use super::{
    DispatchEvent, GatewayCloseCode, GatewayEvent, GatewayIntents, GatewayReconnectStrategy,
    GatewaySession,
};

const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg";
const GATEWAY_VERSION: u8 = 10;
const MAX_OUTBOUND_PAYLOAD_BYTES: usize = 4096;
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(500);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

type GatewaySocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Configuration used to create a Gateway connection.
#[derive(Clone)]
pub struct GatewayConfig {
    token: String,
    intents: GatewayIntents,
    url: String,
    shard: Option<[u32; 2]>,
}

impl std::fmt::Debug for GatewayConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayConfig")
            .field("token", &"[REDACTED]")
            .field("intents", &self.intents)
            .field("url", &self.url)
            .field("shard", &self.shard)
            .finish()
    }
}

impl GatewayConfig {
    /// Creates Gateway configuration using Discord Gateway v10 and JSON encoding.
    #[must_use]
    pub fn new(token: impl Into<String>, intents: GatewayIntents) -> Self {
        Self {
            token: token.into(),
            intents,
            url: DEFAULT_GATEWAY_URL.to_owned(),
            shard: None,
        }
    }

    /// Overrides the Gateway WebSocket URL.
    ///
    /// Version and JSON-encoding query parameters are added when they are not
    /// already present.
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Configures the shard ID and total shard count sent in Identify.
    #[must_use]
    pub fn with_shard(mut self, shard_id: u32, shard_count: u32) -> Self {
        self.shard = Some([shard_id, shard_count]);
        self
    }
}

/// A live Discord Gateway connection with resumable session state.
///
/// Heartbeats and recoverable reconnects are driven by calls to
/// [`Self::next_event`]. Applications should continuously poll `next_event`
/// while the connection is active.
pub struct GatewayConnection {
    config: GatewayConfig,
    socket: GatewaySocket,
    heartbeat: Interval,
    heartbeat_acknowledged: bool,
    sequence: Option<u64>,
    session: Option<GatewaySession>,
    last_heartbeat_sent: Option<Instant>,
    latency: Option<Duration>,
    shutdown: bool,
}

impl std::fmt::Debug for GatewayConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayConnection")
            .field("config", &self.config)
            .field("heartbeat_acknowledged", &self.heartbeat_acknowledged)
            .field("sequence", &self.sequence)
            .field("session", &self.session)
            .field("latency", &self.latency)
            .field("shutdown", &self.shutdown)
            .finish_non_exhaustive()
    }
}

impl GatewayConnection {
    /// Opens the WebSocket, receives Hello, initializes heartbeats, and identifies.
    pub async fn connect(config: GatewayConfig) -> Result<Self> {
        let (socket, heartbeat) = open_and_handshake(&config, None).await?;

        Ok(Self {
            config,
            socket,
            heartbeat,
            heartbeat_acknowledged: true,
            sequence: None,
            session: None,
            last_heartbeat_sent: None,
            latency: None,
            shutdown: false,
        })
    }

    /// Returns the most recently observed Gateway sequence number.
    #[must_use]
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns resumable Gateway session state after a Ready event has been received.
    #[must_use]
    pub const fn session(&self) -> Option<&GatewaySession> {
        self.session.as_ref()
    }

    /// Returns the latency measured between the most recent heartbeat and ACK.
    #[must_use]
    pub const fn latency(&self) -> Option<Duration> {
        self.latency
    }

    /// Closes the Gateway with WebSocket close code 1000 and invalidates local
    /// resumable session state.
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.shutdown {
            return Ok(());
        }

        self.shutdown = true;
        self.session = None;
        self.sequence = None;

        self.socket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Gloamwire shutdown".into(),
            })))
            .await?;

        Ok(())
    }

    /// Waits for and returns the next meaningful Gateway event.
    ///
    /// This method sends scheduled or server-requested heartbeats, tracks
    /// resumable session state, and automatically reconnects recoverable
    /// connections. Opcode 7 and opcode 9 are still surfaced after recovery so
    /// callers can observe lifecycle changes without having to perform them.
    pub async fn next_event(&mut self) -> Result<GatewayEvent> {
        if self.shutdown {
            return Err(Error::GatewayClosed {
                code: Some(GatewayCloseCode::Normal),
                reason: "connection was shut down by the client".to_owned(),
            });
        }

        loop {
            tokio::select! {
                _ = self.heartbeat.tick() => {
                    if !self.heartbeat_acknowledged {
                        self.reconnect(GatewayReconnectStrategy::Resume).await?;
                        continue;
                    }

                    self.send_heartbeat().await?;
                }
                envelope = next_envelope(&mut self.socket) => {
                    let envelope = match envelope {
                        Ok(envelope) => envelope,
                        Err(Error::GatewayClosed { code, reason }) => {
                            let strategy = code
                                .map_or(GatewayReconnectStrategy::Resume, GatewayCloseCode::reconnect_strategy);

                            if strategy == GatewayReconnectStrategy::Stop {
                                return Err(Error::GatewayClosed { code, reason });
                            }

                            self.reconnect(strategy).await?;
                            continue;
                        }
                        Err(Error::WebSocket(_)) => {
                            self.reconnect(GatewayReconnectStrategy::Resume).await?;
                            continue;
                        }
                        Err(error) => return Err(error),
                    };

                    if let Some(sequence) = envelope.s {
                        self.update_sequence(sequence);
                    }

                    match envelope.op {
                        0 => return self.dispatch(envelope),
                        1 => self.send_heartbeat().await?,
                        7 => {
                            self.reconnect(GatewayReconnectStrategy::Resume).await?;
                            return Ok(GatewayEvent::Reconnect);
                        }
                        9 => {
                            let resumable = serde_json::from_value::<bool>(envelope.d)?;
                            let strategy = if resumable {
                                GatewayReconnectStrategy::Resume
                            } else {
                                GatewayReconnectStrategy::Reidentify
                            };
                            self.reconnect(strategy).await?;
                            return Ok(GatewayEvent::InvalidSession { resumable });
                        }
                        11 => {
                            self.heartbeat_acknowledged = true;
                            self.latency = self.last_heartbeat_sent.take().map(|sent| sent.elapsed());
                            return Ok(GatewayEvent::HeartbeatAck);
                        }
                        opcode => {
                            return Ok(GatewayEvent::Unknown {
                                opcode,
                                data: envelope.d,
                            });
                        }
                    }
                }
            }
        }
    }

    fn dispatch(&mut self, envelope: InboundEnvelope) -> Result<GatewayEvent> {
        let name = envelope.t.ok_or_else(|| {
            Error::GatewayProtocol("dispatch event omitted its event name".to_owned())
        })?;
        let sequence = envelope.s.ok_or_else(|| {
            Error::GatewayProtocol("dispatch event omitted its sequence".to_owned())
        })?;

        if name == "READY" {
            let ready = serde_json::from_value::<ReadySessionData>(envelope.d.clone())?;
            self.session = Some(GatewaySession::new(
                ready.session_id,
                ready.resume_gateway_url,
                sequence,
            ));
        }

        Ok(GatewayEvent::Dispatch(DispatchEvent {
            name,
            sequence,
            data: envelope.d,
        }))
    }

    fn update_sequence(&mut self, sequence: u64) {
        self.sequence = Some(sequence);
        if let Some(session) = &mut self.session {
            session.update_sequence(sequence);
        }
    }

    async fn send_heartbeat(&mut self) -> Result<()> {
        let sent_at = Instant::now();
        send_heartbeat(&mut self.socket, self.sequence).await?;
        self.last_heartbeat_sent = Some(sent_at);
        self.heartbeat_acknowledged = false;
        Ok(())
    }

    async fn reconnect(&mut self, mut strategy: GatewayReconnectStrategy) -> Result<()> {
        if strategy == GatewayReconnectStrategy::Resume && self.session.is_none() {
            strategy = GatewayReconnectStrategy::Reidentify;
        }

        if strategy == GatewayReconnectStrategy::Stop {
            return Err(Error::GatewayProtocol(
                "attempted to reconnect a non-reconnectable Gateway session".to_owned(),
            ));
        }

        if strategy == GatewayReconnectStrategy::Reidentify {
            self.session = None;
            self.sequence = None;
        }

        let config = self.config.clone();
        let session = match strategy {
            GatewayReconnectStrategy::Resume => self.session.clone(),
            GatewayReconnectStrategy::Reidentify | GatewayReconnectStrategy::Stop => None,
        };
        let mut attempt = 0_u32;

        loop {
            match open_and_handshake(&config, session.as_ref()).await {
                Ok((socket, heartbeat)) => {
                    self.socket = socket;
                    self.heartbeat = heartbeat;
                    self.heartbeat_acknowledged = true;
                    self.last_heartbeat_sent = None;
                    self.latency = None;
                    return Ok(());
                }
                Err(Error::WebSocket(_)) | Err(Error::GatewayClosed { .. }) => {
                    tokio::time::sleep(reconnect_delay(attempt)).await;
                    attempt = attempt.saturating_add(1);
                }
                Err(error) => return Err(error),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct InboundEnvelope {
    op: u8,
    #[serde(default)]
    d: Value,
    #[serde(default)]
    s: Option<u64>,
    #[serde(default)]
    t: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Hello {
    heartbeat_interval: u64,
}

#[derive(Debug, Deserialize)]
struct ReadySessionData {
    session_id: String,
    resume_gateway_url: String,
}

#[derive(Debug, Serialize)]
struct OutboundEnvelope<T> {
    op: u8,
    d: T,
}

#[derive(Debug, Serialize)]
struct Identify<'a> {
    token: &'a str,
    intents: u64,
    properties: IdentifyProperties<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shard: Option<[u32; 2]>,
}

#[derive(Debug, Serialize)]
struct IdentifyProperties<'a> {
    os: &'a str,
    browser: &'a str,
    device: &'a str,
}

#[derive(Debug, Serialize)]
struct Resume<'a> {
    token: &'a str,
    session_id: &'a str,
    seq: u64,
}

async fn open_and_handshake(
    config: &GatewayConfig,
    session: Option<&GatewaySession>,
) -> Result<(GatewaySocket, Interval)> {
    let base_url = session.map_or(config.url.as_str(), GatewaySession::resume_gateway_url);
    let url = gateway_url(base_url);
    let (mut socket, _) = connect_async(url).await?;

    let hello = next_envelope(&mut socket).await?;
    if hello.op != 10 {
        return Err(Error::GatewayProtocol(format!(
            "expected Hello opcode 10, received {}",
            hello.op
        )));
    }

    let hello = serde_json::from_value::<Hello>(hello.d)?;
    if hello.heartbeat_interval == 0 {
        return Err(Error::GatewayProtocol(
            "Hello contained a zero heartbeat interval".to_owned(),
        ));
    }

    let heartbeat_interval = Duration::from_millis(hello.heartbeat_interval);
    let first_heartbeat = heartbeat_interval.mul_f64(fastrand::f64());
    let mut heartbeat =
        tokio::time::interval_at(Instant::now() + first_heartbeat, heartbeat_interval);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    if let Some(session) = session {
        send_json(
            &mut socket,
            &OutboundEnvelope {
                op: 6,
                d: Resume {
                    token: &config.token,
                    session_id: session.session_id(),
                    seq: session.sequence(),
                },
            },
        )
        .await?;
    } else {
        send_json(
            &mut socket,
            &OutboundEnvelope {
                op: 2,
                d: Identify {
                    token: &config.token,
                    intents: config.intents.bits(),
                    properties: IdentifyProperties {
                        os: std::env::consts::OS,
                        browser: "gloamwire",
                        device: "gloamwire",
                    },
                    shard: config.shard,
                },
            },
        )
        .await?;
    }

    Ok((socket, heartbeat))
}

async fn next_envelope(socket: &mut GatewaySocket) -> Result<InboundEnvelope> {
    loop {
        let message = socket.next().await.ok_or_else(|| Error::GatewayClosed {
            code: None,
            reason: "WebSocket stream ended".to_owned(),
        })??;

        match message {
            Message::Text(text) => return Ok(serde_json::from_str(text.as_str())?),
            Message::Binary(bytes) => return Ok(serde_json::from_slice(&bytes)?),
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
            Message::Close(frame) => {
                let code = frame
                    .as_ref()
                    .map(|frame| GatewayCloseCode::from(u16::from(frame.code)));
                let reason = frame
                    .map(|frame| frame.reason.to_string())
                    .unwrap_or_else(|| "peer closed without a close frame".to_owned());

                return Err(Error::GatewayClosed { code, reason });
            }
            Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

async fn send_heartbeat(socket: &mut GatewaySocket, sequence: Option<u64>) -> Result<()> {
    send_json(socket, &OutboundEnvelope { op: 1, d: sequence }).await
}

async fn send_json<T>(socket: &mut GatewaySocket, payload: &T) -> Result<()>
where
    T: Serialize,
{
    let text = serde_json::to_string(payload)?;
    if text.len() > MAX_OUTBOUND_PAYLOAD_BYTES {
        return Err(Error::GatewayPayloadTooLarge {
            actual: text.len(),
            limit: MAX_OUTBOUND_PAYLOAD_BYTES,
        });
    }

    socket.send(Message::Text(text.into())).await?;
    Ok(())
}

fn gateway_url(base_url: &str) -> String {
    let mut url = base_url.trim_end_matches('/').to_owned();
    let mut separator = if url.contains('?') { '&' } else { '?' };

    if !url.contains("v=") {
        url.push(separator);
        url.push_str("v=");
        url.push_str(&GATEWAY_VERSION.to_string());
        separator = '&';
    }

    if !url.contains("encoding=") {
        url.push(separator);
        url.push_str("encoding=json");
    }

    url
}

fn reconnect_delay(attempt: u32) -> Duration {
    let exponent = attempt.min(6);
    let base_millis = INITIAL_RECONNECT_DELAY
        .as_millis()
        .saturating_mul(1_u128 << exponent)
        .min(MAX_RECONNECT_DELAY.as_millis());
    let jitter_ceiling = (base_millis / 2) as u64;
    let jitter = fastrand::u64(0..=jitter_ceiling);
    let millis = base_millis
        .saturating_add(u128::from(jitter))
        .min(MAX_RECONNECT_DELAY.as_millis()) as u64;

    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{gateway_url, reconnect_delay};

    #[test]
    fn gateway_url_adds_protocol_query() {
        assert_eq!(
            gateway_url("wss://gateway.discord.gg"),
            "wss://gateway.discord.gg?v=10&encoding=json"
        );
    }

    #[test]
    fn gateway_url_preserves_existing_query() {
        assert_eq!(
            gateway_url("wss://gateway.discord.gg?v=10&encoding=json"),
            "wss://gateway.discord.gg?v=10&encoding=json"
        );
    }

    #[test]
    fn reconnect_backoff_is_capped() {
        assert!(reconnect_delay(100) <= Duration::from_secs(30));
    }
}
