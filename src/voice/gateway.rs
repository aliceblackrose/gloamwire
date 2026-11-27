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
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        Message,
        protocol::{CloseFrame, WebSocketConfig, frame::coding::CloseCode},
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
const NEGOTIATION_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PENDING_EVENTS: usize = 32;
const MAX_INBOUND_BYTES: usize = 1024 * 1024;

/// Configuration for one Discord Voice Gateway v8 session.
#[derive(Clone)]
pub struct VoiceGatewayConfig {
    guild_id: GuildId,
    user_id: UserId,
    session_id: String,
    token: crate::secret::SecretString,
    endpoint: String,
    max_dave_protocol_version: u16,
    min_dave_protocol_version: u16,
    allow_insecure_loopback: bool,
    participant_limit: usize,
}

impl std::fmt::Debug for VoiceGatewayConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceGatewayConfig")
            .field("guild_id", &self.guild_id)
            .field("user_id", &self.user_id)
            .field("session_id", &"[REDACTED]")
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
            token: crate::secret::SecretString::new(info.token),
            endpoint: info.endpoint,
            max_dave_protocol_version: 0,
            min_dave_protocol_version: 0,
            allow_insecure_loopback: false,
            participant_limit: 4096,
        }
    }

    /// Advertises the highest DAVE protocol version implemented by the caller.
    ///
    /// Low-level Voice Gateway connections default to zero so provider choice
    /// remains explicit. Managed `DaveVoiceSession` connections derive and
    /// advertise the attached provider's supported version automatically; direct
    /// low-level users may set this when attaching their own DAVE provider.
    #[must_use]
    pub const fn with_max_dave_protocol_version(mut self, version: u16) -> Self {
        self.max_dave_protocol_version = version;
        self
    }

    /// Requires at least this DAVE version for managed media. Zero explicitly
    /// permits transport-only calls (the compatibility default).
    #[must_use]
    pub const fn with_min_dave_protocol_version(mut self, version: u16) -> Self {
        self.min_dave_protocol_version = version;
        self
    }

    /// Permits plaintext only for numeric loopback test endpoints.
    #[must_use]
    pub const fn with_insecure_loopback(mut self, allow: bool) -> Self {
        self.allow_insecure_loopback = allow;
        self
    }

    /// Bounds the managed MLS participant set.
    #[must_use]
    pub const fn with_participant_limit(mut self, limit: usize) -> Self {
        self.participant_limit = limit;
        self
    }

    pub(super) const fn min_dave_protocol_version(&self) -> u16 {
        self.min_dave_protocol_version
    }

    pub(super) const fn participant_limit(&self) -> usize {
        self.participant_limit
    }

    #[must_use]
    pub const fn max_dave_protocol_version(&self) -> u16 {
        self.max_dave_protocol_version
    }
}

/// Live Discord Voice Gateway v8 connection.
pub struct VoiceGatewayConnection {
    config: Arc<VoiceGatewayConfig>,
    socket: Option<VoiceSocket>,
    ready: VoiceReady,
    heartbeat: Interval,
    heartbeat_acknowledged: bool,
    last_heartbeat_sent: Option<(Instant, u64)>,
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
        let (mut socket, mut heartbeat) = open_socket(&config.endpoint, config.allow_insecure_loopback).await?;
        send_json(
            &mut socket,
            IDENTIFY_OPCODE,
            &Identify {
                server_id: config.guild_id,
                user_id: config.user_id,
                session_id: &config.session_id,
                token: config.token.expose(),
                max_dave_protocol_version: config.max_dave_protocol_version,
            },
        )
        .await?;

        let mut sequence = None;
        let mut pending = VecDeque::new();
        let mut sent = None;
        let deadline = Instant::now() + NEGOTIATION_TIMEOUT;
        let ready = loop {
            if pending.len() >= MAX_PENDING_EVENTS {
                return Err(VoiceError::Protocol(
                    "Voice Ready event buffer exceeded".into(),
                ));
            }
            match negotiation_inbound(&mut socket, &mut heartbeat, sequence, &mut sent, deadline)
                .await?
            {
                Inbound::Json(envelope) => {
                    let received = envelope.sequence;
                    if envelope.opcode == READY_OPCODE {
                        let ready = serde_json::from_value::<VoiceReady>(envelope.data)?;
                        update_sequence(&mut sequence, received);
                        break ready;
                    }
                    if envelope.opcode == HELLO_OPCODE {
                        continue;
                    }
                    if let Some(event) = event_from_envelope(envelope)? {
                        super::DaveProtocolEvent::from_gateway_event(&event)?;
                        update_sequence(&mut sequence, received);
                        pending.push_back(event);
                    }
                }
                Inbound::DaveBinary(event) => {
                    let received = event.sequence;
                    let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                        opcode: event.opcode,
                        sequence: received,
                        payload: event.payload,
                    });
                    super::DaveProtocolEvent::from_gateway_event(&event)?;
                    update_sequence(&mut sequence, Some(received));
                    pending.push_back(event);
                }
            }
        };

        Ok(Self {
            config,
            socket: Some(socket),
            ready,
            heartbeat,
            heartbeat_acknowledged: sent.is_none(),
            last_heartbeat_sent: sent,
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
            self.socket.as_mut().ok_or_else(transport_unavailable)?,
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

    /// Sends Voice Gateway opcode 5 Speaking state.
    ///
    /// Discord requires a non-zero Speaking payload before audio transmission.
    /// A zero bitmask is valid after transmission and clears the speaking state.
    pub async fn set_speaking(&mut self, flags: VoiceSpeakingFlags) -> VoiceResult<()> {
        send_json(
            self.socket.as_mut().ok_or_else(transport_unavailable)?,
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
        send_json(
            self.socket.as_mut().ok_or_else(transport_unavailable)?,
            opcode,
            data,
        )
        .await
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

        if payload.len() >= MAX_INBOUND_BYTES {
            return Err(VoiceError::Protocol(
                "oversized outbound Voice Gateway binary payload".into(),
            ));
        }
        let mut bytes = Vec::with_capacity(payload.len() + 1);
        bytes.push(opcode);
        bytes.extend_from_slice(payload);
        write_message(
            self.socket.as_mut().ok_or_else(transport_unavailable)?,
            Message::Binary(bytes.into()),
        )
        .await?;
        Ok(())
    }

    /// Returns the next meaningful Voice Gateway event while driving heartbeats.
    pub async fn next_event(&mut self) -> VoiceResult<VoiceGatewayEvent> {
        match self
            .next_event_or(std::future::pending::<std::convert::Infallible>())
            .await?
        {
            futures_util::future::Either::Left(event) => Ok(event),
            futures_util::future::Either::Right(never) => match never {},
        }
    }

    pub(super) async fn next_event_or<F: std::future::Future>(
        &mut self,
        external: F,
    ) -> VoiceResult<futures_util::future::Either<VoiceGatewayEvent, F::Output>> {
        use futures_util::future::Either;
        tokio::pin!(external);
        if self.shutdown {
            return Err(VoiceError::Protocol(
                "Voice Gateway connection has been shut down".to_owned(),
            ));
        }
        if let Some(event) = self.pending.pop_front() {
            return Ok(Either::Left(event));
        }

        loop {
            tokio::select! {
                output = &mut external => return Ok(Either::Right(output)),
                _ = self.heartbeat.tick() => {
                    if !self.heartbeat_acknowledged {
                        return Err(VoiceError::HeartbeatNotAcknowledged);
                    }
                    self.send_heartbeat().await?;
                }
                inbound = next_inbound(self.socket.as_mut().ok_or_else(transport_unavailable)?) => {
                    match inbound? {
                        Inbound::Json(envelope) => {
                            let received = envelope.sequence;
                            if envelope.opcode == HEARTBEAT_ACK_OPCODE {
                                if let Some((sent, nonce)) = self.last_heartbeat_sent
                                    && heartbeat_ack_matches(&envelope.data, nonce)
                                {
                                    update_sequence(&mut self.sequence, received);
                                    self.heartbeat_acknowledged = true;
                                    self.last_heartbeat_sent = None;
                                    self.latency = Some(sent.elapsed());
                                    return Ok(Either::Left(VoiceGatewayEvent::HeartbeatAck));
                                }
                                continue;
                            }
                            if envelope.opcode == HELLO_OPCODE {
                                continue;
                            }
                            if let Some(event) = event_from_envelope(envelope)? {
                                super::DaveProtocolEvent::from_gateway_event(&event)?;
                                update_sequence(&mut self.sequence, received);
                                return Ok(Either::Left(event));
                            }
                        }
                        Inbound::DaveBinary(event) => {
                            let received = event.sequence;
                            let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                                opcode: event.opcode,
                                sequence: received,
                                payload: event.payload,
                            });
                            super::DaveProtocolEvent::from_gateway_event(&event)?;
                            update_sequence(&mut self.sequence, Some(received));
                            return Ok(Either::Left(event));
                        }
                    }
                }
            }
        }
    }

    /// Opens a new Voice Gateway socket and resumes this v8 session using the
    /// latest `seq_ack`. Buffered messages received before Resumed are queued.
    pub async fn resume(&mut self) -> VoiceResult<()> {
        if self.shutdown {
            return Err(transport_unavailable());
        }
        // Retire the previous socket before initiating the replacement handshake.
        self.socket.take();
        // A stale synthetic Resumed event must not authorize media on a later recovery.
        self.pending
            .retain(|event| !matches!(event, VoiceGatewayEvent::Resumed));
        let (mut socket, mut heartbeat) = open_socket(&self.config.endpoint, self.config.allow_insecure_loopback).await?;
        send_json(
            &mut socket,
            RESUME_OPCODE,
            &Resume {
                server_id: self.config.guild_id,
                session_id: &self.config.session_id,
                token: self.config.token.expose(),
                seq_ack: self.sequence,
            },
        )
        .await?;

        let mut buffered = VecDeque::new();
        let mut sequence = self.sequence;
        let mut sent = None;
        let deadline = Instant::now() + NEGOTIATION_TIMEOUT;
        loop {
            if self.pending.len().saturating_add(buffered.len()) >= MAX_PENDING_EVENTS {
                return Err(VoiceError::Protocol(
                    "Voice Resume event buffer exceeded".into(),
                ));
            }
            match negotiation_inbound(&mut socket, &mut heartbeat, sequence, &mut sent, deadline)
                .await?
            {
                Inbound::Json(envelope) => {
                    let received = envelope.sequence;
                    if envelope.opcode == RESUMED_OPCODE {
                        update_sequence(&mut sequence, received);
                        break;
                    }
                    if envelope.opcode == HELLO_OPCODE {
                        continue;
                    }
                    if let Some(event) = event_from_envelope(envelope)? {
                        super::DaveProtocolEvent::from_gateway_event(&event)?;
                        update_sequence(&mut sequence, received);
                        buffered.push_back(event);
                    }
                }
                Inbound::DaveBinary(event) => {
                    let received = event.sequence;
                    let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
                        opcode: event.opcode,
                        sequence: received,
                        payload: event.payload,
                    });
                    super::DaveProtocolEvent::from_gateway_event(&event)?;
                    update_sequence(&mut sequence, Some(received));
                    buffered.push_back(event);
                }
            }
        }

        self.socket = Some(socket);
        self.heartbeat = heartbeat;
        self.heartbeat_acknowledged = sent.is_none();
        self.last_heartbeat_sent = sent;
        self.latency = None;
        self.sequence = sequence;
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
        self.pending.clear();
        if let Some(mut socket) = self.socket.take() {
            write_message(
                &mut socket,
                Message::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "Gloamwire voice shutdown".into(),
                })),
            )
            .await?;
        }
        Ok(())
    }

    async fn send_heartbeat(&mut self) -> VoiceResult<()> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        send_json(
            self.socket.as_mut().ok_or_else(transport_unavailable)?,
            HEARTBEAT_OPCODE,
            &Heartbeat {
                t: nonce,
                seq_ack: self.sequence,
            },
        )
        .await?;
        self.last_heartbeat_sent = Some((Instant::now(), nonce));
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

async fn open_socket(endpoint: &str, allow_loopback: bool) -> VoiceResult<(VoiceSocket, Interval)> {
    let url = voice_gateway_url(endpoint);
    if !crate::secret::secure_websocket_endpoint(&url, allow_loopback) {
        return Err(VoiceError::Protocol("Voice Gateway endpoint must use TLS (except explicitly enabled loopback fixtures)".into()));
    }
    let (mut socket, _) = tokio::time::timeout(
        NEGOTIATION_TIMEOUT,
        connect_async_with_config(
            url,
            Some(
                WebSocketConfig::default()
                    .max_message_size(Some(MAX_INBOUND_BYTES))
                    .max_frame_size(Some(MAX_INBOUND_BYTES)),
            ),
            false,
        ),
    )
    .await
    .map_err(|_| VoiceError::Timeout(NEGOTIATION_TIMEOUT))??;

    match tokio::time::timeout(NEGOTIATION_TIMEOUT, next_inbound(&mut socket))
        .await
        .map_err(|_| VoiceError::Timeout(NEGOTIATION_TIMEOUT))??
    {
        Inbound::Json(envelope) if envelope.opcode == HELLO_OPCODE => {
            let hello = serde_json::from_value::<Hello>(envelope.data)?;
            if !(1..=300_000).contains(&hello.heartbeat_interval) {
                return Err(VoiceError::Protocol(
                    "Voice Hello contained an invalid heartbeat interval".to_owned(),
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

async fn negotiation_inbound(
    socket: &mut VoiceSocket,
    heartbeat: &mut Interval,
    sequence: Option<u16>,
    sent: &mut Option<(Instant, u64)>,
    deadline: Instant,
) -> VoiceResult<Inbound> {
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                return Err(VoiceError::Timeout(NEGOTIATION_TIMEOUT));
            }
            _ = heartbeat.tick() => {
                if sent.is_some() {
                    return Err(VoiceError::HeartbeatNotAcknowledged);
                }
                let t = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                tokio::time::timeout_at(
                    deadline,
                    send_json(socket, HEARTBEAT_OPCODE, &Heartbeat { t, seq_ack: sequence }),
                ).await.map_err(|_| VoiceError::Timeout(NEGOTIATION_TIMEOUT))??;
                *sent = Some((Instant::now(), t));
            }
            inbound = next_inbound(socket) => {
                let inbound = inbound?;
                if let Inbound::Json(envelope) = &inbound
                    && envelope.opcode == HEARTBEAT_ACK_OPCODE
                {
                    if sent.is_some_and(|(_, nonce)| heartbeat_ack_matches(&envelope.data, nonce)) {
                        *sent = None;
                    } else {
                        continue;
                    }
                }
                return Ok(inbound);
            }
        }
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
                if text.len() > MAX_INBOUND_BYTES {
                    return Err(VoiceError::Protocol(
                        "oversized Voice Gateway message".into(),
                    ));
                }
                let envelope = serde_json::from_str::<VoiceEnvelope>(text.as_str())?;
                return Ok(Inbound::Json(NormalizedEnvelope {
                    opcode: envelope.op,
                    data: envelope.d,
                    sequence: envelope.seq,
                }));
            }
            Message::Binary(bytes) => {
                if bytes.len() > MAX_INBOUND_BYTES {
                    return Err(VoiceError::Protocol(
                        "oversized Voice Gateway message".into(),
                    ));
                }
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
            Message::Ping(payload) => write_message(socket, Message::Pong(payload)).await?,
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

fn heartbeat_ack_matches(data: &Value, nonce: u64) -> bool {
    // Voice v8 acknowledges with the sent timestamp in d.t.
    data.get("t").and_then(Value::as_u64) == Some(nonce) || data.as_u64() == Some(nonce)
}

fn transport_unavailable() -> VoiceError {
    VoiceError::Protocol("Voice Gateway transport unavailable".into())
}

async fn write_message(socket: &mut VoiceSocket, message: Message) -> VoiceResult<()> {
    let timeout = Duration::from_secs(5);
    tokio::time::timeout(timeout, socket.send(message))
        .await
        .map_err(|_| VoiceError::Timeout(timeout))??;
    Ok(())
}

async fn send_json<T>(socket: &mut VoiceSocket, opcode: u8, data: &T) -> VoiceResult<()>
where
    T: Serialize,
{
    let text = serde_json::to_string(&OutboundEnvelope {
        op: opcode,
        d: data,
    })?;
    write_message(socket, Message::Text(text.into())).await
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
    fn heartbeat_ack_requires_matching_nonce() {
        assert!(super::heartbeat_ack_matches(
            &serde_json::json!({"t":123}),
            123
        ));
        assert!(!super::heartbeat_ack_matches(
            &serde_json::json!({"t":122}),
            123
        ));
        assert!(!super::heartbeat_ack_matches(&serde_json::json!({}), 123));
    }

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
