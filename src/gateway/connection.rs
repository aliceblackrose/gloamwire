use std::{sync::Arc, time::Duration};

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

use crate::{
    error::{Error, Result},
    http::GatewayBot,
};

use super::{
    DispatchEvent, GatewayCloseCode, GatewayEvent, GatewayIntents, GatewayReconnectStrategy,
    GatewaySession, RequestChannelInfo, RequestGuildMembers, RequestSoundboardSounds,
    UpdatePresence, UpdateVoiceState,
    compression::{GatewayCompression, GatewayDecoder},
    encoding::{EncodedGatewayPayload, GatewayEncoding},
    identify::GatewayIdentifyLimiter,
    rate_limit::{GatewayRateLimiter, OutboundPriority},
};

const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg";
const GATEWAY_VERSION: u8 = 10;
const MAX_OUTBOUND_PAYLOAD_BYTES: usize = 4096;
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(500);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);
const UPDATE_PRESENCE_OPCODE: u8 = 3;
const UPDATE_VOICE_STATE_OPCODE: u8 = 4;
const REQUEST_GUILD_MEMBERS_OPCODE: u8 = 8;
const REQUEST_SOUNDBOARD_SOUNDS_OPCODE: u8 = 31;
const REQUEST_CHANNEL_INFO_OPCODE: u8 = 43;

type GatewaySocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Configuration used to create a Gateway connection.
#[derive(Clone)]
pub struct GatewayConfig {
    token: String,
    intents: GatewayIntents,
    url: String,
    shard: Option<[u32; 2]>,
    identify_limiter: Option<GatewayIdentifyLimiter>,
    encoding: GatewayEncoding,
    compression: GatewayCompression,
}

impl std::fmt::Debug for GatewayConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayConfig")
            .field("token", &"[REDACTED]")
            .field("intents", &self.intents)
            .field("url", &self.url)
            .field("shard", &self.shard)
            .field("identify_limited", &self.identify_limiter.is_some())
            .field("encoding", &self.encoding)
            .field("compression", &self.compression)
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
            identify_limiter: None,
            encoding: GatewayEncoding::Json,
            compression: GatewayCompression::None,
        }
    }

    /// Creates configuration from Discord's Get Gateway Bot response.
    ///
    /// This uses Discord's discovered Gateway URL and shares the returned
    /// session-start limits across clones of the configuration.
    #[must_use]
    pub fn from_gateway_bot(
        token: impl Into<String>,
        intents: GatewayIntents,
        gateway: &GatewayBot,
    ) -> Self {
        Self {
            token: token.into(),
            intents,
            url: gateway.url.clone(),
            shard: None,
            identify_limiter: Some(GatewayIdentifyLimiter::new(&gateway.session_start_limit)),
            encoding: GatewayEncoding::Json,
            compression: GatewayCompression::None,
        }
    }

    /// Overrides the Gateway WebSocket URL.
    ///
    /// Gloamwire normalizes the API version, encoding, and transport-compression
    /// query parameters when the connection is opened.
    #[must_use]
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Configures Discord Gateway wire encoding.
    #[must_use]
    pub const fn with_encoding(mut self, encoding: GatewayEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Returns the configured Gateway wire encoding.
    #[must_use]
    pub const fn encoding(&self) -> GatewayEncoding {
        self.encoding
    }

    /// Configures Discord Gateway transport compression.
    #[must_use]
    pub const fn with_compression(mut self, compression: GatewayCompression) -> Self {
        self.compression = compression;
        self
    }

    /// Returns the configured Gateway transport compression mode.
    #[must_use]
    pub const fn compression(&self) -> GatewayCompression {
        self.compression
    }

    /// Configures the shard ID and total shard count sent in Identify.
    #[must_use]
    pub fn with_shard(mut self, shard_id: u32, shard_count: u32) -> Self {
        self.shard = Some([shard_id, shard_count]);
        self
    }

    pub(crate) fn shard_id(&self) -> u32 {
        self.shard.map_or(0, |[shard_id, _]| shard_id)
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
    decoder: GatewayDecoder,
    heartbeat: Interval,
    heartbeat_acknowledged: bool,
    sequence: Option<u64>,
    session: Option<GatewaySession>,
    last_heartbeat_sent: Option<Instant>,
    latency: Option<Duration>,
    shutdown: bool,
    rate_limiter: Arc<GatewayRateLimiter>,
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
        let rate_limiter = Arc::new(GatewayRateLimiter::default());
        let (socket, decoder, heartbeat) = open_and_handshake(&config, None, &rate_limiter).await?;

        Ok(Self {
            config,
            socket,
            decoder,
            heartbeat,
            heartbeat_acknowledged: true,
            sequence: None,
            session: None,
            last_heartbeat_sent: None,
            latency: None,
            shutdown: false,
            rate_limiter,
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

    /// Sends Gateway opcode 3 (Update Presence).
    pub async fn update_presence(&mut self, update: &UpdatePresence) -> Result<()> {
        self.send_event(UPDATE_PRESENCE_OPCODE, update).await
    }

    /// Sends Gateway opcode 4 (Update Voice State).
    pub async fn update_voice_state(&mut self, update: &UpdateVoiceState) -> Result<()> {
        self.send_event(UPDATE_VOICE_STATE_OPCODE, update).await
    }

    /// Sends Gateway opcode 8 (Request Guild Members).
    pub async fn request_guild_members(&mut self, request: &RequestGuildMembers) -> Result<()> {
        self.send_event(REQUEST_GUILD_MEMBERS_OPCODE, request).await
    }

    /// Sends Gateway opcode 31 (Request Soundboard Sounds).
    pub async fn request_soundboard_sounds(
        &mut self,
        request: &RequestSoundboardSounds,
    ) -> Result<()> {
        self.send_event(REQUEST_SOUNDBOARD_SOUNDS_OPCODE, request)
            .await
    }

    /// Sends Gateway opcode 43 (Request Channel Info).
    pub async fn request_channel_info(&mut self, request: &RequestChannelInfo) -> Result<()> {
        self.send_event(REQUEST_CHANNEL_INFO_OPCODE, request).await
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
                envelope = next_envelope(&mut self.socket, &mut self.decoder, self.config.encoding) => {
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

    async fn send_event<T>(&mut self, opcode: u8, data: &T) -> Result<()>
    where
        T: Serialize,
    {
        if self.shutdown {
            return Err(Error::GatewayClosed {
                code: Some(GatewayCloseCode::Normal),
                reason: "connection was shut down by the client".to_owned(),
            });
        }

        let payload = encode_payload(
            self.config.encoding,
            &OutboundEnvelope {
                op: opcode,
                d: data,
            },
        )?;
        if let Some(retry_after) = self
            .rate_limiter
            .try_acquire(OutboundPriority::Normal)
            .await
        {
            return Err(Error::GatewayOutboundRateLimited { retry_after });
        }

        send_payload(&mut self.socket, payload).await
    }

    async fn send_heartbeat(&mut self) -> Result<()> {
        let sent_at = Instant::now();
        send_heartbeat(
            &mut self.socket,
            &self.rate_limiter,
            self.config.encoding,
            self.sequence,
        )
        .await?;
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
            let rate_limiter = Arc::new(GatewayRateLimiter::default());
            match open_and_handshake(&config, session.as_ref(), &rate_limiter).await {
                Ok((socket, decoder, heartbeat)) => {
                    self.socket = socket;
                    self.decoder = decoder;
                    self.heartbeat = heartbeat;
                    self.heartbeat_acknowledged = true;
                    self.last_heartbeat_sent = None;
                    self.latency = None;
                    self.rate_limiter = rate_limiter;
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
    rate_limiter: &GatewayRateLimiter,
) -> Result<(GatewaySocket, GatewayDecoder, Interval)> {
    let base_url = session.map_or(config.url.as_str(), GatewaySession::resume_gateway_url);
    let url = gateway_url(base_url, config.encoding, config.compression);
    let (mut socket, _) = connect_async(url).await?;
    let mut decoder = GatewayDecoder::new(config.compression)?;

    let hello = next_envelope(&mut socket, &mut decoder, config.encoding).await?;
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
        send_encoded(
            &mut socket,
            rate_limiter,
            config.encoding,
            OutboundPriority::Normal,
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
        if let Some(identify_limiter) = &config.identify_limiter {
            identify_limiter.acquire(config.shard_id()).await;
        }

        send_encoded(
            &mut socket,
            rate_limiter,
            config.encoding,
            OutboundPriority::Normal,
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

    Ok((socket, decoder, heartbeat))
}

async fn next_envelope(
    socket: &mut GatewaySocket,
    decoder: &mut GatewayDecoder,
    encoding: GatewayEncoding,
) -> Result<InboundEnvelope> {
    loop {
        let message = socket.next().await.ok_or_else(|| Error::GatewayClosed {
            code: None,
            reason: "WebSocket stream ended".to_owned(),
        })??;

        match message {
            Message::Text(text) => return encoding.decode_text(text.as_str()),
            Message::Binary(bytes) => {
                if let Some(decoded) = decoder.decode(&bytes)? {
                    return encoding.decode_bytes(&decoded);
                }
            }
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

async fn send_heartbeat(
    socket: &mut GatewaySocket,
    rate_limiter: &GatewayRateLimiter,
    encoding: GatewayEncoding,
    sequence: Option<u64>,
) -> Result<()> {
    send_encoded(
        socket,
        rate_limiter,
        encoding,
        OutboundPriority::Heartbeat,
        &OutboundEnvelope { op: 1, d: sequence },
    )
    .await
}

async fn send_encoded<T>(
    socket: &mut GatewaySocket,
    rate_limiter: &GatewayRateLimiter,
    encoding: GatewayEncoding,
    priority: OutboundPriority,
    payload: &T,
) -> Result<()>
where
    T: Serialize,
{
    let payload = encode_payload(encoding, payload)?;
    rate_limiter.acquire(priority).await;
    send_payload(socket, payload).await
}

fn encode_payload<T>(encoding: GatewayEncoding, payload: &T) -> Result<EncodedGatewayPayload>
where
    T: Serialize,
{
    let payload = encoding.encode(payload)?;
    if payload.len() > MAX_OUTBOUND_PAYLOAD_BYTES {
        return Err(Error::GatewayPayloadTooLarge {
            actual: payload.len(),
            limit: MAX_OUTBOUND_PAYLOAD_BYTES,
        });
    }

    Ok(payload)
}

async fn send_payload(socket: &mut GatewaySocket, payload: EncodedGatewayPayload) -> Result<()> {
    let message = match payload {
        EncodedGatewayPayload::Text(text) => Message::Text(text.into()),
        EncodedGatewayPayload::Binary(bytes) => Message::Binary(bytes.into()),
    };
    socket.send(message).await?;
    Ok(())
}

fn gateway_url(
    base_url: &str,
    encoding: GatewayEncoding,
    compression: GatewayCompression,
) -> String {
    let mut url = base_url.trim_end_matches('/').to_owned();
    let version = GATEWAY_VERSION.to_string();

    set_query_param(&mut url, "v", Some(&version));
    set_query_param(&mut url, "encoding", Some(encoding.query_value()));
    set_query_param(&mut url, "compress", compression.query_value());
    url
}

fn set_query_param(url: &mut String, key: &str, value: Option<&str>) {
    let (base, query) = url
        .split_once('?')
        .map_or((url.as_str(), ""), |(base, query)| (base, query));
    let mut parameters: Vec<String> = query
        .split('&')
        .filter(|parameter| {
            !parameter.is_empty()
                && parameter
                    .split_once('=')
                    .map_or(parameter != &key, |(existing_key, _)| existing_key != key)
        })
        .map(ToOwned::to_owned)
        .collect();

    if let Some(value) = value {
        parameters.push(format!("{key}={value}"));
    }

    let rebuilt = if parameters.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{}", parameters.join("&"))
    };
    *url = rebuilt;
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

    use super::{GatewayCompression, GatewayEncoding, gateway_url, reconnect_delay};

    #[test]
    fn gateway_url_adds_protocol_query() {
        assert_eq!(
            gateway_url(
                "wss://gateway.discord.gg",
                GatewayEncoding::Json,
                GatewayCompression::None
            ),
            "wss://gateway.discord.gg?v=10&encoding=json"
        );
    }

    #[test]
    fn gateway_url_adds_etf_and_transport_compression() {
        assert_eq!(
            gateway_url(
                "wss://gateway.discord.gg",
                GatewayEncoding::Etf,
                GatewayCompression::ZstdStream
            ),
            "wss://gateway.discord.gg?v=10&encoding=etf&compress=zstd-stream"
        );
    }

    #[test]
    fn gateway_url_normalizes_protocol_query() {
        assert_eq!(
            gateway_url(
                "wss://gateway.discord.gg?compress=zlib-stream&encoding=json&v=9",
                GatewayEncoding::Etf,
                GatewayCompression::ZstdStream
            ),
            "wss://gateway.discord.gg?v=10&encoding=etf&compress=zstd-stream"
        );
    }

    #[test]
    fn gateway_url_removes_disabled_compression() {
        assert_eq!(
            gateway_url(
                "wss://gateway.discord.gg?v=10&encoding=etf&compress=zlib-stream",
                GatewayEncoding::Etf,
                GatewayCompression::None
            ),
            "wss://gateway.discord.gg?v=10&encoding=etf"
        );
    }

    #[test]
    fn reconnect_backoff_is_capped() {
        assert!(reconnect_delay(100) <= Duration::from_secs(30));
    }
}
