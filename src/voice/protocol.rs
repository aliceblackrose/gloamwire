use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::UserId;

use super::{VoiceError, VoiceResult};

/// Discord Voice Gateway protocol version implemented by Gloamwire.
pub const VOICE_GATEWAY_VERSION: u8 = 8;

/// Voice transport-encryption mode negotiated through Select Protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VoiceEncryptionMode(pub String);

impl VoiceEncryptionMode {
    pub const AEAD_AES256_GCM_RTPSIZE: &'static str = "aead_aes256_gcm_rtpsize";
    pub const AEAD_XCHACHA20_POLY1305_RTPSIZE: &'static str =
        "aead_xchacha20_poly1305_rtpsize";

    /// Selects Discord's preferred supported transport-encryption mode.
    ///
    /// AES-256-GCM is preferred when Discord advertises it. XChaCha20-Poly1305
    /// is required by Discord and is used as the portable fallback.
    pub fn preferred(modes: &[Self]) -> VoiceResult<Self> {
        if modes
            .iter()
            .any(|mode| mode.0 == Self::AEAD_AES256_GCM_RTPSIZE)
        {
            return Ok(Self(Self::AEAD_AES256_GCM_RTPSIZE.to_owned()));
        }
        if modes
            .iter()
            .any(|mode| mode.0 == Self::AEAD_XCHACHA20_POLY1305_RTPSIZE)
        {
            return Ok(Self(Self::AEAD_XCHACHA20_POLY1305_RTPSIZE.to_owned()));
        }

        Err(VoiceError::UnsupportedEncryptionMode)
    }
}

impl From<String> for VoiceEncryptionMode {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for VoiceEncryptionMode {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for VoiceEncryptionMode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Voice Gateway opcode 2 Ready data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VoiceReady {
    pub ssrc: u32,
    pub ip: String,
    pub port: u16,
    #[serde(default)]
    pub modes: Vec<VoiceEncryptionMode>,
}

impl VoiceReady {
    /// Chooses the best transport-encryption mode advertised by Discord.
    pub fn preferred_encryption_mode(&self) -> VoiceResult<VoiceEncryptionMode> {
        VoiceEncryptionMode::preferred(&self.modes)
    }
}

/// Voice Gateway opcode 4 Session Description data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VoiceSessionDescription {
    pub mode: VoiceEncryptionMode,
    pub secret_key: [u8; 32],
    #[serde(default)]
    pub dave_protocol_version: u16,
}

/// Voice Gateway speaking bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceSpeakingFlags(pub u32);

impl VoiceSpeakingFlags {
    pub const MICROPHONE: Self = Self(1 << 0);
    pub const SOUNDSHARE: Self = Self(1 << 1);
    pub const PRIORITY: Self = Self(1 << 2);

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Voice Gateway opcode 5 data received from Discord.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VoiceSpeakingEvent {
    pub speaking: u32,
    pub ssrc: u32,
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(default)]
    pub delay: u32,
}

/// DAVE-related Voice Gateway event exposed to an external DAVE implementation.
#[derive(Debug, Clone, PartialEq)]
pub enum DaveGatewayEvent {
    /// JSON Voice Gateway DAVE opcode.
    Json {
        opcode: u8,
        sequence: Option<u16>,
        data: Value,
    },
    /// Binary Voice Gateway DAVE opcode. Server-sent binary messages include a
    /// big-endian sequence number before the opcode.
    Binary {
        opcode: u8,
        sequence: u16,
        payload: Vec<u8>,
    },
}

/// High-level Voice Gateway event after heartbeat/session lifecycle handling.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceGatewayEvent {
    SessionDescription(VoiceSessionDescription),
    Speaking(VoiceSpeakingEvent),
    Resumed,
    HeartbeatAck,
    ClientsConnect(Value),
    ClientDisconnect(Value),
    Dave(DaveGatewayEvent),
    Unknown {
        opcode: u8,
        sequence: Option<u16>,
        data: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::{VoiceEncryptionMode, VoiceError};

    #[test]
    fn prefers_aes_when_available() {
        let modes = vec![
            VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE),
            VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE),
        ];

        assert_eq!(
            VoiceEncryptionMode::preferred(&modes).expect("mode").as_ref(),
            VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE
        );
    }

    #[test]
    fn rejects_only_deprecated_or_unknown_modes() {
        let modes = vec![VoiceEncryptionMode::from("xsalsa20_poly1305")];
        assert!(matches!(
            VoiceEncryptionMode::preferred(&modes),
            Err(VoiceError::UnsupportedEncryptionMode)
        ));
    }
}
