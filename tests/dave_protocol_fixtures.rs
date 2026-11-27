#![cfg(feature = "voice")]

use gloamwire::{
    model::UserId,
    voice::{DaveParticipantSet, DaveProtocolEvent},
};

const LOCAL_USER_ID: u64 = 158_049_329_150_427_136;
const OTHER_USER_ID: u64 = 158_533_742_254_751_744;

#[test]
fn member_join_and_remove_update_expected_participants() {
    let local = UserId::new(LOCAL_USER_ID);
    let other = UserId::new(OTHER_USER_ID);
    let mut participants = DaveParticipantSet::default();

    participants.apply(&DaveProtocolEvent::ClientsConnect {
        user_ids: vec![local, other],
    });
    assert_eq!(participants.len(), 2);
    assert!(participants.contains(local));
    assert!(participants.contains(other));

    participants.apply(&DaveProtocolEvent::ClientDisconnect { user_id: other });
    assert_eq!(participants.len(), 1);
    assert!(participants.contains(local));
    assert!(!participants.contains(other));
}

#[test]
fn participant_limit_is_atomic_and_duplicate_safe() {
    let mut participants = DaveParticipantSet::default();
    let a = UserId::new(1);
    let b = UserId::new(2);
    participants
        .try_apply(
            &DaveProtocolEvent::ClientsConnect {
                user_ids: vec![a, a],
            },
            1,
        )
        .unwrap();
    assert!(
        participants
            .try_apply(
                &DaveProtocolEvent::ClientsConnect {
                    user_ids: vec![a, b]
                },
                1
            )
            .is_err()
    );
    assert_eq!(participants.len(), 1);
    assert!(participants.contains(a));
}

#[test]
fn wrong_dave_wire_framing_is_rejected() {
    use gloamwire::voice::{DaveGatewayEvent, VoiceGatewayEvent};
    let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Json {
        opcode: 25,
        sequence: Some(1),
        data: serde_json::json!({}),
    });
    assert!(DaveProtocolEvent::from_gateway_event(&event).is_err());
    let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
        opcode: 21,
        sequence: 2,
        payload: vec![],
    });
    assert!(DaveProtocolEvent::from_gateway_event(&event).is_err());
}
