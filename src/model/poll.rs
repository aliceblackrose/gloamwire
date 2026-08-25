use serde::{Deserialize, Serialize};

use super::PartialEmoji;

/// Discord poll layout type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PollLayoutType(pub u8);

impl PollLayoutType {
    pub const DEFAULT: Self = Self(1);
}

/// Display media used by a poll question or answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
}

/// One answer returned as part of a Discord poll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollAnswer {
    pub answer_id: u32,
    pub poll_media: PollMedia,
}

/// One answer supplied when creating a Discord poll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollCreateAnswer {
    pub poll_media: PollMedia,
}

/// Precise vote count for one poll answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollAnswerCount {
    pub id: u32,
    pub count: u64,
    pub me_voted: bool,
}

/// Aggregated poll results returned by Discord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollResults {
    pub is_finalized: bool,
    #[serde(default)]
    pub answer_counts: Vec<PollAnswerCount>,
}

/// A Discord poll embedded in a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Poll {
    pub question: PollMedia,
    #[serde(default)]
    pub answers: Vec<PollAnswer>,
    pub expiry: Option<String>,
    pub allow_multiselect: bool,
    pub layout_type: PollLayoutType,
    #[serde(default)]
    pub results: Option<PollResults>,
}

/// Poll payload accepted when creating a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollCreateRequest {
    pub question: PollMedia,
    #[serde(default)]
    pub answers: Vec<PollCreateAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_multiselect: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_type: Option<PollLayoutType>,
}

#[cfg(test)]
mod tests {
    use super::{Poll, PollCreateAnswer, PollCreateRequest, PollLayoutType, PollMedia};

    #[test]
    fn parses_current_poll_response() {
        let poll: Poll = serde_json::from_str(
            r#"{
                "question":{"text":"Choose one"},
                "answers":[
                    {"answer_id":1,"poll_media":{"text":"A"}},
                    {"answer_id":7,"poll_media":{"text":"B","emoji":{"name":"🔥"}}}
                ],
                "expiry":"2026-08-26T20:00:00+00:00",
                "allow_multiselect":false,
                "layout_type":1,
                "results":{
                    "is_finalized":false,
                    "answer_counts":[{"id":1,"count":4,"me_voted":true}]
                }
            }"#,
        )
        .expect("poll");

        assert_eq!(poll.layout_type, PollLayoutType::DEFAULT);
        assert_eq!(poll.answers[1].answer_id, 7);
        assert_eq!(poll.results.expect("results").answer_counts[0].count, 4);
    }

    #[test]
    fn poll_media_text_remains_future_optional() {
        let media: PollMedia =
            serde_json::from_str(r#"{"emoji":{"name":"👍"}}"#).expect("future poll media");
        assert!(media.text.is_none());
    }

    #[test]
    fn create_request_does_not_require_response_answer_ids() {
        let request = PollCreateRequest {
            question: PollMedia {
                text: Some("Question?".to_owned()),
                emoji: None,
            },
            answers: vec![PollCreateAnswer {
                poll_media: PollMedia {
                    text: Some("Answer".to_owned()),
                    emoji: None,
                },
            }],
            duration: Some(24),
            allow_multiselect: Some(false),
            layout_type: Some(PollLayoutType::DEFAULT),
        };

        let value = serde_json::to_value(request).expect("poll request");
        assert!(value["answers"][0].get("answer_id").is_none());
    }

    #[test]
    fn poll_layout_preserves_unknown_values() {
        let layout: PollLayoutType = serde_json::from_str("9").expect("layout");
        assert_eq!(layout, PollLayoutType(9));
    }
}
