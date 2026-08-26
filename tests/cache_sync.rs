#![cfg(feature = "cache")]

use gloamwire::{
    Cache,
    gateway::{DispatchEvent, TypedDispatchEvent},
    model::{ChannelId, Guild, GuildId, MessageId},
};
use serde_json::json;

#[test]
fn message_delete_clears_last_message_without_body_retention() {
    let guild: Guild = serde_json::from_value(json!({
        "id":"1",
        "name":"Gloamwire",
        "owner_id":"2",
        "channels":[{"id":"10","type":0,"name":"general"}]
    }))
    .expect("guild");
    let mut cache = Cache::default();
    cache.update(&TypedDispatchEvent::GuildCreate(guild));

    cache
        .update_dispatch(&DispatchEvent {
            name: "MESSAGE_CREATE".to_owned(),
            sequence: 1,
            data: json!({
                "id":"50",
                "channel_id":"10",
                "guild_id":"1",
                "author":{"id":"2","username":"user"},
                "content":"transient"
            }),
        })
        .expect("message create");

    assert!(cache.message(MessageId::new(50)).is_none());
    assert_eq!(
        cache
            .channel(ChannelId::new(10))
            .expect("cached channel")
            .last_message_id,
        Some(MessageId::new(50))
    );

    cache
        .update_dispatch(&DispatchEvent {
            name: "MESSAGE_DELETE".to_owned(),
            sequence: 2,
            data: json!({
                "id":"50",
                "channel_id":"10",
                "guild_id":"1"
            }),
        })
        .expect("message delete");

    assert_eq!(
        cache
            .channel(ChannelId::new(10))
            .expect("cached channel")
            .last_message_id,
        None
    );
    assert!(cache.guild(GuildId::new(1)).is_some());
}
