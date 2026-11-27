use serde::{Deserialize, Deserializer};

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
