use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use super::{
    RTP_HEADER_BYTES, VoiceEncryptionMode, VoiceError, VoiceResult, VoiceRtpHeader,
    VoiceSessionDescription,
};

const AEAD_TAG_BYTES: usize = 16;
const NONCE_SUFFIX_BYTES: usize = 4;
const RTP_VERSION: u8 = 2;

/// Decrypted Discord RTP-size transport packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceDecryptedRtp {
    /// Unencrypted RTP header, including CSRC entries and the extension preamble.
    pub header: Vec<u8>,
    /// Decrypted RTP header-extension payload, if the X bit was set.
    pub extension_payload: Vec<u8>,
    /// Decrypted media payload with RTP padding removed.
    ///
    /// For DAVE calls this is still a DAVE-encrypted media frame; DAVE
    /// decryption happens after transport decryption.
    pub media: Vec<u8>,
    /// 32-bit nonce suffix carried by the wire packet.
    pub nonce: u32,
}

/// Discord voice RTP-size transport encryptor/decryptor.
///
/// This layer implements the SFU transport encryption that remains present in
/// DAVE/E2EE calls. It does not implement DAVE media encryption itself.
pub struct VoiceTransportCrypto {
    cipher: VoiceTransportCipher,
    mode: VoiceEncryptionMode,
    next_send_nonce: u64,
}

impl std::fmt::Debug for VoiceTransportCrypto {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoiceTransportCrypto")
            .field("mode", &self.mode)
            .field("next_send_nonce", &self.next_send_nonce)
            .finish_non_exhaustive()
    }
}

enum VoiceTransportCipher {
    Aes256Gcm(Aes256Gcm),
    XChaCha20Poly1305(XChaCha20Poly1305),
}

impl VoiceTransportCrypto {
    /// Creates transport crypto from Voice Gateway Session Description data.
    pub fn from_session_description(description: &VoiceSessionDescription) -> VoiceResult<Self> {
        Self::new(description.mode.clone(), description.secret_key)
    }

    /// Creates transport crypto from a negotiated mode and 32-byte session key.
    pub fn new(mode: VoiceEncryptionMode, secret_key: [u8; 32]) -> VoiceResult<Self> {
        let cipher = match mode.as_ref() {
            VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE => VoiceTransportCipher::Aes256Gcm(
                Aes256Gcm::new_from_slice(&secret_key).map_err(|_| {
                    VoiceError::Crypto("AES-256-GCM rejected the 32-byte session key".to_owned())
                })?,
            ),
            VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE => {
                VoiceTransportCipher::XChaCha20Poly1305(
                    XChaCha20Poly1305::new_from_slice(&secret_key).map_err(|_| {
                        VoiceError::Crypto(
                            "XChaCha20-Poly1305 rejected the 32-byte session key".to_owned(),
                        )
                    })?,
                )
            }
            unsupported => {
                return Err(VoiceError::Protocol(format!(
                    "unsupported Discord voice transport-encryption mode {unsupported}"
                )));
            }
        };

        Ok(Self {
            cipher,
            mode,
            next_send_nonce: 0,
        })
    }

    /// Returns the negotiated Discord transport-encryption mode.
    #[must_use]
    pub const fn mode(&self) -> &VoiceEncryptionMode {
        &self.mode
    }

    /// Returns the next outbound 32-bit nonce value without consuming it.
    #[must_use]
    pub fn next_send_nonce(&self) -> Option<u32> {
        u32::try_from(self.next_send_nonce).ok()
    }

    /// Encrypts a normal Discord audio packet from a fixed 12-byte RTP header.
    pub fn encrypt_audio(
        &mut self,
        header: VoiceRtpHeader,
        media: &[u8],
    ) -> VoiceResult<Vec<u8>> {
        self.encrypt_rtp(&header.encode(), media)
    }

    /// Encrypts a Discord RTP-size packet.
    ///
    /// `header` must contain exactly the unencrypted RTP portion: the base RTP
    /// header, all CSRC entries, and, when X is set, the four-byte extension
    /// preamble. The extension payload itself belongs at the beginning of
    /// `protected_payload` because RTP-size modes encrypt it.
    pub fn encrypt_rtp(
        &mut self,
        header: &[u8],
        protected_payload: &[u8],
    ) -> VoiceResult<Vec<u8>> {
        let layout = rtp_layout(header)?;
        if header.len() != layout.header_len {
            return Err(VoiceError::InvalidRtpPacket(format!(
                "RTP-size encryption expected {} unencrypted header bytes, received {}",
                layout.header_len,
                header.len()
            )));
        }
        if protected_payload.len() < layout.extension_payload_len {
            return Err(VoiceError::InvalidRtpPacket(format!(
                "RTP extension declared {} encrypted bytes but payload has {}",
                layout.extension_payload_len,
                protected_payload.len()
            )));
        }

        let nonce = u32::try_from(self.next_send_nonce).map_err(|_| VoiceError::NonceExhausted)?;
        let nonce_suffix = nonce.to_be_bytes();
        let ciphertext = self.encrypt_with_nonce(&nonce_suffix, header, protected_payload)?;
        self.next_send_nonce += 1;

        let mut packet = Vec::with_capacity(
            header.len() + ciphertext.len() + NONCE_SUFFIX_BYTES,
        );
        packet.extend_from_slice(header);
        packet.extend_from_slice(&ciphertext);
        packet.extend_from_slice(&nonce_suffix);
        Ok(packet)
    }

    /// Authenticates and decrypts one Discord RTP-size packet.
    pub fn decrypt_rtp(&self, packet: &[u8]) -> VoiceResult<VoiceDecryptedRtp> {
        let layout = rtp_layout(packet)?;
        let minimum = layout.header_len + AEAD_TAG_BYTES + NONCE_SUFFIX_BYTES;
        if packet.len() < minimum {
            return Err(VoiceError::InvalidRtpPacket(format!(
                "encrypted RTP packet requires at least {minimum} bytes, received {}",
                packet.len()
            )));
        }

        let suffix_start = packet.len() - NONCE_SUFFIX_BYTES;
        let nonce_suffix: [u8; NONCE_SUFFIX_BYTES] = packet[suffix_start..]
            .try_into()
            .expect("four-byte slice");
        let ciphertext = &packet[layout.header_len..suffix_start];
        let header = &packet[..layout.header_len];
        let plaintext = self.decrypt_with_nonce(&nonce_suffix, header, ciphertext)?;

        if plaintext.len() < layout.extension_payload_len {
            return Err(VoiceError::InvalidRtpPacket(format!(
                "decrypted RTP payload is shorter than its {}-byte extension",
                layout.extension_payload_len
            )));
        }

        let extension_payload = plaintext[..layout.extension_payload_len].to_vec();
        let mut media = plaintext[layout.extension_payload_len..].to_vec();
        if layout.has_padding {
            let padding = media.last().copied().ok_or_else(|| {
                VoiceError::InvalidRtpPacket(
                    "RTP padding bit was set on an empty media payload".to_owned(),
                )
            })? as usize;
            if padding == 0 || padding > media.len() {
                return Err(VoiceError::InvalidRtpPacket(format!(
                    "invalid RTP padding length {padding} for {} decrypted media bytes",
                    media.len()
                )));
            }
            media.truncate(media.len() - padding);
        }

        Ok(VoiceDecryptedRtp {
            header: header.to_vec(),
            extension_payload,
            media,
            nonce: u32::from_be_bytes(nonce_suffix),
        })
    }

    fn encrypt_with_nonce(
        &self,
        nonce_suffix: &[u8; NONCE_SUFFIX_BYTES],
        aad: &[u8],
        plaintext: &[u8],
    ) -> VoiceResult<Vec<u8>> {
        match &self.cipher {
            VoiceTransportCipher::Aes256Gcm(cipher) => {
                let nonce = aes_nonce(nonce_suffix);
                cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: plaintext,
                            aad,
                        },
                    )
                    .map_err(|_| VoiceError::Crypto("AES-256-GCM encryption failed".to_owned()))
            }
            VoiceTransportCipher::XChaCha20Poly1305(cipher) => {
                let nonce = xchacha_nonce(nonce_suffix);
                cipher
                    .encrypt(
                        XNonce::from_slice(&nonce),
                        Payload {
                            msg: plaintext,
                            aad,
                        },
                    )
                    .map_err(|_| {
                        VoiceError::Crypto("XChaCha20-Poly1305 encryption failed".to_owned())
                    })
            }
        }
    }

    fn decrypt_with_nonce(
        &self,
        nonce_suffix: &[u8; NONCE_SUFFIX_BYTES],
        aad: &[u8],
        ciphertext: &[u8],
    ) -> VoiceResult<Vec<u8>> {
        match &self.cipher {
            VoiceTransportCipher::Aes256Gcm(cipher) => {
                let nonce = aes_nonce(nonce_suffix);
                cipher
                    .decrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: ciphertext,
                            aad,
                        },
                    )
                    .map_err(|_| {
                        VoiceError::Crypto(
                            "AES-256-GCM authentication/decryption failed".to_owned(),
                        )
                    })
            }
            VoiceTransportCipher::XChaCha20Poly1305(cipher) => {
                let nonce = xchacha_nonce(nonce_suffix);
                cipher
                    .decrypt(
                        XNonce::from_slice(&nonce),
                        Payload {
                            msg: ciphertext,
                            aad,
                        },
                    )
                    .map_err(|_| {
                        VoiceError::Crypto(
                            "XChaCha20-Poly1305 authentication/decryption failed".to_owned(),
                        )
                    })
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RtpLayout {
    header_len: usize,
    extension_payload_len: usize,
    has_padding: bool,
}

fn rtp_layout(packet: &[u8]) -> VoiceResult<RtpLayout> {
    if packet.len() < RTP_HEADER_BYTES {
        return Err(VoiceError::InvalidRtpPacket(format!(
            "RTP header requires {RTP_HEADER_BYTES} bytes, received {}",
            packet.len()
        )));
    }

    let version = packet[0] >> 6;
    if version != RTP_VERSION {
        return Err(VoiceError::InvalidRtpPacket(format!(
            "expected RTP version {RTP_VERSION}, received {version}"
        )));
    }

    let csrc_count = usize::from(packet[0] & 0x0f);
    let base_header_len = RTP_HEADER_BYTES + csrc_count * 4;
    if packet.len() < base_header_len {
        return Err(VoiceError::InvalidRtpPacket(format!(
            "RTP header declares {csrc_count} CSRC entries but packet is only {} bytes",
            packet.len()
        )));
    }

    let has_extension = packet[0] & 0x10 != 0;
    let has_padding = packet[0] & 0x20 != 0;
    if !has_extension {
        return Ok(RtpLayout {
            header_len: base_header_len,
            extension_payload_len: 0,
            has_padding,
        });
    }

    let header_len = base_header_len + 4;
    if packet.len() < header_len {
        return Err(VoiceError::InvalidRtpPacket(
            "RTP extension bit was set without a four-byte extension preamble".to_owned(),
        ));
    }
    let words = u16::from_be_bytes([packet[base_header_len + 2], packet[base_header_len + 3]]);
    let extension_payload_len = usize::from(words) * 4;

    Ok(RtpLayout {
        header_len,
        extension_payload_len,
        has_padding,
    })
}

fn aes_nonce(suffix: &[u8; NONCE_SUFFIX_BYTES]) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    nonce[..NONCE_SUFFIX_BYTES].copy_from_slice(suffix);
    nonce
}

fn xchacha_nonce(suffix: &[u8; NONCE_SUFFIX_BYTES]) -> [u8; 24] {
    let mut nonce = [0_u8; 24];
    nonce[..NONCE_SUFFIX_BYTES].copy_from_slice(suffix);
    nonce
}

#[cfg(test)]
mod tests {
    use super::VoiceTransportCrypto;
    use crate::voice::{VoiceEncryptionMode, VoiceRtpHeader};

    const KEY: [u8; 32] = [0x42; 32];

    #[test]
    fn aes_gcm_round_trips_audio_and_appends_big_endian_nonce() {
        let mut crypto = VoiceTransportCrypto::new(
            VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE),
            KEY,
        )
        .expect("crypto");
        let header = VoiceRtpHeader {
            sequence: 7,
            timestamp: 960,
            ssrc: 42,
        };

        let first = crypto.encrypt_audio(header, b"opus-one").expect("encrypt");
        let second = crypto.encrypt_audio(header, b"opus-two").expect("encrypt");
        assert_eq!(&first[first.len() - 4..], &[0, 0, 0, 0]);
        assert_eq!(&second[second.len() - 4..], &[0, 0, 0, 1]);

        let decrypted = crypto.decrypt_rtp(&first).expect("decrypt");
        assert_eq!(decrypted.media, b"opus-one");
        assert_eq!(decrypted.nonce, 0);
    }

    #[test]
    fn xchacha_round_trips_audio() {
        let mut crypto = VoiceTransportCrypto::new(
            VoiceEncryptionMode::from(
                VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE,
            ),
            KEY,
        )
        .expect("crypto");
        let header = VoiceRtpHeader {
            sequence: u16::MAX,
            timestamp: u32::MAX,
            ssrc: 123,
        };

        let packet = crypto.encrypt_audio(header, b"dave-or-opus").expect("encrypt");
        let decrypted = crypto.decrypt_rtp(&packet).expect("decrypt");
        assert_eq!(decrypted.header, header.encode());
        assert_eq!(decrypted.media, b"dave-or-opus");
    }

    #[test]
    fn rtpsize_keeps_extension_preamble_as_aad_and_encrypts_extension_payload() {
        let mut crypto = VoiceTransportCrypto::new(
            VoiceEncryptionMode::from(
                VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE,
            ),
            KEY,
        )
        .expect("crypto");
        let mut header = VoiceRtpHeader {
            sequence: 1,
            timestamp: 2,
            ssrc: 3,
        }
        .encode()
        .to_vec();
        header[0] |= 0x10;
        header.extend_from_slice(&[0xBE, 0xDE, 0x00, 0x01]);
        let extension = [0x10, 0x20, 0x30, 0x40];
        let mut protected = extension.to_vec();
        protected.extend_from_slice(b"opus");

        let packet = crypto.encrypt_rtp(&header, &protected).expect("encrypt");
        assert_eq!(&packet[..header.len()], header.as_slice());
        let decrypted = crypto.decrypt_rtp(&packet).expect("decrypt");
        assert_eq!(decrypted.extension_payload, extension);
        assert_eq!(decrypted.media, b"opus");
    }

    #[test]
    fn decrypt_removes_authenticated_rtp_padding() {
        let mut crypto = VoiceTransportCrypto::new(
            VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE),
            KEY,
        )
        .expect("crypto");
        let mut header = VoiceRtpHeader {
            sequence: 1,
            timestamp: 2,
            ssrc: 3,
        }
        .encode();
        header[0] |= 0x20;
        let protected = [b'a', b'b', b'c', 0, 0, 3];

        let packet = crypto.encrypt_rtp(&header, &protected).expect("encrypt");
        let decrypted = crypto.decrypt_rtp(&packet).expect("decrypt");
        assert_eq!(decrypted.media, b"abc");
    }

    #[test]
    fn tampered_header_fails_authentication() {
        let mut crypto = VoiceTransportCrypto::new(
            VoiceEncryptionMode::from(VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE),
            KEY,
        )
        .expect("crypto");
        let header = VoiceRtpHeader {
            sequence: 1,
            timestamp: 2,
            ssrc: 3,
        };
        let mut packet = crypto.encrypt_audio(header, b"opus").expect("encrypt");
        packet[3] ^= 1;

        assert!(crypto.decrypt_rtp(&packet).is_err());
    }
}
