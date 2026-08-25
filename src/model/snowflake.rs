use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Discord's epoch in Unix milliseconds (2015-01-01T00:00:00Z).
pub const DISCORD_EPOCH_MILLIS: u64 = 1_420_070_400_000;

/// A Discord snowflake identifier.
///
/// Discord serializes snowflakes as decimal strings in JSON. Gloamwire stores
/// them as `u64` while accepting either string or integer input when decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snowflake(u64);

impl Snowflake {
    /// Creates a snowflake from its raw integer value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw integer value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the creation timestamp encoded by the snowflake, in Unix milliseconds.
    #[must_use]
    pub const fn timestamp_millis(self) -> u64 {
        (self.0 >> 22) + DISCORD_EPOCH_MILLIS
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Snowflake> for u64 {
    fn from(value: Snowflake) -> Self {
        value.0
    }
}

impl FromStr for Snowflake {
    type Err = std::num::ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SnowflakeVisitor;

        impl de::Visitor<'_> for SnowflakeVisitor {
            type Value = Snowflake;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a Discord snowflake encoded as a decimal string or u64")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Snowflake(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map(Snowflake).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(SnowflakeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_string() {
        let value = serde_json::to_string(&Snowflake::new(123)).expect("serialize snowflake");
        assert_eq!(value, "\"123\"");
    }

    #[test]
    fn deserializes_string_and_integer() {
        let string: Snowflake = serde_json::from_str("\"123\"").expect("string snowflake");
        let integer: Snowflake = serde_json::from_str("123").expect("integer snowflake");
        assert_eq!(string, Snowflake::new(123));
        assert_eq!(integer, Snowflake::new(123));
    }
}
