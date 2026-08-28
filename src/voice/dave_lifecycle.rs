use super::{DaveGatewayCommand, DaveProvider, DaveProviderError};

/// Optional lifecycle contract for DAVE providers managed by [`super::VoiceSession`].
///
/// The base [`DaveProvider`] trait only models MLS events and media transforms.
/// Providers implementing this trait additionally let Gloamwire initialize and
/// reinitialize them from the Voice Gateway's negotiated DAVE protocol version.
pub trait DaveProviderLifecycle: DaveProvider {
    /// Configures the selected DAVE protocol version.
    ///
    /// Version zero means transport-only media. Non-zero versions should create
    /// or reinitialize the provider's MLS session and return any immediate Voice
    /// Gateway commands, typically an MLS key package.
    fn configure_protocol_version(
        &mut self,
        protocol_version: u16,
    ) -> Result<Vec<DaveGatewayCommand>, DaveProviderError>;

    /// Returns the DAVE protocol version currently active for outbound media.
    fn active_protocol_version(&self) -> u16;

    /// Returns whether the provider has established media ratchets and can
    /// encrypt/decrypt DAVE frames for the active epoch.
    fn is_ready(&self) -> bool;
}
