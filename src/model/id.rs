use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::Snowflake;

macro_rules! snowflake_ids {
    ($($name:ident),+ $(,)?) => {
        $(
            #[doc = concat!("A strongly typed Discord `", stringify!($name), "` snowflake.")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct $name(Snowflake);

            impl $name {
                /// Creates an ID from its raw snowflake value.
                #[must_use]
                pub const fn new(value: u64) -> Self {
                    Self(Snowflake::new(value))
                }

                /// Returns the raw numeric snowflake value.
                #[must_use]
                pub const fn get(self) -> u64 {
                    self.0.get()
                }

                /// Returns the creation timestamp encoded by this snowflake, in Unix milliseconds.
                #[must_use]
                pub const fn timestamp_millis(self) -> u64 {
                    self.0.timestamp_millis()
                }

                /// Returns the underlying generic snowflake.
                #[must_use]
                pub const fn snowflake(self) -> Snowflake {
                    self.0
                }
            }

            impl fmt::Display for $name {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.0.fmt(formatter)
                }
            }

            impl FromStr for $name {
                type Err = std::num::ParseIntError;

                fn from_str(value: &str) -> Result<Self, Self::Err> {
                    value.parse::<Snowflake>().map(Self)
                }
            }

            impl From<u64> for $name {
                fn from(value: u64) -> Self {
                    Self::new(value)
                }
            }

            impl From<Snowflake> for $name {
                fn from(value: Snowflake) -> Self {
                    Self(value)
                }
            }

            impl From<$name> for Snowflake {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            impl From<$name> for u64 {
                fn from(value: $name) -> Self {
                    value.get()
                }
            }
        )+
    };
}

snowflake_ids!(
    ApplicationId,
    AttachmentId,
    ChannelId,
    CommandId,
    EmojiId,
    EntitlementId,
    GuildId,
    InteractionId,
    MessageId,
    RoleId,
    ScheduledEventId,
    SkuId,
    SoundboardSoundId,
    StickerId,
    UserId,
    WebhookId,
);

#[cfg(test)]
mod tests {
    use super::GuildId;

    #[test]
    fn typed_ids_keep_discord_string_serialization() {
        let id = GuildId::new(123);
        assert_eq!(serde_json::to_string(&id).expect("serialize"), "\"123\"");
    }

    #[test]
    fn typed_ids_parse_from_decimal_strings() {
        let id: GuildId = "123".parse().expect("parse");
        assert_eq!(id, GuildId::new(123));
    }
}
