use std::collections::HashMap;

#[cfg(feature = "dave-davey")]
use crate::model::ChannelId;
use crate::model::UserId;

use super::{
    DaveGatewayCommand, DaveParticipantSet, DaveProtocolEvent, DaveProviderError,
    DaveProviderLifecycle, OPUS_SILENCE_FLUSH_FRAMES, OPUS_SILENCE_FRAME, RTP_HEADER_BYTES,
    VoiceError, VoiceFramePacer, VoiceGatewayConfig, VoiceGatewayEvent, VoiceOpusFrame,
    VoiceRecoveryOutcome, VoiceResult, VoiceRtpHeader, VoiceRtpSequencer, VoiceSession,
    VoiceSpeakingFlags,
};

/// One decoded Opus frame received through Discord's transport and optional
/// DAVE media-encryption layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceReceivedOpus {
    /// RTP header carried by the Discord UDP packet.
    pub header: VoiceRtpHeader,
    /// Discord user mapped to the RTP SSRC through Voice Speaking events.
    pub sender: Option<UserId>,
    /// Decrypted encoded Opus frame.
    pub payload: Vec<u8>,
    /// Decrypted RTP header-extension payload, when present.
    pub extension_payload: Vec<u8>,
    /// RTP-size transport nonce suffix carried by the packet.
    pub transport_nonce: u32,
}

/// Fully managed Discord voice session with a pluggable DAVE/MLS provider.
///
/// This composes the media pipeline in Discord's required order:
/// encoded Opus -> DAVE -> RTP -> RTP-size transport AEAD -> UDP. Receive-side
/// processing reverses that order and uses Voice Speaking events to associate
/// incoming SSRCs with Discord users for DAVE sender ratchets.
pub struct DaveVoiceSession<P> {
    session: VoiceSession,
    provider: P,
    participants: DaveParticipantSet,
    ssrc_users: HashMap<u32, UserId>,
    sequencer: VoiceRtpSequencer,
}

impl<P> std::fmt::Debug for DaveVoiceSession<P>
where
    P: DaveProviderLifecycle + std::fmt::Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaveVoiceSession")
            .field("session", &self.session)
            .field("provider", &self.provider)
            .field("participants", &self.participants.len())
            .field("known_ssrcs", &self.ssrc_users.len())
            .finish_non_exhaustive()
    }
}

impl<P> DaveVoiceSession<P>
where
    P: DaveProviderLifecycle,
{
    /// Connects voice while automatically advertising the provider's highest
    /// supported DAVE protocol version and initializing it from Session
    /// Description.
    pub async fn connect(config: VoiceGatewayConfig, mut provider: P) -> VoiceResult<Self> {
        let config = config.with_max_dave_protocol_version(provider.max_protocol_version());
        let mut session = VoiceSession::connect(config).await?;
        let commands = provider
            .configure_protocol_version(session.session_description().dave_protocol_version)
            .map_err(provider_error)?;
        send_commands(&mut session, commands).await?;

        let sequencer = VoiceRtpSequencer::new(session.gateway().ready().ssrc);
        Ok(Self {
            session,
            provider,
            participants: DaveParticipantSet::default(),
            ssrc_users: HashMap::new(),
            sequencer,
        })
    }

    /// Returns the negotiated low-level Voice Gateway/UDP transport session.
    #[must_use]
    pub const fn session(&self) -> &VoiceSession {
        &self.session
    }

    /// Returns mutable access to the low-level voice session.
    #[must_use]
    pub const fn session_mut(&mut self) -> &mut VoiceSession {
        &mut self.session
    }

    /// Returns the active DAVE provider.
    #[must_use]
    pub const fn provider(&self) -> &P {
        &self.provider
    }

    /// Returns mutable access to the active DAVE provider.
    #[must_use]
    pub const fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// Returns Discord users currently expected in the MLS group.
    #[must_use]
    pub const fn participants(&self) -> &DaveParticipantSet {
        &self.participants
    }

    /// Sends Voice Gateway Speaking state.
    pub async fn set_speaking(&mut self, flags: VoiceSpeakingFlags) -> VoiceResult<()> {
        self.session.gateway_mut().set_speaking(flags).await
    }

    /// Flushes Discord's canonical five 20 ms Opus silence frames and then
    /// clears the Voice Gateway Speaking state.
    ///
    /// Silence frames intentionally bypass DAVE frame encryption because Discord
    /// permits the canonical `0xF8FFFE` frame to pass through during E2EE calls.
    pub async fn finish_speaking(&mut self) -> VoiceResult<()> {
        let mut pacer = VoiceFramePacer::default();
        for _ in 0..OPUS_SILENCE_FLUSH_FRAMES {
            pacer.wait_for_next_frame().await;
            self.send_opus_frame(VoiceOpusFrame::silence()).await?;
        }
        self.set_speaking(VoiceSpeakingFlags(0)).await
    }

    /// Returns and applies the next Voice Gateway event.
    ///
    /// DAVE events are passed to the provider and any resulting client commands
    /// are sent before this method returns the original event to the caller.
    pub async fn next_event(&mut self) -> VoiceResult<VoiceGatewayEvent> {
        let event = self.session.next_event().await?;
        self.apply_gateway_event(&event).await?;
        Ok(event)
    }

    /// Applies a Voice Gateway event to participant, SSRC, and DAVE state.
    pub async fn apply_gateway_event(&mut self, event: &VoiceGatewayEvent) -> VoiceResult<()> {
        if let VoiceGatewayEvent::Speaking(speaking) = event
            && let Some(user_id) = speaking.user_id
        {
            self.ssrc_users.insert(speaking.ssrc, user_id);
        }

        let Some(dave_event) = DaveProtocolEvent::from_gateway_event(event)? else {
            return Ok(());
        };

        if let DaveProtocolEvent::ClientDisconnect { user_id } = &dave_event {
            self.ssrc_users.retain(|_, mapped| mapped != user_id);
        }

        self.participants.apply(&dave_event);
        let commands = match self
            .provider
            .handle_gateway_event(&dave_event, &self.participants)
        {
            Ok(commands) => commands,
            Err(error) => {
                if let Some(transition_id) = recovery_transition_id(&dave_event) {
                    let mut commands =
                        vec![DaveGatewayCommand::InvalidCommitWelcome { transition_id }];
                    let version = self.provider.active_protocol_version();
                    commands.extend(
                        self.provider
                            .configure_protocol_version(version)
                            .map_err(provider_error)?,
                    );
                    send_commands(&mut self.session, commands).await?;
                    return Ok(());
                }
                return Err(provider_error(error));
            }
        };
        send_commands(&mut self.session, commands).await
    }

    /// Sends one complete encoded Opus frame through DAVE, RTP, transport AEAD,
    /// and the connected UDP socket.
    pub async fn send_opus_frame(&mut self, frame: VoiceOpusFrame<'_>) -> VoiceResult<usize> {
        let media = if frame.payload() == OPUS_SILENCE_FRAME {
            frame.payload().to_vec()
        } else {
            self.provider
                .encrypt_opus(frame.payload())
                .map_err(provider_error)?
        };
        let header = self
            .sequencer
            .next_header_with_timestamp_step(frame.duration().timestamp_step());
        let packet = self
            .session
            .transport_crypto_mut()
            .encrypt_audio(header, &media)?;
        self.session.udp().send(&packet).await
    }

    /// Decrypts one already-received Discord UDP packet into an encoded Opus
    /// frame, including DAVE sender-ratchet processing when required.
    pub fn decrypt_rtp_packet(&mut self, packet: &[u8]) -> VoiceResult<VoiceReceivedOpus> {
        let decrypted = self.session.transport_crypto().decrypt_rtp(packet)?;
        let header = decode_audio_header(&decrypted.header)?;
        let sender = self.ssrc_users.get(&header.ssrc).copied();

        let payload = if decrypted.media == OPUS_SILENCE_FRAME {
            decrypted.media
        } else if let Some(sender) = sender {
            self.provider
                .decrypt_opus(sender, &decrypted.media)
                .map_err(provider_error)?
        } else if self.provider.active_protocol_version() == 0 {
            decrypted.media
        } else {
            return Err(VoiceError::Protocol(format!(
                "received DAVE media for unknown RTP SSRC {} before a Speaking mapping",
                header.ssrc
            )));
        };

        Ok(VoiceReceivedOpus {
            header,
            sender,
            payload,
            extension_payload: decrypted.extension_payload,
            transport_nonce: decrypted.nonce,
        })
    }

    /// Receives one UDP packet and returns its fully decrypted encoded Opus
    /// frame. The supplied buffer must be large enough for the encrypted RTP
    /// packet; 2048 bytes is sufficient for normal Discord voice packets.
    pub async fn recv_opus(&mut self, buffer: &mut [u8]) -> VoiceResult<VoiceReceivedOpus> {
        let received = self.session.udp().recv(buffer).await?;
        self.decrypt_rtp_packet(&buffer[..received])
    }

    /// Applies Voice Gateway close-code recovery to the managed session.
    pub async fn recover_after_close(
        &mut self,
        code: Option<super::VoiceCloseCode>,
    ) -> VoiceResult<VoiceRecoveryOutcome> {
        self.session.recover_after_close(code).await
    }

    /// Gracefully closes the Voice Gateway transport.
    pub async fn shutdown(&mut self) -> VoiceResult<()> {
        self.session.shutdown().await
    }
}

#[cfg(feature = "dave-davey")]
impl DaveVoiceSession<super::DaveyProvider> {
    /// Connects a managed voice session using Gloamwire's optional pure-Rust
    /// `davey` backend. `channel_id` is the voice channel snowflake and becomes
    /// the MLS group ID.
    pub async fn connect_davey(
        info: super::VoiceConnectionInfo,
        channel_id: ChannelId,
    ) -> VoiceResult<Self> {
        let provider = super::DaveyProvider::new(info.user_id, channel_id);
        Self::connect(VoiceGatewayConfig::new(info), provider).await
    }
}

fn provider_error(error: DaveProviderError) -> VoiceError {
    VoiceError::Protocol(format!("DAVE provider error: {error}"))
}

fn recovery_transition_id(event: &DaveProtocolEvent) -> Option<u16> {
    match event {
        DaveProtocolEvent::AnnounceCommit { transition_id, .. }
        | DaveProtocolEvent::Welcome { transition_id, .. } => Some(*transition_id),
        _ => None,
    }
}

async fn send_commands(
    session: &mut VoiceSession,
    commands: Vec<DaveGatewayCommand>,
) -> VoiceResult<()> {
    for command in commands {
        command.send(session.gateway_mut()).await?;
    }
    Ok(())
}

fn decode_audio_header(bytes: &[u8]) -> VoiceResult<VoiceRtpHeader> {
    if bytes.len() < RTP_HEADER_BYTES || bytes[0] >> 6 != 2 {
        return Err(VoiceError::InvalidRtpPacket(
            "decrypted packet did not contain a valid RTP v2 audio header".to_owned(),
        ));
    }
    Ok(VoiceRtpHeader {
        sequence: u16::from_be_bytes([bytes[2], bytes[3]]),
        timestamp: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        ssrc: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
    })
}

#[cfg(test)]
mod tests {
    use super::decode_audio_header;
    use crate::voice::VoiceRtpHeader;

    #[test]
    fn receive_header_accepts_rtp_extension_bit() {
        let header = VoiceRtpHeader {
            sequence: 7,
            timestamp: 8,
            ssrc: 9,
        };
        let mut bytes = header.encode();
        bytes[0] |= 0x10;
        assert_eq!(decode_audio_header(&bytes).expect("header"), header);
    }
}
