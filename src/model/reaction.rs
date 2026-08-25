use serde::{Deserialize, Serialize};

use super::EmojiId;

/// Partial emoji data embedded in reactions and reaction Gateway events.
///
/// Discord may omit `animated`, and the name of a deleted or otherwise
/// unavailable custom emoji can be `null`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartialEmoji {
    #[serde(default)]
    pub id: Option<EmojiId>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub animated: Option<bool>,
}

/// Discord reaction type.
///
/// This remains a numeric newtype so future reaction types can be preserved
/// without making deserialization fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReactionType(pub u8);

impl ReactionType {
    /// A normal reaction.
    pub const NORMAL: Self = Self(0);
    /// A super reaction.
    pub const BURST: Self = Self(1);
}

/// Breakdown of normal and super-reaction counts for an emoji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReactionCountDetails {
    pub burst: u64,
    pub normal: u64,
}

/// Reactions for one emoji on a Discord message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    /// Total normal and super reactions for this emoji.
    pub count: u64,
    /// Normal/super reaction count breakdown.
    pub count_details: ReactionCountDetails,
    /// Whether the current user added a normal reaction with this emoji.
    pub me: bool,
    /// Whether the current user added a super reaction with this emoji.
    pub me_burst: bool,
    /// Emoji used for the reaction.
    pub emoji: PartialEmoji,
    /// Hex colors used for the super-reaction animation.
    #[serde(default)]
    pub burst_colors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{PartialEmoji, Reaction, ReactionType};

    #[test]
    fn parses_current_reaction_shape() {
        let reaction: Reaction = serde_json::from_str(
            r##"{
                "count":3,
                "count_details":{"burst":1,"normal":2},
                "me":true,
                "me_burst":false,
                "emoji":{"id":null,"name":"🔥"},
                "burst_colors":["#ff0000"]
            }"##,
        )
        .expect("reaction");

        assert_eq!(reaction.count, 3);
        assert_eq!(reaction.count_details.burst, 1);
        assert_eq!(reaction.count_details.normal, 2);
        assert_eq!(reaction.emoji.name.as_deref(), Some("🔥"));
    }

    #[test]
    fn partial_emoji_allows_deleted_custom_emoji_name() {
        let emoji: PartialEmoji =
            serde_json::from_str(r#"{"id":"41771983429993937","name":null,"animated":true}"#)
                .expect("partial emoji");

        assert_eq!(emoji.id.expect("emoji id").get(), 41_771_983_429_993_937);
        assert!(emoji.name.is_none());
        assert_eq!(emoji.animated, Some(true));
    }

    #[test]
    fn reaction_type_preserves_unknown_values() {
        let kind: ReactionType = serde_json::from_str("7").expect("reaction type");
        assert_eq!(kind, ReactionType(7));
    }
}
