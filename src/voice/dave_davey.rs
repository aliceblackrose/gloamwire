use std::{collections::HashMap, num::NonZeroU16};

use davey::{DAVE_PROTOCOL_VERSION, DaveSession, MediaType, ProposalsOperationType};

use crate::model::{ChannelId, UserId};

use super::{
    DaveGatewayCommand, DaveParticipantSet, DaveProposalOperation, DaveProtocolEvent, DaveProvider,
    DaveProviderError, DaveProviderLifecycle,
};

/// Pure-Rust DAVE/MLS provider backed by the optional `davey` crate.
///
/// This provider owns the OpenMLS session, DAVE transition state, sender
/// ratchets, and encoded-Opus encryption/decryption. Gloamwire remains
/// responsible for Voice Gateway framing, RTP packetization, and transport AEAD.
pub struct DaveyProvider {
    user_id: UserId,
    channel_id: ChannelId,
    active_protocol_version: u16,
    session: Option<DaveSession>,
    pending_transitions: HashMap<u16, u16>,
    downgraded: bool,
}

impl std::fmt::Debug for DaveyProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaveyProvider")
            .field("user_id", &self.user_id)
            .field("channel_id", &self.channel_id)
            .field("active_protocol_version", &self.active_protocol_version)
            .field("ready", &self.is_ready())
            .field("pending_transitions", &self.pending_transitions.len())
            .field("downgraded", &self.downgraded)
            .finish_non_exhaustive()
    }
}

impl DaveyProvider {
    /// Creates a provider for one Discord user and voice channel.
    ///
    /// The provider starts in transport-only mode. Call
    /// [`DaveProviderLifecycle::configure_protocol_version`] with the Voice
    /// Gateway Session Description value after transport negotiation.
    #[must_use]
    pub fn new(user_id: UserId, channel_id: ChannelId) -> Self {
        Self {
            user_id,
            channel_id,
            active_protocol_version: 0,
            session: None,
            pending_transitions: HashMap::new(),
            downgraded: false,
        }
    }

    /// Returns the voice privacy code for the established MLS group, when ready.
    #[must_use]
    pub fn voice_privacy_code(&self) -> Option<&str> {
        self.session
            .as_ref()
            .and_then(DaveSession::voice_privacy_code)
    }

    /// Returns whether the underlying DAVE session can currently decrypt a
    /// transport-only frame from `sender` during a transition grace period.
    #[must_use]
    pub fn can_passthrough(&self, sender: UserId) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.can_passthrough(sender.get()))
    }

    fn validate_protocol_version(protocol_version: u16) -> Result<(), DaveProviderError> {
        if protocol_version > DAVE_PROTOCOL_VERSION {
            return Err(DaveProviderError::new(format!(
                "davey supports DAVE protocol versions up to {DAVE_PROTOCOL_VERSION}, Discord selected {protocol_version}"
            )));
        }
        Ok(())
    }

    fn nonzero_version(protocol_version: u16) -> Result<NonZeroU16, DaveProviderError> {
        NonZeroU16::new(protocol_version)
            .ok_or_else(|| DaveProviderError::new("DAVE protocol version zero has no MLS session"))
    }

    fn initialize_mls(&mut self, protocol_version: u16) -> Result<Vec<u8>, DaveProviderError> {
        Self::validate_protocol_version(protocol_version)?;
        let protocol_version = Self::nonzero_version(protocol_version)?;

        match &mut self.session {
            Some(session) => session
                .reinit(
                    protocol_version,
                    self.user_id.get(),
                    self.channel_id.get(),
                    None,
                )
                .map_err(|error| {
                    DaveProviderError::new(format!("failed to reinitialize davey MLS: {error}"))
                })?,
            None => {
                self.session = Some(
                    DaveSession::new(
                        protocol_version,
                        self.user_id.get(),
                        self.channel_id.get(),
                        None,
                    )
                    .map_err(|error| {
                        DaveProviderError::new(format!("failed to initialize davey MLS: {error}"))
                    })?,
                );
            }
        }

        self.session
            .as_mut()
            .expect("DAVE session was initialized above")
            .create_key_package()
            .map_err(|error| {
                DaveProviderError::new(format!("failed to create DAVE MLS key package: {error}"))
            })
    }

    fn execute_transition(&mut self, transition_id: u16) -> Result<(), DaveProviderError> {
        let Some(next_version) = self.pending_transitions.remove(&transition_id) else {
            return Err(DaveProviderError::new(format!(
                "received DAVE execute transition {transition_id} without a prepared transition"
            )));
        };

        let old_version = self.active_protocol_version;
        self.active_protocol_version = next_version;

        if old_version != next_version && next_version == 0 {
            self.downgraded = true;
        } else if transition_id > 0 && self.downgraded && next_version > 0 {
            self.downgraded = false;
            if let Some(session) = &mut self.session {
                session.set_passthrough_mode(true, Some(10));
            }
        }

        Ok(())
    }

    fn process_commit_or_welcome(
        &mut self,
        transition_id: u16,
        process: impl FnOnce(&mut DaveSession) -> Result<(), String>,
    ) -> Result<Vec<DaveGatewayCommand>, DaveProviderError> {
        let Some(session) = &mut self.session else {
            return Err(DaveProviderError::new(
                "received DAVE MLS commit/welcome before initializing davey",
            ));
        };

        match process(session) {
            Ok(()) => {
                if transition_id == 0 {
                    return Ok(Vec::new());
                }
                self.pending_transitions
                    .insert(transition_id, self.active_protocol_version);
                Ok(vec![DaveGatewayCommand::ReadyForTransition {
                    transition_id,
                }])
            }
            Err(_error) => {
                let mut commands = vec![DaveGatewayCommand::InvalidCommitWelcome { transition_id }];
                if self.active_protocol_version > 0 {
                    let key_package = self.initialize_mls(self.active_protocol_version)?;
                    commands.push(DaveGatewayCommand::KeyPackage(key_package));
                }
                Ok(commands)
            }
        }
    }
}

impl DaveProviderLifecycle for DaveyProvider {
    fn configure_protocol_version(
        &mut self,
        protocol_version: u16,
    ) -> Result<Vec<DaveGatewayCommand>, DaveProviderError> {
        Self::validate_protocol_version(protocol_version)?;
        let old_version = self.active_protocol_version;
        self.active_protocol_version = protocol_version;
        self.pending_transitions.clear();

        if protocol_version == 0 {
            if let Some(session) = &mut self.session {
                session.reset().map_err(|error| {
                    DaveProviderError::new(format!("failed to reset davey MLS session: {error}"))
                })?;
                session.set_passthrough_mode(true, Some(10));
            }
            self.downgraded = old_version > 0;
            return Ok(Vec::new());
        }

        let key_package = self.initialize_mls(protocol_version)?;
        self.downgraded = false;
        Ok(vec![DaveGatewayCommand::KeyPackage(key_package)])
    }

    fn active_protocol_version(&self) -> u16 {
        self.active_protocol_version
    }

    fn is_ready(&self) -> bool {
        self.session.as_ref().is_some_and(DaveSession::is_ready)
    }
}

impl DaveProvider for DaveyProvider {
    fn max_protocol_version(&self) -> u16 {
        DAVE_PROTOCOL_VERSION
    }

    fn handle_gateway_event(
        &mut self,
        event: &DaveProtocolEvent,
        participants: &DaveParticipantSet,
    ) -> Result<Vec<DaveGatewayCommand>, DaveProviderError> {
        match event {
            DaveProtocolEvent::ClientsConnect { .. }
            | DaveProtocolEvent::ClientDisconnect { .. }
            | DaveProtocolEvent::Unknown(_) => Ok(Vec::new()),
            DaveProtocolEvent::PrepareTransition {
                protocol_version,
                transition_id,
            } => {
                Self::validate_protocol_version(*protocol_version)?;
                self.pending_transitions
                    .insert(*transition_id, *protocol_version);

                if *transition_id == 0 {
                    self.execute_transition(*transition_id)?;
                    return Ok(Vec::new());
                }

                if *protocol_version == 0
                    && let Some(session) = &mut self.session
                {
                    session.set_passthrough_mode(true, Some(120));
                }

                Ok(vec![DaveGatewayCommand::ReadyForTransition {
                    transition_id: *transition_id,
                }])
            }
            DaveProtocolEvent::ExecuteTransition { transition_id } => {
                self.execute_transition(*transition_id)?;
                Ok(Vec::new())
            }
            DaveProtocolEvent::PrepareEpoch {
                protocol_version,
                epoch,
            } => {
                if *epoch == 1 {
                    self.configure_protocol_version(*protocol_version)
                } else {
                    Ok(Vec::new())
                }
            }
            DaveProtocolEvent::ExternalSender { package } => {
                let Some(session) = &mut self.session else {
                    return Err(DaveProviderError::new(
                        "received DAVE external sender before initializing davey",
                    ));
                };
                session.set_external_sender(package).map_err(|error| {
                    DaveProviderError::new(format!(
                        "failed to configure DAVE MLS external sender: {error}"
                    ))
                })?;
                Ok(Vec::new())
            }
            DaveProtocolEvent::Proposals { operation, payload } => {
                let operation = match operation {
                    DaveProposalOperation::Append => ProposalsOperationType::APPEND,
                    DaveProposalOperation::Revoke => ProposalsOperationType::REVOKE,
                    DaveProposalOperation::Unknown(value) => {
                        return Err(DaveProviderError::new(format!(
                            "unsupported DAVE MLS proposal operation {value}"
                        )));
                    }
                };
                let expected_user_ids: Vec<u64> = participants.iter().map(UserId::get).collect();
                let Some(session) = &mut self.session else {
                    return Err(DaveProviderError::new(
                        "received DAVE MLS proposals before initializing davey",
                    ));
                };
                let commit_welcome = session
                    .process_proposals(operation, payload, Some(&expected_user_ids))
                    .map_err(|error| {
                        DaveProviderError::new(format!(
                            "failed to process DAVE MLS proposals: {error}"
                        ))
                    })?;

                let Some(commit_welcome) = commit_welcome else {
                    return Ok(Vec::new());
                };
                let mut wire = commit_welcome.commit;
                if let Some(welcome) = commit_welcome.welcome {
                    wire.extend_from_slice(&welcome);
                }
                Ok(vec![DaveGatewayCommand::CommitWelcome(wire)])
            }
            DaveProtocolEvent::AnnounceCommit {
                transition_id,
                commit,
            } => self.process_commit_or_welcome(*transition_id, |session| {
                session
                    .process_commit(commit)
                    .map_err(|error| error.to_string())
            }),
            DaveProtocolEvent::Welcome {
                transition_id,
                welcome,
            } => self.process_commit_or_welcome(*transition_id, |session| {
                session
                    .process_welcome(welcome)
                    .map_err(|error| error.to_string())
            }),
        }
    }

    fn encrypt_opus(&mut self, frame: &[u8]) -> Result<Vec<u8>, DaveProviderError> {
        if self.active_protocol_version == 0 {
            return Ok(frame.to_vec());
        }
        let Some(session) = &mut self.session else {
            return Err(DaveProviderError::new(
                "DAVE is active but davey has no MLS session",
            ));
        };
        if !session.is_ready() {
            return Err(DaveProviderError::new(
                "DAVE MLS session is not ready to encrypt Opus",
            ));
        }
        session
            .encrypt_opus(frame)
            .map(|frame| frame.into_owned())
            .map_err(|error| {
                DaveProviderError::new(format!("failed to DAVE-encrypt Opus: {error}"))
            })
    }

    fn decrypt_opus(&mut self, sender: UserId, frame: &[u8]) -> Result<Vec<u8>, DaveProviderError> {
        let Some(session) = &mut self.session else {
            if self.active_protocol_version == 0 {
                return Ok(frame.to_vec());
            }
            return Err(DaveProviderError::new(
                "DAVE is active but davey has no MLS session",
            ));
        };

        let should_decrypt = self.active_protocol_version > 0
            || (session.is_ready() && session.can_passthrough(sender.get()));
        if !should_decrypt {
            return Ok(frame.to_vec());
        }

        session
            .decrypt(sender.get(), MediaType::AUDIO, frame)
            .map_err(|error| {
                DaveProviderError::new(format!("failed to DAVE-decrypt Opus: {error}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{ChannelId, UserId};

    use super::DaveyProvider;
    use crate::voice::{DAVE_KEY_PACKAGE_OPCODE, DaveProvider, DaveProviderLifecycle};

    #[test]
    fn configures_protocol_one_and_generates_key_package() {
        let mut provider = DaveyProvider::new(UserId::new(10), ChannelId::new(20));
        assert_eq!(provider.max_protocol_version(), 1);
        let commands = provider
            .configure_protocol_version(1)
            .expect("configure DAVE v1");
        assert_eq!(provider.active_protocol_version(), 1);
        assert!(!provider.is_ready());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].opcode(), DAVE_KEY_PACKAGE_OPCODE);
    }

    #[test]
    fn transport_only_configuration_does_not_create_mls() {
        let mut provider = DaveyProvider::new(UserId::new(10), ChannelId::new(20));
        let commands = provider
            .configure_protocol_version(0)
            .expect("configure transport-only");
        assert!(commands.is_empty());
        assert_eq!(provider.active_protocol_version(), 0);
        assert!(!provider.is_ready());
    }
}
