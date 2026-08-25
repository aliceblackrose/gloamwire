use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{ApplicationId, EntitlementId, GuildId, SkuId, Snowflake, SubscriptionId, UserId};

/// Discord SKU type.
///
/// The numeric representation is retained so future SKU types remain
/// deserializable without requiring an immediate Gloamwire release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkuType(pub u8);

impl SkuType {
    pub const DURABLE: Self = Self(2);
    pub const CONSUMABLE: Self = Self(3);
    pub const SUBSCRIPTION: Self = Self(5);
    pub const SUBSCRIPTION_GROUP: Self = Self(6);
}

bitflags! {
    /// Flags describing how a Discord SKU can be purchased and applied.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct SkuFlags: u64 {
        const AVAILABLE = 1 << 2;
        const GUILD_SUBSCRIPTION = 1 << 7;
        const USER_SUBSCRIPTION = 1 << 8;
    }
}

impl Serialize for SkuFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for SkuFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Self::from_bits_retain)
    }
}

/// A premium offering exposed by a Discord application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sku {
    pub id: SkuId,
    #[serde(rename = "type")]
    pub kind: SkuType,
    pub application_id: ApplicationId,
    pub name: String,
    pub slug: String,
    pub flags: SkuFlags,
}

/// Discord entitlement type.
///
/// The numeric representation is retained for forward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntitlementType(pub u8);

impl EntitlementType {
    pub const PURCHASE: Self = Self(1);
    pub const PREMIUM_SUBSCRIPTION: Self = Self(2);
    pub const DEVELOPER_GIFT: Self = Self(3);
    pub const TEST_MODE_PURCHASE: Self = Self(4);
    pub const FREE_PURCHASE: Self = Self(5);
    pub const USER_GIFT: Self = Self(6);
    pub const PREMIUM_PURCHASE: Self = Self(7);
    pub const APPLICATION_SUBSCRIPTION: Self = Self(8);
}

/// Access granted to a user or guild for one Discord SKU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entitlement {
    pub id: EntitlementId,
    pub sku_id: SkuId,
    pub application_id: ApplicationId,
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(rename = "type")]
    pub kind: EntitlementType,
    pub deleted: bool,
    #[serde(default)]
    pub starts_at: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default)]
    pub consumed: Option<bool>,
    /// Subscription associated with this entitlement when Discord provides one.
    #[serde(default)]
    pub subscription_id: Option<SubscriptionId>,
    /// Promotion identifier present in some Discord entitlement payloads.
    #[serde(default)]
    pub promotion_id: Option<Snowflake>,
    /// Gift-code flags present in some Discord entitlement payloads.
    #[serde(default)]
    pub gift_code_flags: Option<u64>,
}

/// Current lifecycle status of a Discord subscription.
///
/// Subscription status is reporting data; entitlement state is the source of
/// truth for whether premium access should be granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubscriptionStatus(pub u8);

impl SubscriptionStatus {
    pub const ACTIVE: Self = Self(0);
    pub const INACTIVE: Self = Self(1);
    pub const ENDING: Self = Self(2);
}

/// A recurring Discord subscription containing one or more SKUs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub user_id: UserId,
    #[serde(default)]
    pub sku_ids: Vec<SkuId>,
    #[serde(default)]
    pub entitlement_ids: Vec<EntitlementId>,
    #[serde(default)]
    pub renewal_sku_ids: Option<Vec<SkuId>>,
    pub current_period_start: String,
    pub current_period_end: String,
    pub status: SubscriptionStatus,
    #[serde(default)]
    pub canceled_at: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        Entitlement, EntitlementType, Sku, SkuFlags, SkuType, Subscription, SubscriptionStatus,
    };

    #[test]
    fn parses_subscription_sku_and_flags() {
        let sku: Sku = serde_json::from_str(
            r#"{
                "id":"1088510058284990888",
                "type":5,
                "application_id":"788708323867885999",
                "name":"Test Premium",
                "slug":"test-premium",
                "flags":128
            }"#,
        )
        .expect("sku");

        assert_eq!(sku.kind, SkuType::SUBSCRIPTION);
        assert!(sku.flags.contains(SkuFlags::GUILD_SUBSCRIPTION));
    }

    #[test]
    fn parses_current_entitlement_relationships() {
        let entitlement: Entitlement = serde_json::from_str(
            r#"{
                "id":"1019653849998299136",
                "sku_id":"1019475255913222144",
                "application_id":"1019370614521200640",
                "user_id":"771129655544643584",
                "promotion_id":null,
                "type":1,
                "deleted":false,
                "gift_code_flags":0,
                "consumed":false,
                "starts_at":"2026-08-01T00:00:00+00:00",
                "ends_at":null,
                "guild_id":"1015034326372454400",
                "subscription_id":"1019653835926409216"
            }"#,
        )
        .expect("entitlement");

        assert_eq!(entitlement.kind, EntitlementType::PURCHASE);
        assert_eq!(
            entitlement.subscription_id.expect("subscription").get(),
            1019653835926409216
        );
        assert_eq!(entitlement.consumed, Some(false));
    }

    #[test]
    fn parses_current_subscription() {
        let subscription: Subscription = serde_json::from_str(
            r#"{
                "id":"1278078770116427839",
                "user_id":"1088605110638227537",
                "sku_ids":["1158857122189168803"],
                "entitlement_ids":["1278078770116427840"],
                "renewal_sku_ids":null,
                "current_period_start":"2026-08-01T19:48:44.406602+00:00",
                "current_period_end":"2026-09-01T19:48:44.406602+00:00",
                "status":0,
                "canceled_at":null
            }"#,
        )
        .expect("subscription");

        assert_eq!(subscription.status, SubscriptionStatus::ACTIVE);
        assert_eq!(subscription.sku_ids[0].get(), 1158857122189168803);
    }

    #[test]
    fn monetization_numeric_types_preserve_unknown_values() {
        let sku_type: SkuType = serde_json::from_str("99").expect("sku type");
        let entitlement_type: EntitlementType =
            serde_json::from_str("99").expect("entitlement type");
        let status: SubscriptionStatus = serde_json::from_str("99").expect("subscription status");

        assert_eq!(sku_type, SkuType(99));
        assert_eq!(entitlement_type, EntitlementType(99));
        assert_eq!(status, SubscriptionStatus(99));
    }
}
