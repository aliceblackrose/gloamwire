#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use gloamwire::voice::{
        VoiceEncryptionMode, VoiceRtpHeader, VoiceTransportCrypto, VoiceTransportDecryptor,
    };

    for name in [
        VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE,
        VoiceEncryptionMode::AEAD_XCHACHA20_POLY1305_RTPSIZE,
    ] {
        // Fixed synthetic test key; never used with live voice traffic.
        let mode = VoiceEncryptionMode::from(name);
        let receiver = VoiceTransportDecryptor::new(mode.clone(), [0; 32]).unwrap();
        let _ = receiver.decrypt_rtp(data);
        if data.len() <= 8192 {
            let mut sender = VoiceTransportCrypto::new_with_fresh_key(mode, [0; 32]).unwrap();
            let header = VoiceRtpHeader {
                sequence: 1,
                timestamp: 2,
                ssrc: 3,
            };
            let packet = sender.encrypt_audio(header, data).unwrap();
            let decoded = receiver.decrypt_rtp(&packet).unwrap();
            assert_eq!(decoded.media, data);
            let mut tampered = packet;
            tampered[3] ^= 1;
            assert!(receiver.decrypt_rtp(&tampered).is_err());
        }
    }
});
