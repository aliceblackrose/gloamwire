use std::{collections::HashSet, error::Error, fmt};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::model::UserId;

use super::{DaveGatewayEvent, VoiceError, VoiceGatewayConnection, VoiceGatewayEvent, VoiceResult};

pub const DAVE_PREPARE_TRANSITION_OPCODE: u8 = 21;
pub const DAVE_EXECUTE_TRANSITION_OPCODE: u8 = 22;
pub const DAVE_READY_FOR_TRANSITION_OPCODE: u8 = 23;
pub const DAVE_PREPARE_EPOCH_OPCODE: u8 = 24;
pub const DAVE_EXTERNAL_SENDER_OPCODE: u8 = 25;
pub const DAVE_KEY_PACKAGE_OPCODE: u8 = 26;
pub const DAVE_PROPOSALS_OPCODE: u8 = 27;
pub const DAVE_COMMIT_WELCOME_OPCODE: u8 = 28;
pub const DAVE_ANNOUNCE_COMMIT_OPCODE: u8 = 29;
pub const DAVE_WELCOME_OPCODE: u8 = 30;
pub const DAVE_INVALID_COMMIT_WELCOME_OPCODE: u8 = 31;

/// Operation applied to the MLS proposal collection carried by Voice opcode 27.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DaveProposalOperation {
    Append,
    Revoke,
    Unknown(u8),
}

impl From<u8> for DaveProposalOperation {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Append,
            1 => Self::Revoke,
            unknown => Self::Unknown(unknown),
        }
    }
}

/// Typed DAVE-relevant Voice Gateway event.
///
/// MLS structures intentionally remain opaque byte strings here. Their parsing,
/// validation, group state, and cryptography belong to a DAVE provider rather
/// than the low-level Voice Gateway protocol layer.
#[derive(Debug, Clone, PartialEq)]
pub enum DaveProtocolEvent {
    ClientsConnect {
        user_ids: Vec<UserId>,
    },
    ClientDisconnect {
        user_id: UserId,
    },
    PrepareTransition {
        protocol_version: u16,
        transition_id: u16,
    },
    ExecuteTransition {
        transition_id: u16,
    },
    PrepareEpoch {
        protocol_version: u16,
        epoch: u64,
    },
    ExternalSender {
        package: Vec<u8>,
    },
    Proposals {
        operation: DaveProposalOperation,
        payload: Vec<u8>,
    },
    AnnounceCommit {
        transition_id: u16,
        commit: Vec<u8>,
    },
    Welcome {
        transition_id: u16,
        welcome: Vec<u8>,
    },
    Unknown(DaveGatewayEvent),
}

impl DaveProtocolEvent {
    /// Converts a Voice Gateway event into its DAVE protocol representation.
    /// Non-DAVE events return `Ok(None)`.
    pub fn from_gateway_event(event: &VoiceGatewayEvent) -> VoiceResult<Option<Self>> {
        let event = match event {
            VoiceGatewayEvent::ClientsConnect(value) => {
                let data = serde_json::from_value::<ClientsConnect>(value.clone())?;
                Self::ClientsConnect {
                    user_ids: data.user_ids,
                }
            }
            VoiceGatewayEvent::ClientDisconnect(value) => {
                let data = serde_json::from_value::<ClientDisconnect>(value.clone())?;
                Self::ClientDisconnect {
                    user_id: data.user_id,
                }
            }
            VoiceGatewayEvent::Dave(event) => Self::from_dave_gateway_event(event)?,
            _ => return Ok(None),
        };
        Ok(Some(event))
    }

    fn from_dave_gateway_event(event: &DaveGatewayEvent) -> VoiceResult<Self> {
        match event {
            DaveGatewayEvent::Json { opcode, data, .. } => match *opcode {
                DAVE_PREPARE_TRANSITION_OPCODE => {
                    let data = serde_json::from_value::<PrepareTransition>(data.clone())?;
                    Ok(Self::PrepareTransition {
                        protocol_version: data.protocol_version,
                        transition_id: data.transition_id,
                    })
                }
                DAVE_EXECUTE_TRANSITION_OPCODE => {
                    let data = serde_json::from_value::<TransitionId>(data.clone())?;
                    Ok(Self::ExecuteTransition {
                        transition_id: data.transition_id,
                    })
                }
                DAVE_PREPARE_EPOCH_OPCODE => {
                    let data = serde_json::from_value::<PrepareEpoch>(data.clone())?;
                    Ok(Self::PrepareEpoch {
                        protocol_version: data.protocol_version,
                        epoch: data.epoch,
                    })
                }
                _ => Ok(Self::Unknown(event.clone())),
            },
            DaveGatewayEvent::Binary {
                opcode, payload, ..
            } => match *opcode {
                DAVE_EXTERNAL_SENDER_OPCODE => Ok(Self::ExternalSender {
                    package: payload.clone(),
                }),
                DAVE_PROPOSALS_OPCODE => {
                    let (&operation, payload) = payload.split_first().ok_or_else(|| {
                        VoiceError::Protocol(
                            "DAVE proposals opcode 27 omitted its operation type".to_owned(),
                        )
                    })?;
                    Ok(Self::Proposals {
                        operation: operation.into(),
                        payload: payload.to_vec(),
                    })
                }
                DAVE_ANNOUNCE_COMMIT_OPCODE => {
                    let (transition_id, commit) = transition_payload(payload, *opcode)?;
                    Ok(Self::AnnounceCommit {
                        transition_id,
                        commit,
                    })
                }
                DAVE_WELCOME_OPCODE => {
                    let (transition_id, welcome) = transition_payload(payload, *opcode)?;
                    Ok(Self::Welcome {
                        transition_id,
                        welcome,
                    })
                }
                _ => Ok(Self::Unknown(event.clone())),
            },
        }
    }
}

fn transition_payload(payload: &[u8], opcode: u8) -> VoiceResult<(u16, Vec<u8>)> {
    if payload.len() < 2 {
        return Err(VoiceError::Protocol(format!(
            "DAVE binary opcode {opcode} omitted its big-endian transition ID"
        )));
    }
    Ok((
        u16::from_be_bytes([payload[0], payload[1]]),
        payload[2..].to_vec(),
    ))
}

#[derive(Debug, Deserialize)]
struct ClientsConnect {
    user_ids: Vec<UserId>,
}

#[derive(Debug, Deserialize)]
struct ClientDisconnect {
    user_id: UserId,
}

#[derive(Debug, Deserialize)]
struct PrepareTransition {
    protocol_version: u16,
    transition_id: u16,
}

#[derive(Debug, Deserialize)]
struct TransitionId {
    transition_id: u16,
}

#[derive(Debug, Deserialize)]
struct PrepareEpoch {
    protocol_version: u16,
    epoch: u64,
}

/// Client-to-Voice-Gateway DAVE command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaveGatewayCommand {
    ReadyForTransition { transition_id: u16 },
    KeyPackage(Vec<u8>),
    CommitWelcome(Vec<u8>),
    InvalidCommitWelcome { transition_id: u16 },
}

impl DaveGatewayCommand {
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        match self {
            Self::ReadyForTransition { .. } => DAVE_READY_FOR_TRANSITION_OPCODE,
            Self::KeyPackage(_) => DAVE_KEY_PACKAGE_OPCODE,
            Self::CommitWelcome(_) => DAVE_COMMIT_WELCOME_OPCODE,
            Self::InvalidCommitWelcome { .. } => DAVE_INVALID_COMMIT_WELCOME_OPCODE,
        }
    }

    /// Sends this typed DAVE command using the correct JSON or binary Voice
    /// Gateway framing.
    pub async fn send(&self, gateway: &mut VoiceGatewayConnection) -> VoiceResult<()> {
        if let Some(data) = self.json_data() {
            return gateway.send_dave_json(self.opcode(), &data).await;
        }
        if let Some(payload) = self.binary_payload() {
            return gateway.send_dave_binary(self.opcode(), payload).await;
        }
        Err(VoiceError::Protocol(
            "DAVE command had no serializable payload".to_owned(),
        ))
    }

    pub(crate) fn json_data(&self) -> Option<Value> {
        match self {
            Self::ReadyForTransition { transition_id }
            | Self::InvalidCommitWelcome { transition_id } => {
                Some(json!({ "transition_id": transition_id }))
            }
            Self::KeyPackage(_) | Self::CommitWelcome(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn binary_payload(&self) -> Option<&[u8]> {
        match self {
            Self::KeyPackage(payload) | Self::CommitWelcome(payload) => Some(payload),
            Self::ReadyForTransition { .. } | Self::InvalidCommitWelcome { .. } => None,
        }
    }
}

/// Current set of Discord users expected to participate in the DAVE MLS group.
///
/// This tracker is updated from Voice opcodes 11 and 13 and can be passed to a
/// provider when validating externally-generated MLS add proposals.
#[derive(Debug, Clone, Default)]
pub struct DaveParticipantSet {
    user_ids: HashSet<UserId>,
}

impl DaveParticipantSet {
    pub fn apply(&mut self, event: &DaveProtocolEvent) {
        match event {
            DaveProtocolEvent::ClientsConnect { user_ids } => {
                self.user_ids.extend(user_ids.iter().copied());
            }
            DaveProtocolEvent::ClientDisconnect { user_id } => {
                self.user_ids.remove(user_id);
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn contains(&self, user_id: UserId) -> bool {
        self.user_ids.contains(&user_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.user_ids.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.user_ids.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = UserId> + '_ {
        self.user_ids.iter().copied()
    }
}

/// Backend error returned through the provider-neutral DAVE interface.
#[derive(Debug)]
pub struct DaveProviderError {
    message: String,
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

impl DaveProviderError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    #[must_use]
    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for DaveProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DaveProviderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// Backend-neutral DAVE/MLS provider boundary.
///
/// Implementations own MLS group state and encoded-frame cryptography. Gloamwire
/// remains responsible for Voice Gateway wire framing, RTP packetization, and
/// transport AEAD.
pub trait DaveProvider: Send {
    /// Highest DAVE protocol version this provider can implement.
    fn max_protocol_version(&self) -> u16;

    /// Applies one typed Voice Gateway DAVE event and returns any required client
    /// response commands (key packages, commits/welcomes, transition readiness).
    fn handle_gateway_event(
        &mut self,
        event: &DaveProtocolEvent,
        participants: &DaveParticipantSet,
    ) -> Result<Vec<DaveGatewayCommand>, DaveProviderError>;

    /// Encrypts one complete encoded Opus frame before RTP packetization.
    fn encrypt_opus(&mut self, frame: &[u8]) -> Result<Vec<u8>, DaveProviderError>;

    /// Decrypts one complete DAVE-protected Opus frame after RTP transport AEAD.
    fn decrypt_opus(&mut self, sender: UserId, frame: &[u8]) -> Result<Vec<u8>, DaveProviderError>;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::model::UserId;

    use super::{
        DAVE_ANNOUNCE_COMMIT_OPCODE, DAVE_PROPOSALS_OPCODE, DaveGatewayCommand, DaveParticipantSet,
        DaveProposalOperation, DaveProtocolEvent,
    };
    use crate::voice::{DaveGatewayEvent, VoiceGatewayEvent};

    #[test]
    fn parses_participant_events_and_updates_expected_users() {
        let connected = VoiceGatewayEvent::ClientsConnect(json!({
            "user_ids": ["10", "20"]
        }));
        let connected = DaveProtocolEvent::from_gateway_event(&connected)
            .expect("parse clients connect")
            .expect("DAVE event");
        let mut participants = DaveParticipantSet::default();
        participants.apply(&connected);
        assert!(participants.contains(UserId::new(10)));
        assert!(participants.contains(UserId::new(20)));
        assert_eq!(participants.len(), 2);

        let disconnected = VoiceGatewayEvent::ClientDisconnect(json!({ "user_id": "10" }));
        let disconnected = DaveProtocolEvent::from_gateway_event(&disconnected)
            .expect("parse client disconnect")
            .expect("DAVE event");
        participants.apply(&disconnected);
        assert!(!participants.contains(UserId::new(10)));
        assert!(participants.contains(UserId::new(20)));
    }

    #[test]
    fn parses_json_transition_events() {
        let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Json {
            opcode: 21,
            sequence: Some(7),
            data: json!({"protocol_version": 1, "transition_id": 42}),
        });
        assert_eq!(
            DaveProtocolEvent::from_gateway_event(&event).expect("parse"),
            Some(DaveProtocolEvent::PrepareTransition {
                protocol_version: 1,
                transition_id: 42,
            })
        );

        let event = VoiceGatewayEvent::Dave(DaveGatewayEvent::Json {
            opcode: 24,
            sequence: Some(8),
            data: json!({"protocol_version": 1, "epoch": 99}),
        });
        assert_eq!(
            DaveProtocolEvent::from_gateway_event(&event).expect("parse"),
            Some(DaveProtocolEvent::PrepareEpoch {
                protocol_version: 1,
                epoch: 99,
            })
        );
    }

    #[test]
    fn parses_binary_proposals_and_big_endian_transition_ids() {
        let proposals = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
            opcode: DAVE_PROPOSALS_OPCODE,
            sequence: 9,
            payload: vec![0, 1, 2, 3],
        });
        assert_eq!(
            DaveProtocolEvent::from_gateway_event(&proposals).expect("parse"),
            Some(DaveProtocolEvent::Proposals {
                operation: DaveProposalOperation::Append,
                payload: vec![1, 2, 3],
            })
        );

        let commit = VoiceGatewayEvent::Dave(DaveGatewayEvent::Binary {
            opcode: DAVE_ANNOUNCE_COMMIT_OPCODE,
            sequence: 10,
            payload: vec![0x12, 0x34, 9, 8, 7],
        });
        assert_eq!(
            DaveProtocolEvent::from_gateway_event(&commit).expect("parse"),
            Some(DaveProtocolEvent::AnnounceCommit {
                transition_id: 0x1234,
                commit: vec![9, 8, 7],
            })
        );
    }

    #[test]
    fn preserves_unknown_dave_opcodes() {
        let raw = DaveGatewayEvent::Binary {
            opcode: 99,
            sequence: 11,
            payload: vec![1, 2],
        };
        let event = VoiceGatewayEvent::Dave(raw.clone());
        assert_eq!(
            DaveProtocolEvent::from_gateway_event(&event).expect("parse"),
            Some(DaveProtocolEvent::Unknown(raw))
        );
    }

    #[test]
    fn commands_keep_json_and_binary_wire_kinds_separate() {
        let ready = DaveGatewayCommand::ReadyForTransition { transition_id: 77 };
        assert_eq!(ready.opcode(), 23);
        assert_eq!(ready.json_data(), Some(json!({"transition_id": 77})));
        assert!(ready.binary_payload().is_none());

        let key_package = DaveGatewayCommand::KeyPackage(vec![4, 5, 6]);
        assert_eq!(key_package.opcode(), 26);
        assert_eq!(key_package.binary_payload(), Some(&[4, 5, 6][..]));
        assert!(key_package.json_data().is_none());
    }
}
