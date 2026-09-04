#![cfg(feature = "dave-davey")]

use gloamwire::{
    model::{ChannelId, UserId},
    voice::{
        DAVE_KEY_PACKAGE_OPCODE, DaveGatewayCommand, DaveParticipantSet, DaveProposalOperation,
        DaveProtocolEvent, DaveProvider, DaveProviderLifecycle, DaveyProvider,
    },
};

const CHANNEL_ID: u64 = 927_310_423_890_473_011;
const LOCAL_USER_ID: u64 = 158_049_329_150_427_136;
const OTHER_USER_ID: u64 = 158_533_742_254_751_744;

// Fixture published by davey for Discord DAVE protocol v1 external-sender tests.
const EXTERNAL_SENDER: &[u8] = &[
    0x40, 0x41, 0x04, 0xca, 0x1a, 0x2b, 0x10, 0x25, 0x01, 0xd0, 0x67, 0x2b, 0xd4, 0x5e, 0xd7, 0x4f,
    0xfb, 0x83, 0xe0, 0x78, 0xb2, 0xba, 0x5b, 0x12, 0xc3, 0xf6, 0x9f, 0xad, 0x56, 0xf0, 0x83, 0xb6,
    0xa3, 0x5f, 0xc9, 0x89, 0xc6, 0x73, 0x6b, 0x58, 0x52, 0xb5, 0xae, 0xcd, 0xfc, 0xdf, 0x20, 0x6e,
    0x15, 0x6d, 0x3d, 0x1d, 0xba, 0x8e, 0x3e, 0x5b, 0x2f, 0x89, 0xfc, 0x0c, 0x16, 0xf1, 0x16, 0x14,
    0xe8, 0x4e, 0x4a, 0x00, 0x01, 0x01, 0x00,
];

fn provider() -> DaveyProvider {
    DaveyProvider::new(UserId::new(LOCAL_USER_ID), ChannelId::new(CHANNEL_ID))
}

fn key_package(commands: &[DaveGatewayCommand]) -> &[u8] {
    match commands {
        [DaveGatewayCommand::KeyPackage(payload)] => payload,
        _ => panic!("expected exactly one DAVE key package"),
    }
}

#[test]
fn group_creation_accepts_external_sender_fixture() {
    let mut provider = provider();
    let commands = provider
        .configure_protocol_version(1)
        .expect("configure DAVE v1");
    assert_eq!(commands[0].opcode(), DAVE_KEY_PACKAGE_OPCODE);
    assert!(!key_package(&commands).is_empty());

    let commands = provider
        .handle_gateway_event(
            &DaveProtocolEvent::ExternalSender {
                package: EXTERNAL_SENDER.to_vec(),
            },
            &DaveParticipantSet::default(),
        )
        .expect("accept external sender fixture");
    assert!(commands.is_empty());
}

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
fn epoch_prepare_initializes_only_the_first_epoch() {
    let mut provider = provider();
    let participants = DaveParticipantSet::default();

    let initial = provider
        .handle_gateway_event(
            &DaveProtocolEvent::PrepareEpoch {
                protocol_version: 1,
                epoch: 1,
            },
            &participants,
        )
        .expect("prepare first epoch");
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].opcode(), DAVE_KEY_PACKAGE_OPCODE);
    assert_eq!(provider.active_protocol_version(), 1);

    let later = provider
        .handle_gateway_event(
            &DaveProtocolEvent::PrepareEpoch {
                protocol_version: 1,
                epoch: 2,
            },
            &participants,
        )
        .expect("prepare later epoch");
    assert!(later.is_empty());
}

#[test]
fn reconfiguration_rotates_the_key_package_for_recovery() {
    let mut provider = provider();
    let first = provider
        .configure_protocol_version(1)
        .expect("initial DAVE configuration");
    let first = key_package(&first).to_vec();

    let recovered = provider
        .configure_protocol_version(1)
        .expect("DAVE recovery reconfiguration");
    let recovered = key_package(&recovered);

    assert_ne!(first, recovered);
    assert_eq!(provider.active_protocol_version(), 1);
}

#[test]
fn protocol_transitions_execute_only_after_prepare() {
    let mut provider = provider();
    provider
        .configure_protocol_version(1)
        .expect("configure DAVE v1");
    let participants = DaveParticipantSet::default();

    let ready = provider
        .handle_gateway_event(
            &DaveProtocolEvent::PrepareTransition {
                protocol_version: 0,
                transition_id: 7,
            },
            &participants,
        )
        .expect("prepare downgrade");
    assert_eq!(
        ready,
        vec![DaveGatewayCommand::ReadyForTransition { transition_id: 7 }]
    );
    assert_eq!(provider.active_protocol_version(), 1);

    provider
        .handle_gateway_event(
            &DaveProtocolEvent::ExecuteTransition { transition_id: 7 },
            &participants,
        )
        .expect("execute downgrade");
    assert_eq!(provider.active_protocol_version(), 0);
}

#[test]
fn invalid_protocol_messages_fail_closed() {
    let mut provider = provider();
    let participants = DaveParticipantSet::default();

    assert!(provider.configure_protocol_version(2).is_err());
    assert!(
        provider
            .handle_gateway_event(
                &DaveProtocolEvent::ExecuteTransition { transition_id: 99 },
                &participants,
            )
            .is_err()
    );

    provider
        .configure_protocol_version(1)
        .expect("configure DAVE v1");
    assert!(
        provider
            .handle_gateway_event(
                &DaveProtocolEvent::Proposals {
                    operation: DaveProposalOperation::Unknown(9),
                    payload: vec![1, 2, 3],
                },
                &participants,
            )
            .is_err()
    );
}
