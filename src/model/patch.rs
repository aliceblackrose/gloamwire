use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Distinguishes omitted properties from explicit null and supplied values.
/// Use with serde(default) on fields in partial protocol payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Absent,
    Null,
    Value(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

impl<T> Patch<T> {
    /// True when the property was omitted from the payload.
    #[must_use]
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Returns the supplied value, if any.
    #[must_use]
    pub const fn as_option(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => value.serialize(serializer),
            Self::Null | Self::Absent => serializer.serialize_none(),
        }
    }
}

impl<T: Clone> Patch<T> {
    /// Applies a partial update to a nullable model property.
    pub fn apply_to(&self, target: &mut Option<T>) {
        match self {
            Self::Absent => {}
            Self::Null => *target = None,
            Self::Value(value) => *target = Some(value.clone()),
        }
    }
}
