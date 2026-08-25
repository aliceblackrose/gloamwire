use serde::{Deserialize, Serialize};

use super::{AttachmentId, User};

/// A file attached to a Discord message or resolved interaction option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: AttachmentId,
    pub filename: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    pub size: u64,
    pub url: String,
    pub proxy_url: String,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub placeholder_version: Option<u32>,
    #[serde(default)]
    pub ephemeral: Option<bool>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub waveform: Option<String>,
    #[serde(default)]
    pub flags: Option<u64>,
    #[serde(default)]
    pub clip_participants: Vec<User>,
    #[serde(default)]
    pub clip_created_at: Option<String>,
}
