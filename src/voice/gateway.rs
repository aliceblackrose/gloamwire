use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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

use crate::model::{GuildId, UserId};

use super::{
    DaveGatewayEvent, VOICE_GATEWAY_VERSION, VoiceCloseCode, VoiceConnectionInfo,
    VoiceEncryptionMode, VoiceError, VoiceGatewayEvent, VoiceReady, VoiceResult,
    VoiceSessionDescription, VoiceSpeakingEvent, VoiceSpeakingFlags, VoiceUdpDiscovery,
};

type VoiceSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const IDENTIFY_OPCODE: u8 = 0;
const SELECT_PROTOCOL_OPCODE: u8 = 1;
const READY_OPCODE: u8 = 2;
const HEARTBEAT_OPCODE: u8 = 3;
const SESSION_DESCRIPTION_OPCODE: u8 = 4;
const SPEAKING_OPCODE: u8 = 5;
const HEARTBEAT_ACK_OPCODE: u8 = 6;
const RESUME_OPCODE: u8 = 7;
const HELLO_OPCODE: u8 = 8;
const RESUMED_OPCODE: u8 = 9;
const CLIENTS_CONNECT_OPCODE: u8 = 11;
const CLIENT_DISCONNECT_OPCODE: u8 = 13;
const DAVE_FIRST_OPCODE: u8 = 21;
const DAVE_LAST_OPCODE: u8 = 31;

/// Configuration for one Discord Voice Gateway v8 session.
#[derive(Clone)]
pub struct VoiceGatewayConfig {
    guild_id: GuildId,
    user_id: UserId,
    session_id: String,
    token: String,
    endpoint: String,
    max_dave_protocol_version: u16,
}

impl std::fmt::Debug for VoiceGatewayConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceGatewayConfig")
            .field("guild_id", &self.guild_id)
            .field("user_id", &self.user_id)
            .field("session_id", &self.session_id)
            .field("token", &"[REDACTED]")
            .field("endpoint", &self.endpoint)
            .field("max_dave_protocol_version", &self.max_dave_protocol_version)
            .finish()
    }
}

impl VoiceGatewayConfig {
    /// Creates configuration from a completed main-Gateway voice rendezvous.
    #[must_use]
    pub fn new(info: VoiceConnectionInfo) -> Self {
        Self {
            guild_id: info.guild_id,
            user_id: info.user_id,
            session_id: info.session_id,
            token: info.token,
            endpoint: info.endpoint,
            max_dave_protocol_version: 0,
        }
    }

    /// Advertises the highest DAVE protocol version implemented by the caller.
    ///
    /// Gloamwire exposes DAVE Voice Gateway messages but does not yet implement
    /// MLS/media cryptography itself. Leave this at zero unless an external DAVE
    /// implementation is attached to the connection.
    #[must_use]
    pub const fn with_max_dave_protocol_version(mut self, version: u16) -> Self {
        self.max_dave_protocol_version = version;
        self
    }

    #[must_use]
    pub const fn max_dave_protocol_version(&self) -> u16 {
        self.max_dave_protocol_version
    }
}

/// Live Discord Voice Gateway v8 connection.
pub struct VoiceGatewayConnection {
    config: Arc<VoiceGatewayConfig>,
    socket: VoiceSocket,
    ready: VoiceReady,
    heartbeat: Interval,
    heartbeat_acknowledged: bool,
    last_heartbeat_sent: Option<Instant>,
    latency: Option<Duration>,
    sequence: Option<u16>,
    pending: VecDeque<VoiceGatewayEvent>,
    shutdown: bool,
}

impl std::fmt::Debug for VoiceGatewayConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceGatewayConnection")
            .field("config", &self.config)
            .field("ready", &self.ready)
            .field("heartbeat_acknowledged", &self.heartbeat_acknowledged)
            .field("latency", &self.latency)
            .field("sequence", &self.sequence)
            .field("shutdown", &self.shutdown)
            .finish_non_exhaustive()
    }
}

impl VoiceGatewayConnection {
    /// Opens the Voice Gateway, receives Hello, identifies, and waits for Ready.
    pub async fn connect(config: VoiceGatewayConfig) -> VoiceResult<Self> {
        let config = Arc::new(config);
        let (mut socket, heartbeat) = open_socket(&config.endpoint).await?;
        send_json(
            &mut socket,
            IDENTIFY_OPCODE,
            &Identify {
                server_id: config.guild_id,
                user_id: config.user_id,
                session_id: &config.session_id,
                token: &config.token,
                max_dave_protocol_version: config.max_dave_protocol_version,
            },
        )
        .await?;

        let mut sequence = None;
        let mut pending = VecDeque::new();
        let ready = loop {
            match next_inbound(&mut socket).await? {
                Inbound::Json(envelope) => {
                    update_sequence(&mut sequence, envelope.sequence);
                    if envelope.opcode == READY_OPCODE {
                        break serde_json::from_value::<VoiceReady>(envelope.data)?;
                    }
                    if envelope.opcode == HELLO_OPCODE {
                        continue;
                    }
                    if let Some(event) = event_from_envelope(envelope)? {
                        pending.push_back(event);
                    }
                }
                Inbound::DaveBinary(event) => {
                    update_sequence(&mut sequence, Some(event.sequence));
                    pending.push_back(VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                        opcode: event.opcode,
                        sequence: event.sequence,
                        payload: event.payload,
                    }));
                }
            }
        };

        Ok(Self {
            config,
            socket,
            ready,
            heartbeat,
            heartbeat_acknowledged: true,
            last_heartbeat_sent: None,
            latency: None,
            sequence,
            pending,
            shutdown: false,
        })
    }

    /// Returns the Voice Ready data, including SSRC, UDP endpoint, and modes.
    #[must_use]
    pub const fn ready(&self) -> &VoiceReady {
        &self.ready
    }

    /// Returns the most recently observed Voice Gateway v8 sequence number.
    #[must_use]
    pub const fn sequence(&self) -> Option<u16> {
        self.sequence
    }

    /// Returns latency measured from the latest heartbeat/ACK pair.
    #[must_use]
    pub const fn latency(&self) -> Option<Duration> {
        self.latency
    }

    /// Sends Voice Gateway opcode 1 after UDP IP discovery.
    pub async fn select_protocol(
        &mut self,
        discovery: &VoiceUdpDiscovery,
        mode: &VoiceEncryptionMode,
    ) -> VoiceResult<()> {
        if !self.ready.modes.iter().any(|advertised| advertised == mode) {
            return Err(VoiceError::Protocol(format!(
                "selected transport-encryption mode {} was not advertised by Discord",
                mode.as_ref()
            )));
        }

        send_json(
            &mut self.socket,
            SELECT_PROTOCOL_OPCODE,
            &SelectProtocol {
                protocol: "udp",
                data: SelectProtocolData {
                    address: &discovery.address,
                    port: discovery.port,
                    mode: mode.as_ref(),
                },
            },
        )
        .await
    }

    /// Sends Voice Gateway opcode 5. Discord requires at least one Speaking
    /// payload before audio is transmitted.
    pub async fn set_speaking(&mut self, flags: VoiceSpeakingFlags) -> VoiceResult<()> {
        if flags.bits() == 0 {
            return Err(VoiceError::Protocol(
                "Voice Speaking flags must be non-zero before audio transmission".to_owned(),
            ));
        }

        send_json(
            &mut self.socket,
            SPEAKING_OPCODE,
            &Speaking {
                speaking: flags.bits(),
                delay: 0,
                ssrc: self.ready.ssrc,
            },
        )
        .await
    }

    /// Sends a JSON DAVE client opcode for an external DAVE implementation.
    pub async fn send_dave_json<T>(&mut self, opcode: u8, data: &T) -> VoiceResult<()>
    where
        T: Serialize,
    {
        if !matches!(opcode, 23 | 31) {
            return Err(VoiceError::Protocol(format!(
                "Voice DAVE opcode {opcode} is not a client JSON opcode"
            )));
        }
        send_json(&mut self.socket, opcode, data).await
    }

    /// Sends a binary DAVE client opcode. Client binary messages contain the
    /// opcode byte followed directly by the DAVE payload and do not include a
    /// server sequence number.
    pub async fn send_dave_binary(&mut self, opcode: u8, payload: &[u8]) -> VoiceResult<()> {
        if !matches!(opcode, 26 | 28) {
            return Err(VoiceError::Protocol(format!(
                "Voice DAVE opcode {opcode} is not a client binary opcode"
            )));
        }

        let mut bytes = Vec::with_capacity(payload.len() + 1);
        bytes.push(opcode);
        bytes.extend_from_slice(payload);
        self.socket.send(Message::Binary(bytes.into())).await?;
        Ok(())
    }

    /// Returns the next meaningful Voice Gateway event while driving heartbeats.
    pub async fn next_event(&mut self) -> VoiceResult<VoiceGatewayEvent> {
        if self.shutdown {
            return Err(VoiceError::Protocol(
                "Voice Gateway connection has been shut down".to_owned(),
            ));
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }

        loop {
            tokio::select! {
                _ = self.heartbeat.tick() => {
                    if !self.heartbeat_acknowledged {
                        return Err(VoiceError::HeartbeatNotAcknowledged);
                    }
                    self.send_heartbeat().await?;
                }
                inbound = next_inbound(&mut self.socket) => {
                    match inbound? {
                        Inbound::Json(envelope) => {
                            update_sequence(&mut self.sequence, envelope.sequence);
                            if envelope.opcode == HEARTBEAT_ACK_OPCODE {
                                self.heartbeat_acknowledged = true;
                                self.latency = self.last_heartbeat_sent.take().map(|sent| sent.elapsed());
                                return Ok(VoiceGatewayEvent::HeartbeatAck);
                            }
                            if envelope.opcode == HELLO_OPCODE {
                                continue;
                            }
                            if let Some(event) = event_from_envelope(envelope)? {
                                return Ok(event);
                            }
                        }
                        Inbound::DaveBinary(event) => {
                            update_sequence(&mut self.sequence, Some(event.sequence));
                            return Ok(VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                                opcode: event.opcode,
                                sequence: event.sequence,
                                payload: event.payload,
                            }));
                        }
                    }
                }
            }
        }
    }

    /// Opens a new Voice Gateway socket and resumes this v8 session using the
    /// latest `seq_ack`. Buffered messages received before Resumed are queued.
    pub async fn resume(&mut self) -> VoiceResult<()> {
        let (mut socket, heartbeat) = open_socket(&self.config.endpoint).await?;
        send_json(
            &mut socket,
            RESUME_OPCODE,
            &Resume {
                server_id: self.config.guild_id,
                session_id: &self.config.session_id,
                token: &self.config.token,
                seq_ack: self.sequence,
            },
        )
        .await?;

        let mut buffered = VecDeque::new();
        loop {
            match next_inbound(&mut socket).await? {
                Inbound::Json(envelope) => {
                    update_sequence(&mut self.sequence, envelope.sequence);
                    if envelope.opcode == RESUMED_OPCODE {
                        break;
                    }
                    if envelope.opcode == HELLO_OPCODE {
                        continue;
                    }
                    if let Some(event) = event_from_envelope(envelope)? {
                        buffered.push_back(event);
                    }
                }
                Inbound::DaveBinary(event) => {
                    update_sequence(&mut self.sequence, Some(event.sequence));
                    buffered.push_back(VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                        opcode: event.opcode,
                        sequence: event.sequence,
                        payload: event.payload,
                    }));
                }
            }
        }

        self.socket = socket;
        self.heartbeat = heartbeat;
        self.heartbeat_acknowledged = true;
        self.last_heartbeat_sent = None;
        self.latency = None;
        self.pending.extend(buffered);
        self.pending.push_back(VoiceGatewayEvent::Resumed);
        Ok(())
    }

    /// Gracefully closes the Voice Gateway WebSocket.
    pub async fn shutdown(&mut self) -> VoiceResult<()> {
        if self.shutdown {
            return Ok(());
        }
        self.shutdown = true;
        self.socket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "Gloamwire voice shutdown".into(),
            })))
            .await?;
        Ok(())
    }

    async fn send_heartbeat(&mut self) -> VoiceResult<()> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        send_json(
            &mut self.socket,
            HEARTBEAT_OPCODE,
            &Heartbeat {
                t: nonce,
                seq_ack: self.sequence,
            },
        )
        .await?;
        self.last_heartbeat_sent = Some(Instant::now());
        self.heartbeat_acknowledged = false;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct VoiceEnvelope {
    op: u8,
    #[serde(default)]
    d: Value,
    #[serde(default)]
    seq: Option<u16>,
}

struct NormalizedEnvelope {
    opcode: u8,
    data: Value,
    sequence: Option<u16>,
}

struct DaveBinaryInbound {
    opcode: u8,
    sequence: u16,
    payload: Vec<u8>,
}

enum Inbound {
    Json(NormalizedEnvelope),
    DaveBinary(DaveBinaryInbound),
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
    server_id: GuildId,
    user_id: UserId,
    session_id: &'a str,
    token: &'a str,
    max_dave_protocol_version: u16,
}

#[derive(Debug, Serialize)]
struct Resume<'a> {
    server_id: GuildId,
    session_id: &'a str,
    token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    seq_ack: Option<u16>,
}

#[derive(Debug, Serialize)]
struct Heartbeat {
    t: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    seq_ack: Option<u16>,
}

#[derive(Debug, Serialize)]
struct SelectProtocol<'a> {
    protocol: &'static str,
    data: SelectProtocolData<'a>,
}

#[derive(Debug, Serialize)]
struct SelectProtocolData<'a> {
    address: &'a str,
    port: u16,
    mode: &'a str,
}

#[derive(Debug, Serialize)]
struct Speaking {
    speaking: u32,
    delay: u8,
    ssrc: u32,
}

async fn open_socket(endpoint: &str) -> VoiceResult<(VoiceSocket, Interval)> {
    let url = voice_gateway_url(endpoint);
    let (mut socket, _) = connect_async(url).await?;

    match next_inbound(&mut socket).await? {
        Inbound::Json(envelope) if envelope.opcode == HELLO_OPCODE => {
            let hello = serde_json::from_value::<Hello>(envelope.data)?;
            if hello.heartbeat_interval == 0 {
                return Err(VoiceError::Protocol(
                    "Voice Hello contained a zero heartbeat interval".to_owned(),
                ));
            }
            let interval = Duration::from_millis(hello.heartbeat_interval);
            let mut heartbeat = tokio::time::interval_at(Instant::now() + interval, interval);
            heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
            Ok((socket, heartbeat))
        }
        Inbound::Json(envelope) => Err(VoiceError::Protocol(format!(
            "expected Voice Hello opcode 8, received {}",
            envelope.opcode
        ))),
        Inbound::DaveBinary(event) => Err(VoiceError::Protocol(format!(
            "received binary Voice opcode {} before Hello",
            event.opcode
        ))),
    }
}

async fn next_inbound(socket: &mut VoiceSocket) -> VoiceResult<Inbound> {
    loop {
        let message = socket.next().await.ok_or_else(|| VoiceError::Closed {
            code: None,
            reason: "Voice Gateway WebSocket stream ended".to_owned(),
        })??;

        match message {
            Message::Text(text) => {
                let envelope = serde_json::from_str::<VoiceEnvelope>(text.as_str())?;
                return Ok(Inbound::Json(NormalizedEnvelope {
                    opcode: envelope.op,
                    data: envelope.d,
                    sequence: envelope.seq,
                }));
            }
            Message::Binary(bytes) => {
                if bytes.len() < 3 {
                    return Err(VoiceError::Protocol(
                        "Voice Gateway binary message was shorter than sequence + opcode"
                            .to_owned(),
                    ));
                }
                let sequence = u16::from_be_bytes([bytes[0], bytes[1]]);
                let opcode = bytes[2];
                if !(DAVE_FIRST_OPCODE..=DAVE_LAST_OPCODE).contains(&opcode) {
                    return Err(VoiceError::Protocol(format!(
                        "unexpected binary Voice Gateway opcode {opcode}"
                    )));
                }
                return Ok(Inbound::DaveBinary(DaveBinaryInbound {
                    opcode,
                    sequence,
                    payload: bytes[3..].to_vec(),
                }));
            }
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
            Message::Close(frame) => return Err(closed_error(frame)),
            Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

fn closed_error(frame: Option<CloseFrame>) -> VoiceError {
    let code = frame
        .as_ref()
        .map(|frame| VoiceCloseCode::from(u16::from(frame.code)));
    let reason = frame
        .map(|frame| frame.reason.to_string())
        .unwrap_or_else(|| "peer closed without a close frame".to_owned());
    VoiceError::Closed { code, reason }
}

fn event_from_envelope(envelope: NormalizedEnvelope) -> VoiceResult<Option<VoiceGatewayEvent>> {
    let event = match envelope.opcode {
        READY_OPCODE | HELLO_OPCODE => return Ok(None),
        SESSION_DESCRIPTION_OPCODE => VoiceGatewayEvent::SessionDescription(
            serde_json::from_value::<VoiceSessionDescription>(envelope.data)?,
        ),
        SPEAKING_OPCODE => VoiceGatewayEvent::Speaking(
            serde_json::from_value::<VoiceSpeakingEvent>(envelope.data)?,
        ),
        HEARTBEAT_ACK_OPCODE => VoiceGatewayEvent::HeartbeatAck,
        RESUMED_OPCODE => VoiceGatewayEvent::Resumed,
        CLIENTS_CONNECT_OPCODE => VoiceGatewayEvent::ClientsConnect(envelope.data),
        CLIENT_DISCONNECT_OPCODE => VoiceGatewayEvent::ClientDisconnect(envelope.data),
        DAVE_FIRST_OPCODE..=DAVE_LAST_OPCODE => VoiceGatewayEvent::Dave(DaveGatewayEvent::Json {
            opcode: envelope.opcode,
            sequence: envelope.sequence,
            data: envelope.data,
        }),
        _ => VoiceGatewayEvent::Unknown {
            opcode: envelope.opcode,
            sequence: envelope.sequence,
            data: envelope.data,
        },
    };
    Ok(Some(event))
}

async fn send_json<T>(socket: &mut VoiceSocket, opcode: u8, data: &T) -> VoiceResult<()>
where
    T: Serialize,
{
    let text = serde_json::to_string(&OutboundEnvelope {
        op: opcode,
        d: data,
    })?;
    socket.send(Message::Text(text.into())).await?;
    Ok(())
}

fn update_sequence(current: &mut Option<u16>, received: Option<u16>) {
    if let Some(sequence) = received {
        *current = Some(sequence);
    }
}

fn voice_gateway_url(endpoint: &str) -> String {
    let mut url = if endpoint.starts_with("wss://") || endpoint.starts_with("ws://") {
        endpoint.to_owned()
    } else {
        format!("wss://{endpoint}")
    };

    let authority_start = url.find("://").map_or(0, |index| index + 3);
    let query_start = url.find('?').unwrap_or(url.len());
    if !url[authority_start..query_start].contains('/') {
        url.insert(query_start, '/');
    }

    let separator = if url.contains('?') { '&' } else { '?' };
    url.push(separator);
    url.push_str("v=");
    url.push_str(&VOICE_GATEWAY_VERSION.to_string());
    url
}

#[cfg(test)]
mod tests {
    use super::voice_gateway_url;

    #[test]
    fn voice_gateway_url_adds_secure_scheme_path_and_v8() {
        assert_eq!(
            voice_gateway_url("voice.example.test:443"),
            "wss://voice.example.test:443/?v=8"
        );
    }

    #[test]
    fn voice_gateway_url_adds_path_before_existing_query() {
        assert_eq!(
            voice_gateway_url("ws://127.0.0.1:8080?fixture=1"),
            "ws://127.0.0.1:8080/?fixture=1&v=8"
        );
    }

    #[test]
    fn voice_gateway_url_preserves_existing_scheme_path_and_query() {
        assert_eq!(
            voice_gateway_url("ws://127.0.0.1:8080/socket?fixture=1"),
            "ws://127.0.0.1:8080/socket?fixture=1&v=8"
        );
    }
}
