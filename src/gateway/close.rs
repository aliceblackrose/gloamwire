/// Discord Gateway close codes understood by Gloamwire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GatewayCloseCode {
    /// WebSocket normal closure (`1000`).
    Normal,
    /// WebSocket endpoint going away (`1001`).
    GoingAway,
    /// Discord could not determine the Gateway failure (`4000`).
    UnknownError,
    /// The client sent an unknown opcode (`4001`).
    UnknownOpcode,
    /// The client sent an invalid payload (`4002`).
    DecodeError,
    /// The client sent a payload before authenticating (`4003`).
    NotAuthenticated,
    /// The bot token was rejected (`4004`).
    AuthenticationFailed,
    /// The client attempted to authenticate twice (`4005`).
    AlreadyAuthenticated,
    /// The sequence number supplied to Resume was invalid (`4007`).
    InvalidSequence,
    /// The connection exceeded a Gateway rate limit (`4008`).
    RateLimited,
    /// The Gateway session timed out (`4009`).
    SessionTimedOut,
    /// The shard configuration was invalid (`4010`).
    InvalidShard,
    /// Discord requires the application to use sharding (`4011`).
    ShardingRequired,
    /// The requested Gateway API version was invalid (`4012`).
    InvalidApiVersion,
    /// The Identify payload contained invalid intents (`4013`).
    InvalidIntents,
    /// The application is not permitted to use one or more requested intents (`4014`).
    DisallowedIntents,
    /// A close code not modeled by this version of Gloamwire.
    Other(u16),
}

impl GatewayCloseCode {
    /// Returns the numeric WebSocket close code.
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::Normal => 1000,
            Self::GoingAway => 1001,
            Self::UnknownError => 4000,
            Self::UnknownOpcode => 4001,
            Self::DecodeError => 4002,
            Self::NotAuthenticated => 4003,
            Self::AuthenticationFailed => 4004,
            Self::AlreadyAuthenticated => 4005,
            Self::InvalidSequence => 4007,
            Self::RateLimited => 4008,
            Self::SessionTimedOut => 4009,
            Self::InvalidShard => 4010,
            Self::ShardingRequired => 4011,
            Self::InvalidApiVersion => 4012,
            Self::InvalidIntents => 4013,
            Self::DisallowedIntents => 4014,
            Self::Other(code) => code,
        }
    }

    /// Returns the reconnect strategy implied by this close code.
    #[must_use]
    pub const fn reconnect_strategy(self) -> GatewayReconnectStrategy {
        match self {
            Self::AuthenticationFailed
            | Self::InvalidShard
            | Self::ShardingRequired
            | Self::InvalidApiVersion
            | Self::InvalidIntents
            | Self::DisallowedIntents => GatewayReconnectStrategy::Stop,
            Self::Normal | Self::GoingAway | Self::InvalidSequence | Self::SessionTimedOut => {
                GatewayReconnectStrategy::Reidentify
            }
            Self::UnknownError
            | Self::UnknownOpcode
            | Self::DecodeError
            | Self::NotAuthenticated
            | Self::AlreadyAuthenticated
            | Self::RateLimited
            | Self::Other(_) => GatewayReconnectStrategy::Resume,
        }
    }
}

impl From<u16> for GatewayCloseCode {
    fn from(code: u16) -> Self {
        match code {
            1000 => Self::Normal,
            1001 => Self::GoingAway,
            4000 => Self::UnknownError,
            4001 => Self::UnknownOpcode,
            4002 => Self::DecodeError,
            4003 => Self::NotAuthenticated,
            4004 => Self::AuthenticationFailed,
            4005 => Self::AlreadyAuthenticated,
            4007 => Self::InvalidSequence,
            4008 => Self::RateLimited,
            4009 => Self::SessionTimedOut,
            4010 => Self::InvalidShard,
            4011 => Self::ShardingRequired,
            4012 => Self::InvalidApiVersion,
            4013 => Self::InvalidIntents,
            4014 => Self::DisallowedIntents,
            code => Self::Other(code),
        }
    }
}

/// How a Gateway connection should recover from a disconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GatewayReconnectStrategy {
    /// Attempt to resume the existing Gateway session.
    Resume,
    /// Start a new Gateway session with Identify.
    Reidentify,
    /// Do not reconnect automatically.
    Stop,
}

#[cfg(test)]
mod tests {
    use super::{GatewayCloseCode, GatewayReconnectStrategy};

    #[test]
    fn invalid_sequence_requires_new_session() {
        assert_eq!(
            GatewayCloseCode::InvalidSequence.reconnect_strategy(),
            GatewayReconnectStrategy::Reidentify
        );
    }

    #[test]
    fn authentication_failure_is_fatal() {
        assert_eq!(
            GatewayCloseCode::AuthenticationFailed.reconnect_strategy(),
            GatewayReconnectStrategy::Stop
        );
    }

    #[test]
    fn unknown_codes_remain_forward_compatible() {
        assert_eq!(GatewayCloseCode::from(4999), GatewayCloseCode::Other(4999));
    }
}
