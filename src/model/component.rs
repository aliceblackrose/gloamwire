use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ChannelType, PartialEmoji, SkuId, Snowflake};

/// Discord component type.
///
/// This remains a numeric newtype so future component types can be retained
/// without making deserialization fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentType(pub u8);

impl ComponentType {
    pub const ACTION_ROW: Self = Self(1);
    pub const BUTTON: Self = Self(2);
    pub const STRING_SELECT: Self = Self(3);
    pub const TEXT_INPUT: Self = Self(4);
    pub const USER_SELECT: Self = Self(5);
    pub const ROLE_SELECT: Self = Self(6);
    pub const MENTIONABLE_SELECT: Self = Self(7);
    pub const CHANNEL_SELECT: Self = Self(8);
    pub const SECTION: Self = Self(9);
    pub const TEXT_DISPLAY: Self = Self(10);
    pub const THUMBNAIL: Self = Self(11);
    pub const MEDIA_GALLERY: Self = Self(12);
    pub const FILE: Self = Self(13);
    pub const SEPARATOR: Self = Self(14);
    pub const CONTAINER: Self = Self(17);
    pub const LABEL: Self = Self(18);
    pub const FILE_UPLOAD: Self = Self(19);
    pub const RADIO_GROUP: Self = Self(21);
    pub const CHECKBOX_GROUP: Self = Self(22);
    pub const CHECKBOX: Self = Self(23);
}

/// Style field shared by component types whose wire format uses an integer style.
///
/// Button and text-input styles intentionally share this forward-compatible
/// numeric representation because Discord assigns overlapping values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentStyle(pub u8);

impl ComponentStyle {
    pub const PRIMARY: Self = Self(1);
    pub const SECONDARY: Self = Self(2);
    pub const SUCCESS: Self = Self(3);
    pub const DANGER: Self = Self(4);
    pub const LINK: Self = Self(5);
    pub const PREMIUM: Self = Self(6);

    pub const SHORT: Self = Self(1);
    pub const PARAGRAPH: Self = Self(2);
}

/// Styles for buttons that send an interaction to the application.
///
/// Link and premium buttons use dedicated constructors because they do not
/// send button-click interactions and require different fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractiveButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
}

impl From<InteractiveButtonStyle> for ComponentStyle {
    fn from(style: InteractiveButtonStyle) -> Self {
        match style {
            InteractiveButtonStyle::Primary => Self::PRIMARY,
            InteractiveButtonStyle::Secondary => Self::SECONDARY,
            InteractiveButtonStyle::Success => Self::SUCCESS,
            InteractiveButtonStyle::Danger => Self::DANGER,
        }
    }
}

/// Separator spacing value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeparatorSpacing(pub u8);

impl SeparatorSpacing {
    pub const SMALL: Self = Self(1);
    pub const LARGE: Self = Self(2);
}

/// Default entity selected in an auto-populated select component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectDefaultValue {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: String,
}

/// One selectable option used by string selects, radio groups, and checkbox groups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentOption {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// User-provided or API-returned scalar component value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComponentValue {
    Boolean(bool),
    String(String),
}

/// Media reference used by Components V2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnfurledMediaItem {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One item in a media-gallery component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaGalleryItem {
    pub media: UnfurledMediaItem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

/// A Discord message or modal component.
///
/// Discord's component system is recursive and continues to gain new component
/// types. Known fields are exposed directly while unknown future fields are
/// preserved in [`Self::extra`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    #[serde(rename = "type")]
    pub kind: ComponentType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<ComponentStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku_id: Option<SkuId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ComponentOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_values: Vec<SelectDefaultValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_types: Vec<ChannelType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ComponentValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessory: Option<Box<Component>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<UnfurledMediaItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<MediaGalleryItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<UnfurledMediaItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<SeparatorSpacing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<Box<Component>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Component {
    fn new(kind: ComponentType) -> Self {
        Self {
            kind,
            id: None,
            custom_id: None,
            style: None,
            label: None,
            emoji: None,
            sku_id: None,
            url: None,
            disabled: None,
            components: Vec::new(),
            options: Vec::new(),
            placeholder: None,
            default_values: Vec::new(),
            min_values: None,
            max_values: None,
            required: None,
            channel_types: Vec::new(),
            min_length: None,
            max_length: None,
            value: None,
            values: Vec::new(),
            content: None,
            accessory: None,
            media: None,
            description: None,
            spoiler: None,
            items: Vec::new(),
            file: None,
            name: None,
            size: None,
            divider: None,
            spacing: None,
            accent_color: None,
            component: None,
            file_types: Vec::new(),
            default: None,
            extra: BTreeMap::new(),
        }
    }

    /// Creates an interactive button with the given style and custom ID.
    ///
    /// Set a label, an emoji, or both before sending. Custom IDs must be unique
    /// within the message and contain 1–100 characters.
    #[must_use]
    pub fn button(style: InteractiveButtonStyle, custom_id: impl Into<String>) -> Self {
        Self {
            style: Some(style.into()),
            custom_id: Some(custom_id.into()),
            ..Self::new(ComponentType::BUTTON)
        }
    }

    /// Creates a link button. Set a label, an emoji, or both before sending.
    ///
    /// Link buttons open the URL instead of sending an interaction.
    #[must_use]
    pub fn link_button(url: impl Into<String>) -> Self {
        Self {
            style: Some(ComponentStyle::LINK),
            url: Some(url.into()),
            ..Self::new(ComponentType::BUTTON)
        }
    }

    /// Creates a premium purchase button for an application SKU.
    ///
    /// Discord supplies its own label; do not set a label, emoji, custom ID,
    /// or URL. Premium buttons do not send button-click interactions.
    #[must_use]
    pub fn premium_button(sku_id: SkuId) -> Self {
        Self {
            style: Some(ComponentStyle::PREMIUM),
            sku_id: Some(sku_id),
            ..Self::new(ComponentType::BUTTON)
        }
    }

    /// Creates an action row from message components.
    ///
    /// Discord permits one to five buttons or one select menu in a message
    /// action row. Modal rows have separate rules. Layout and field limits
    /// are validated by Discord; this constructor does not enforce them.
    #[must_use]
    pub fn action_row(components: impl IntoIterator<Item = Component>) -> Self {
        Self {
            components: components.into_iter().collect(),
            ..Self::new(ComponentType::ACTION_ROW)
        }
    }

    /// Sets a component label (up to 80 characters for a button).
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a component emoji, including emoji-only buttons.
    #[must_use]
    pub fn with_emoji(mut self, emoji: PartialEmoji) -> Self {
        self.emoji = Some(emoji);
        self
    }

    /// Sets whether an interactive message component is disabled.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
}

/// Data used to open a Discord modal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modal {
    pub custom_id: String,
    pub title: String,
    #[serde(default)]
    pub components: Vec<Component>,
}

#[cfg(test)]
mod tests {
    use super::{
        Component, ComponentStyle, ComponentType, ComponentValue, InteractiveButtonStyle, Modal,
    };
    use crate::model::{
        CreateMessage, EditMessage, InteractionMessageData, InteractionResponse, PartialEmoji,
        SkuId,
    };
    use serde_json::json;

    #[test]
    fn interactive_button_styles_serialize_with_custom_ids() {
        for (style, wire_style) in [
            (InteractiveButtonStyle::Primary, 1),
            (InteractiveButtonStyle::Secondary, 2),
            (InteractiveButtonStyle::Success, 3),
            (InteractiveButtonStyle::Danger, 4),
        ] {
            let button = Component::button(style, "confirm").with_label("Confirm");
            assert_eq!(
                serde_json::to_value(&button).expect("button"),
                json!({"type": 2, "style": wire_style, "custom_id": "confirm", "label": "Confirm"})
            );
        }
    }

    #[test]
    fn button_variants_use_distinct_wire_fields() {
        let link = Component::link_button("https://example.com").with_label("Open");
        let premium = Component::premium_button(SkuId::new(123)).with_disabled(true);
        assert_eq!(
            serde_json::to_value(&link).expect("link"),
            json!({"type": 2, "style": 5, "url": "https://example.com", "label": "Open"})
        );
        assert_eq!(
            serde_json::to_value(&premium).expect("premium"),
            json!({"type": 2, "style": 6, "sku_id": "123", "disabled": true})
        );
    }

    #[test]
    fn button_rows_work_in_message_requests_and_callbacks() {
        let row = Component::action_row([
            Component::button(InteractiveButtonStyle::Success, "confirm")
                .with_emoji(PartialEmoji {
                    id: None,
                    name: Some("✅".to_owned()),
                    animated: None,
                })
                .with_disabled(false),
            Component::button(InteractiveButtonStyle::Danger, "cancel")
                .with_label("Cancel")
                .with_disabled(true),
        ]);
        let expected = json!([{
            "type": 1,
            "components": [
                {"type": 2, "style": 3, "custom_id": "confirm", "disabled": false,
                 "emoji": {"id": null, "name": "✅", "animated": null}},
                {"type": 2, "style": 4, "custom_id": "cancel", "label": "Cancel", "disabled": true}
            ]
        }]);
        let message = CreateMessage {
            components: vec![row.clone()],
            ..CreateMessage::default()
        };
        let edit = EditMessage {
            components: Some(vec![row.clone()]),
            ..EditMessage::default()
        };
        let response = InteractionResponse::message(InteractionMessageData {
            components: vec![row],
            ..InteractionMessageData::default()
        });
        assert_eq!(
            serde_json::to_value(message).expect("message")["components"],
            expected
        );
        assert_eq!(
            serde_json::to_value(edit).expect("edit")["components"],
            expected
        );
        assert_eq!(
            serde_json::to_value(response).expect("response")["data"]["components"],
            expected
        );
        let parsed: Vec<Component> = serde_json::from_value(expected).expect("row");
        assert_eq!(parsed[0].components[0].kind, ComponentType::BUTTON);
    }

    #[test]
    fn parses_components_v2_container() {
        let component: Component = serde_json::from_str(
            r##"{
                "type":17,
                "accent_color":703487,
                "components":[
                    {"type":10,"content":"# Encounter"},
                    {"type":1,"components":[
                        {"type":2,"custom_id":"pet","label":"Pet it!","style":1}
                    ]}
                ]
            }"##,
        )
        .expect("container");

        assert_eq!(component.kind, ComponentType::CONTAINER);
        assert_eq!(component.components[0].kind, ComponentType::TEXT_DISPLAY);
        assert_eq!(
            component.components[1].components[0].style,
            Some(ComponentStyle::PRIMARY)
        );
    }

    #[test]
    fn parses_current_modal_components() {
        let modal: Modal = serde_json::from_str(
            r#"{
                "custom_id":"bug_modal",
                "title":"Bug report",
                "components":[
                    {
                        "type":18,
                        "label":"Class",
                        "component":{
                            "type":21,
                            "custom_id":"class_radio",
                            "options":[
                                {"label":"Warrior","value":"warrior"},
                                {"label":"Rogue","value":"rogue"}
                            ]
                        }
                    },
                    {
                        "type":18,
                        "label":"Screenshot",
                        "component":{
                            "type":19,
                            "custom_id":"file_upload",
                            "min_values":1,
                            "max_values":10,
                            "required":true,
                            "file_types":["image",".pdf"]
                        }
                    }
                ]
            }"#,
        )
        .expect("modal");

        let radio = modal.components[0]
            .component
            .as_deref()
            .expect("radio group");
        assert_eq!(radio.kind, ComponentType::RADIO_GROUP);
        assert_eq!(radio.options.len(), 2);

        let upload = modal.components[1]
            .component
            .as_deref()
            .expect("file upload");
        assert_eq!(upload.kind, ComponentType::FILE_UPLOAD);
        assert_eq!(upload.file_types, ["image", ".pdf"]);
    }

    #[test]
    fn parses_modal_interaction_scalar_values() {
        let checkbox: Component =
            serde_json::from_str(r#"{"type":23,"id":2,"custom_id":"confirm","value":true}"#)
                .expect("checkbox response");

        assert_eq!(checkbox.value, Some(ComponentValue::Boolean(true)));
    }

    #[test]
    fn preserves_unknown_component_types_and_fields() {
        let component: Component =
            serde_json::from_str(r#"{"type":99,"future_field":{"enabled":true}}"#)
                .expect("future component");

        assert_eq!(component.kind, ComponentType(99));
        assert_eq!(component.extra["future_field"]["enabled"], true);
    }
}
