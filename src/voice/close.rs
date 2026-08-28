/// Recovery strategy for a Discord Voice Gateway close code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoiceReconnectStrategy {
    /// Attempt to resume the existing Voice Gateway session.
    Resume,
    /// Return to the main-Gateway voice rendezvous flow and create a new session.
    Restart,
    /// Do not reconnect automatically.
    Stop,
}

/// Discord Voice Gateway close code.
///
/// Unknown values are preserved so newly introduced Discord codes remain
/// observable without requiring an immediate Gloamwire release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceCloseCode(pub u16);

impl VoiceCloseCode {
    pub const UNKNOWN_OPCODE: Self = Self(4001);
    pub const DECODE_ERROR: Self = Self(4002);
    pub const NOT_AUTHENTICATED: Self = Self(4003);
    pub const AUTHENTICATION_FAILED: Self = Self(4004);
    pub const ALREADY_AUTHENTICATED: Self = Self(4005);
    pub const SESSION_INVALID: Self = Self(4006);
    pub const SESSION_TIMEOUT: Self = Self(4009);
    pub const SERVER_NOT_FOUND: Self = Self(4011);
    pub const UNKNOWN_PROTOCOL: Self = Self(4012);
    pub const DISCONNECTED: Self = Self(4014);
    pub const SERVER_CRASHED: Self = Self(4015);
    pub const UNKNOWN_ENCRYPTION_MODE: Self = Self(4016);
    pub const DAVE_REQUIRED: Self = Self(4017);
    pub const BAD_REQUEST: Self = Self(4020);
    pub const RATE_LIMITED: Self = Self(4021);
    pub const CALL_TERMINATED: Self = Self(4022);

    /// Returns the safest recovery action for this close code.
    #[must_use]
    pub const fn reconnect_strategy(self) -> VoiceReconnectStrategy {
        match self.0 {
            4015 => VoiceReconnectStrategy::Resume,
            4006 | 4009 | 4011 => VoiceReconnectStrategy::Restart,
            4014 | 4017 | 4021 | 4022 => VoiceReconnectStrategy::Stop,
            4001 | 4002 | 4003 | 4005 | 4012 | 4016 | 4020 => VoiceReconnectStrategy::Restart,
            4004 => VoiceReconnectStrategy::Stop,
            _ => VoiceReconnectStrategy::Resume,
        }
    }
}

impl From<u16> for VoiceCloseCode {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<VoiceCloseCode> for u16 {
    fn from(value: VoiceCloseCode) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::{VoiceCloseCode, VoiceReconnectStrategy};

    #[test]
    fn server_crash_is_resumable() {
        assert_eq!(
            VoiceCloseCode::SERVER_CRASHED.reconnect_strategy(),
            VoiceReconnectStrategy::Resume
        );
    }

    #[test]
    fn e2ee_requirement_is_terminal_for_current_configuration() {
        assert_eq!(
            VoiceCloseCode::DAVE_REQUIRED.reconnect_strategy(),
            VoiceReconnectStrategy::Stop
        );
    }
}
