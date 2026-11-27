#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use gloamwire::voice::{DaveGatewayEvent, DaveParticipantSet, DaveProtocolEvent, VoiceGatewayEvent};

    let Some((&opcode, payload)) = data.split_first() else {
        return;
    };
    let mut participants = DaveParticipantSet::default();
    let binary = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
        opcode,
        sequence: 0,
        payload: payload.to_vec(),
    });
    if let Ok(Some(event)) = DaveProtocolEvent::from_gateway_event(&binary) {
        let _ = participants.try_apply(&event, 128);
    }
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
        for input in [
            VoiceGatewayEvent::Dave(DaveGatewayEvent::Json {
                opcode,
                sequence: None,
                data: value.clone(),
            }),
            VoiceGatewayEvent::ClientsConnect(value.clone()),
            VoiceGatewayEvent::ClientDisconnect(value),
        ] {
            if let Ok(Some(event)) = DaveProtocolEvent::from_gateway_event(&input) {
                let _ = participants.try_apply(&event, 128);
                assert!(participants.len() <= 128);
            }
        }
    }
});
