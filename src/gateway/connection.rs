use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpStream, time::{Instant, Interval, MissedTickBehavior}};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::Message,
};

use crate::error::{Error, Result};

use super::{DispatchEvent, GatewayEvent, GatewayIntents};

const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const MAX_OUTBOUND_PAYLOAD_BYTES: usize = 4096;

type GatewaySocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Configuration used to create a Gateway connection.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    token: String,
    intents: GatewayIntents,
    url: String,
    shard: Option<[u32; 2]>,
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

/// A live Discord Gateway WebSocket connection.
///
/// Heartbeats are driven by calls to [`Self::next_event`]. Applications should
/// therefore continuously poll `next_event` while the connection is active.
pub struct GatewayConnection {
    socket: GatewaySocket,
    heartbeat: Interval,
    heartbeat_acknowledged: bool,
    sequence: Option<u64>,
}

impl std::fmt::Debug for GatewayConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayConnection")
            .field("heartbeat_acknowledged", &self.heartbeat_acknowledged)
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}

impl GatewayConnection {
    /// Opens the WebSocket, receives Hello, initializes heartbeats, and identifies.
    pub async fn connect(config: GatewayConfig) -> Result<Self> {
        let (mut socket, _) = connect_async(&config.url).await?;

        let hello = next_envelope(&mut socket).await?;
        if hello.op != 10 {
            return Err(Error::GatewayProtocol(format!(
                "expected Hello opcode 10, received {}",
                hello.op
            )));
        }

        let hello: Hello = serde_json::from_value(hello.d)?;
        if hello.heartbeat_interval == 0 {
            return Err(Error::GatewayProtocol(
                "Hello contained a zero heartbeat interval".to_owned(),
            ));
        }

        let heartbeat_interval = Duration::from_millis(hello.heartbeat_interval);
        let first_heartbeat = heartbeat_interval.mul_f64(fastrand::f64());
        let mut heartbeat = tokio::time::interval_at(
            Instant::now() + first_heartbeat,
            heartbeat_interval,
        );
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let identify = OutboundEnvelope {
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
        };
        send_json(&mut socket, &identify).await?;

        Ok(Self {
            socket,
            heartbeat,
            heartbeat_acknowledged: true,
            sequence: None,
        })
    }

    /// Returns the most recently observed Gateway sequence number.
    #[must_use]
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Waits for and returns the next meaningful Gateway event.
    ///
    /// This method also sends scheduled or server-requested heartbeats and
    /// detects zombied connections when an ACK is missed.
    pub async fn next_event(&mut self) -> Result<GatewayEvent> {
        loop {
            tokio::select! {
                _ = self.heartbeat.tick() => {
                    if !self.heartbeat_acknowledged {
                        return Err(Error::HeartbeatNotAcknowledged);
                    }

                    send_heartbeat(&mut self.socket, self.sequence).await?;
                    self.heartbeat_acknowledged = false;
                }
                envelope = next_envelope(&mut self.socket) => {
                    let envelope = envelope?;
                    if let Some(sequence) = envelope.s {
                        self.sequence = Some(sequence);
                    }

                    match envelope.op {
                        0 => {
                            let name = envelope.t.ok_or_else(|| {
                                Error::GatewayProtocol("dispatch event omitted its event name".to_owned())
                            })?;
                            let sequence = envelope.s.ok_or_else(|| {
                                Error::GatewayProtocol("dispatch event omitted its sequence".to_owned())
                            })?;
                            return Ok(GatewayEvent::Dispatch(DispatchEvent {
                                name,
                                sequence,
                                data: envelope.d,
                            }));
                        }
                        1 => {
                            send_heartbeat(&mut self.socket, self.sequence).await?;
                            self.heartbeat_acknowledged = false;
                        }
                        7 => return Ok(GatewayEvent::Reconnect),
                        9 => {
                            let resumable = serde_json::from_value::<bool>(envelope.d)?;
                            return Ok(GatewayEvent::InvalidSession { resumable });
                        }
                        11 => {
                            self.heartbeat_acknowledged = true;
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

async fn next_envelope(socket: &mut GatewaySocket) -> Result<InboundEnvelope> {
    loop {
        let message = socket
            .next()
            .await
            .ok_or_else(|| Error::GatewayClosed("WebSocket stream ended".to_owned()))??;

        match message {
            Message::Text(text) => return Ok(serde_json::from_str(text.as_ref())?),
            Message::Binary(bytes) => return Ok(serde_json::from_slice(&bytes)?),
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
            Message::Close(frame) => {
                return Err(Error::GatewayClosed(format!("{frame:?}")));
            }
            _ => {}
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
