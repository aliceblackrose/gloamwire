//! Discord Voice Gateway v8 and UDP/RTP transport primitives.
//!
//! Voice connections are separate from the main Discord Gateway. A guild voice
//! join first collects `VOICE_STATE_UPDATE` and `VOICE_SERVER_UPDATE`, then
//! identifies to the Voice Gateway and establishes a UDP path. Gloamwire
//! implements Discord's current RTP-size transport AEAD modes and accepts
//! already-encoded Opus frames at a codec-agnostic boundary; DAVE/E2EE media
//! cryptography remains a separate protocol layer.

mod close;
mod crypto;
mod error;
mod gateway;
mod opus;
mod protocol;
mod rendezvous;
mod rtp;
mod session;
mod udp;

pub use close::{VoiceCloseCode, VoiceReconnectStrategy};
pub use crypto::{VoiceDecryptedRtp, VoiceTransportCrypto};
pub use error::{VoiceError, VoiceResult};
pub use gateway::{VoiceGatewayConfig, VoiceGatewayConnection};
pub use opus::{
    DISCORD_OPUS_CHANNELS, DISCORD_OPUS_SAMPLE_RATE, OPUS_SILENCE_FLUSH_FRAMES, VoiceFramePacer,
    VoiceOpusFrame, VoiceOpusFrameDuration,
};
pub use protocol::{
    DaveGatewayEvent, VOICE_GATEWAY_VERSION, VoiceEncryptionMode, VoiceGatewayEvent, VoiceReady,
    VoiceSessionDescription, VoiceSpeakingEvent, VoiceSpeakingFlags,
};
pub use rendezvous::{
    VoiceConnectionInfo, VoiceRendezvous, VoiceRendezvousStatus, VoiceServerUpdate,
};
pub use rtp::{
    DISCORD_OPUS_PAYLOAD_TYPE, OPUS_20MS_TIMESTAMP_STEP, OPUS_SILENCE_FRAME, RTP_HEADER_BYTES,
    VoiceRtpHeader, VoiceRtpSequencer,
};
pub use session::{VoiceRecoveryOutcome, VoiceSession};
pub use udp::{VoiceUdpDiscovery, VoiceUdpSocket};
