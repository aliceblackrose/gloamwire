use std::collections::VecDeque;

use super::{
    VoiceCloseCode, VoiceEncryptionMode, VoiceGatewayConfig, VoiceGatewayConnection,
    VoiceGatewayEvent, VoiceReconnectStrategy, VoiceResult, VoiceSessionDescription,
    VoiceTransportCrypto, VoiceUdpDiscovery, VoiceUdpSocket,
};

/// Result of applying Discord's Voice Gateway close-code recovery policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoiceRecoveryOutcome {
    /// The existing Voice Gateway session was resumed with its latest `seq_ack`.
    Resumed,
    /// The current voice credentials are no longer reusable. The caller must
    /// repeat the main-Gateway voice rendezvous before creating a new session.
    RestartRequired,
    /// Discord's close code is terminal for the current voice configuration.
    Stopped,
}

/// Fully negotiated Discord voice transport session.
///
/// This owns the Voice Gateway connection, UDP socket, selected RTP transport
/// encryption mode, and the transport AEAD state derived from Session
/// Description. DAVE remains a separate media-encryption layer above this type.
pub struct VoiceSession {
    gateway: VoiceGatewayConnection,
    udp: VoiceUdpSocket,
    discovery: VoiceUdpDiscovery,
    mode: VoiceEncryptionMode,
    description: VoiceSessionDescription,
    transport_crypto: VoiceTransportCrypto,
    pending: VecDeque<VoiceGatewayEvent>,
}

impl std::fmt::Debug for VoiceSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceSession")
            .field("gateway", &self.gateway)
            .field("udp", &self.udp)
            .field("discovery", &self.discovery)
            .field("mode", &self.mode)
            .field("dave_protocol_version", &self.description.dave_protocol_version)
            .field("pending_events", &self.pending.len())
            .finish_non_exhaustive()
    }
}

impl VoiceSession {
    /// Establishes the Voice Gateway, UDP discovery path, Select Protocol, and
    /// Session Description transport cryptography in the required order.
    pub async fn connect(config: VoiceGatewayConfig) -> VoiceResult<Self> {
        let mut gateway = VoiceGatewayConnection::connect(config).await?;
        let mode = gateway.ready().preferred_encryption_mode()?;
        let udp = VoiceUdpSocket::connect(gateway.ready()).await?;
        let discovery = udp.discover().await?;
        gateway.select_protocol(&discovery, &mode).await?;

        let mut pending = VecDeque::new();
        let description = loop {
            match gateway.next_event().await? {
                VoiceGatewayEvent::SessionDescription(description) => break description,
                event => pending.push_back(event),
            }
        };

        if description.mode != mode {
            return Err(super::VoiceError::Protocol(format!(
                "Voice Session Description selected mode {} after client selected {}",
                description.mode.as_ref(),
                mode.as_ref()
            )));
        }

        let transport_crypto = VoiceTransportCrypto::from_session_description(&description)?;

        Ok(Self {
            gateway,
            udp,
            discovery,
            mode,
            description,
            transport_crypto,
            pending,
        })
    }

    /// Returns the live Voice Gateway connection.
    #[must_use]
    pub const fn gateway(&self) -> &VoiceGatewayConnection {
        &self.gateway
    }

    /// Returns mutable access to the live Voice Gateway connection.
    #[must_use]
    pub const fn gateway_mut(&mut self) -> &mut VoiceGatewayConnection {
        &mut self.gateway
    }

    /// Returns the connected UDP transport socket.
    #[must_use]
    pub const fn udp(&self) -> &VoiceUdpSocket {
        &self.udp
    }

    /// Returns the external address discovered through Discord's UDP server.
    #[must_use]
    pub const fn discovery(&self) -> &VoiceUdpDiscovery {
        &self.discovery
    }

    /// Returns the negotiated RTP transport-encryption mode.
    #[must_use]
    pub const fn mode(&self) -> &VoiceEncryptionMode {
        &self.mode
    }

    /// Returns the Voice Gateway Session Description used for this transport.
    #[must_use]
    pub const fn session_description(&self) -> &VoiceSessionDescription {
        &self.description
    }

    /// Returns the RTP transport encryptor/decryptor.
    #[must_use]
    pub const fn transport_crypto(&self) -> &VoiceTransportCrypto {
        &self.transport_crypto
    }

    /// Returns mutable RTP transport cryptography state for outbound nonces.
    #[must_use]
    pub const fn transport_crypto_mut(&mut self) -> &mut VoiceTransportCrypto {
        &mut self.transport_crypto
    }

    /// Returns the next Voice Gateway event, including events buffered while
    /// waiting for Session Description or Resume completion.
    pub async fn next_event(&mut self) -> VoiceResult<VoiceGatewayEvent> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        self.gateway.next_event().await
    }

    /// Applies the reconnect policy for a Voice Gateway close.
    ///
    /// A missing close code is treated as resumable. Restart-required close
    /// codes intentionally do not reuse the current session ID/token; the
    /// caller must repeat the main-Gateway voice rendezvous first.
    pub async fn recover_after_close(
        &mut self,
        code: Option<VoiceCloseCode>,
    ) -> VoiceResult<VoiceRecoveryOutcome> {
        let strategy = code.map_or(VoiceReconnectStrategy::Resume, VoiceCloseCode::reconnect_strategy);
        match strategy {
            VoiceReconnectStrategy::Resume => {
                self.gateway.resume().await?;
                Ok(VoiceRecoveryOutcome::Resumed)
            }
            VoiceReconnectStrategy::Restart => Ok(VoiceRecoveryOutcome::RestartRequired),
            VoiceReconnectStrategy::Stop => Ok(VoiceRecoveryOutcome::Stopped),
        }
    }

    /// Gracefully closes the Voice Gateway. The UDP socket is released when the
    /// session is dropped.
    pub async fn shutdown(&mut self) -> VoiceResult<()> {
        self.gateway.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use tokio::net::{TcpListener, TcpStream, UdpSocket};
    use tokio_tungstenite::{
        WebSocketStream, accept_async,
        tungstenite::{
            Message,
            protocol::{CloseFrame, frame::coding::CloseCode},
        },
    };

    use crate::model::{GuildId, UserId};

    use super::{VoiceRecoveryOutcome, VoiceSession};
    use crate::voice::{
        VoiceCloseCode, VoiceConnectionInfo, VoiceEncryptionMode, VoiceError, VoiceGatewayConfig,
        VoiceGatewayEvent,
    };

    const SSRC: u32 = 0x1020_3040;

    async fn send_json(
        socket: &mut WebSocketStream<TcpStream>,
        opcode: u8,
        data: Value,
        sequence: Option<u16>,
    ) {
        let mut envelope = json!({"op": opcode, "d": data});
        if let Some(sequence) = sequence {
            envelope["seq"] = json!(sequence);
        }
        socket
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("send fixture payload");
    }

    async fn recv_json(socket: &mut WebSocketStream<TcpStream>) -> Value {
        loop {
            match socket
                .next()
                .await
                .expect("fixture websocket message")
                .expect("fixture websocket frame")
            {
                Message::Text(text) => {
                    return serde_json::from_str(text.as_str()).expect("client JSON payload");
                }
                Message::Ping(payload) => socket
                    .send(Message::Pong(payload))
                    .await
                    .expect("fixture pong"),
                Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                Message::Close(frame) => panic!("client closed fixture websocket: {frame:?}"),
            }
        }
    }

    #[tokio::test]
    async fn negotiates_udp_transport_heartbeats_and_resume() {
        let udp_server = UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP fixture");
        let udp_addr = udp_server.local_addr().expect("UDP fixture address");
        let udp_task = tokio::spawn(async move {
            let mut request = [0_u8; 74];
            let (received, peer) = udp_server
                .recv_from(&mut request)
                .await
                .expect("receive discovery request");
            assert_eq!(received, 74);
            assert_eq!(&request[..2], &1_u16.to_be_bytes());
            assert_eq!(&request[2..4], &70_u16.to_be_bytes());
            assert_eq!(&request[4..8], &SSRC.to_be_bytes());

            let mut response = [0_u8; 74];
            response[..2].copy_from_slice(&2_u16.to_be_bytes());
            response[2..4].copy_from_slice(&70_u16.to_be_bytes());
            response[4..8].copy_from_slice(&SSRC.to_be_bytes());
            response[8..21].copy_from_slice(b"203.0.113.42");
            response[72..74].copy_from_slice(&50_000_u16.to_be_bytes());
            udp_server
                .send_to(&response, peer)
                .await
                .expect("send discovery response");
        });

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind websocket fixture");
        let websocket_addr = listener.local_addr().expect("websocket fixture address");
        let websocket_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("initial voice connection");
            let mut socket = accept_async(stream).await.expect("accept initial websocket");
            send_json(&mut socket, 8, json!({"heartbeat_interval": 20}), None).await;

            let identify = recv_json(&mut socket).await;
            assert_eq!(identify["op"], 0);
            assert_eq!(identify["d"]["server_id"], "10");
            assert_eq!(identify["d"]["user_id"], "20");
            assert_eq!(identify["d"]["session_id"], "fixture-session");
            assert_eq!(identify["d"]["token"], "fixture-token");
            assert_eq!(identify["d"]["max_dave_protocol_version"], 0);

            send_json(
                &mut socket,
                2,
                json!({
                    "ssrc": SSRC,
                    "ip": udp_addr.ip().to_string(),
                    "port": udp_addr.port(),
                    "modes": [
                        VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE,
                        VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE
                    ]
                }),
                Some(10),
            )
            .await;

            let select_protocol = recv_json(&mut socket).await;
            assert_eq!(select_protocol["op"], 1);
            assert_eq!(select_protocol["d"]["protocol"], "udp");
            assert_eq!(select_protocol["d"]["data"]["address"], "203.0.113.42");
            assert_eq!(select_protocol["d"]["data"]["port"], 50_000);
            assert_eq!(
                select_protocol["d"]["data"]["mode"],
                VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE
            );

            send_json(
                &mut socket,
                5,
                json!({"speaking": 1, "ssrc": 77, "user_id": "30", "delay": 0}),
                Some(11),
            )
            .await;
            send_json(
                &mut socket,
                4,
                json!({
                    "mode": VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE,
                    "secret_key": [7; 32],
                    "dave_protocol_version": 0
                }),
                Some(12),
            )
            .await;

            let heartbeat = recv_json(&mut socket).await;
            assert_eq!(heartbeat["op"], 3);
            assert_eq!(heartbeat["d"]["seq_ack"], 12);
            send_json(&mut socket, 6, json!({}), Some(13)).await;
            socket
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Library(4015),
                    reason: "fixture crash".into(),
                })))
                .await
                .expect("send fixture close");
            drop(socket);

            let (stream, _) = listener.accept().await.expect("resume voice connection");
            let mut resumed = accept_async(stream).await.expect("accept resumed websocket");
            send_json(&mut resumed, 8, json!({"heartbeat_interval": 20}), None).await;
            let resume = recv_json(&mut resumed).await;
            assert_eq!(resume["op"], 7);
            assert_eq!(resume["d"]["server_id"], "10");
            assert_eq!(resume["d"]["session_id"], "fixture-session");
            assert_eq!(resume["d"]["token"], "fixture-token");
            assert_eq!(resume["d"]["seq_ack"], 13);

            send_json(
                &mut resumed,
                5,
                json!({"speaking": 1, "ssrc": 88, "user_id": "40", "delay": 0}),
                Some(14),
            )
            .await;
            send_json(&mut resumed, 9, json!({}), Some(15)).await;
        });

        let config = VoiceGatewayConfig::new(VoiceConnectionInfo {
            guild_id: GuildId::new(10),
            user_id: UserId::new(20),
            session_id: "fixture-session".to_owned(),
            token: "fixture-token".to_owned(),
            endpoint: format!("ws://{websocket_addr}"),
        });
        let mut session = VoiceSession::connect(config).await.expect("voice session");
        assert_eq!(session.discovery().address, "203.0.113.42");
        assert_eq!(session.discovery().port, 50_000);
        assert_eq!(
            session.mode().as_ref(),
            VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE
        );
        assert_eq!(session.gateway().sequence(), Some(12));

        let VoiceGatewayEvent::Speaking(speaking) = session.next_event().await.expect("buffered speaking") else {
            panic!("expected buffered Speaking event");
        };
        assert_eq!(speaking.ssrc, 77);

        assert_eq!(
            session.next_event().await.expect("heartbeat ACK"),
            VoiceGatewayEvent::HeartbeatAck
        );
        let close = session.next_event().await.expect_err("fixture close");
        let VoiceError::Closed { code, .. } = close else {
            panic!("expected Voice Gateway close error");
        };
        assert_eq!(code, Some(VoiceCloseCode::SERVER_CRASHED));
        assert_eq!(
            session.recover_after_close(code).await.expect("resume recovery"),
            VoiceRecoveryOutcome::Resumed
        );

        let VoiceGatewayEvent::Speaking(speaking) = session.next_event().await.expect("resumed speaking") else {
            panic!("expected buffered resumed Speaking event");
        };
        assert_eq!(speaking.ssrc, 88);
        assert_eq!(
            session.next_event().await.expect("resumed event"),
            VoiceGatewayEvent::Resumed
        );
        assert_eq!(session.gateway().sequence(), Some(15));

        assert_eq!(
            session
                .recover_after_close(Some(VoiceCloseCode::SESSION_INVALID))
                .await
                .expect("restart classification"),
            VoiceRecoveryOutcome::RestartRequired
        );
        assert_eq!(
            session
                .recover_after_close(Some(VoiceCloseCode::DAVE_REQUIRED))
                .await
                .expect("stop classification"),
            VoiceRecoveryOutcome::Stopped
        );

        udp_task.await.expect("UDP fixture task");
        websocket_task.await.expect("websocket fixture task");
    }
}