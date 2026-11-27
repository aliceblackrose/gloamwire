#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = gloamwire::voice::VoiceRtpHeader::decode(data);
    let mut crypto = gloamwire::voice::VoiceTransportCrypto::new_with_fresh_key(
        gloamwire::voice::VoiceEncryptionMode::from(
            gloamwire::voice::VoiceEncryptionMode::AEAD_AES256_GCM_RTPSIZE,
        ),
        [0; 32],
    )
    .unwrap();
    let split = data.first().map_or(0, |value| usize::from(*value)).min(data.len());
    let _ = crypto.encrypt_rtp(&data[..split], &data[split..]);
});
