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

#[test]
fn partial_message_update_preserves_omitted_fields_and_clears_explicit_values() {
    let mut cache = Cache::new(gloamwire::CacheConfig::new().message_capacity(4));
    cache
        .update_dispatch(&DispatchEvent {
            name: "MESSAGE_CREATE".into(),
            sequence: 1,
            data: json!({
                "id": "50", "channel_id": "10",
                "author": {"id": "2", "username": "user"},
                "content": "old", "edited_timestamp": "previous", "pinned": true
            }),
        })
        .expect("message create");
    cache
        .update_dispatch(&DispatchEvent {
            name: "MESSAGE_UPDATE".into(),
            sequence: 2,
            data: json!({
                "id": "50", "channel_id": "10",
                "content": "new", "edited_timestamp": null
            }),
        })
        .expect("message update");
    let message = cache.message(MessageId::new(50)).expect("cached message");
    assert_eq!(message.content, "new");
    assert!(message.pinned);
    assert!(message.edited_timestamp.is_none());
}

#[test]
fn ready_from_other_shard_preserves_cached_guild() {
    use gloamwire::gateway::{GatewayEvent, ShardCount, ShardEvent, ShardId};
    let mut cache = Cache::default();
    let count = ShardCount::new(2).expect("shard count");
    cache
        .update_shard_event(
            &ShardEvent {
                shard_id: ShardId::new(0),
                event: GatewayEvent::Dispatch(DispatchEvent {
                    name: "GUILD_CREATE".into(),
                    sequence: 1,
                    data: json!({"id": "1", "name": "guild", "owner_id": "2"}),
                }),
            },
            count,
        )
        .expect("guild create");
    cache
        .update_shard_event(
            &ShardEvent {
                shard_id: ShardId::new(1),
                event: GatewayEvent::Dispatch(DispatchEvent {
                    name: "READY".into(),
                    sequence: 1,
                    data: json!({
                        "v": 10, "user": {"id": "2", "username": "bot"},
                        "guilds": [], "session_id": "session",
                        "resume_gateway_url": "wss://gateway.discord.gg",
                        "application": {"id": "2", "flags": 0}
                    }),
                }),
            },
            count,
        )
        .expect("ready");
    assert!(cache.guild(GuildId::new(1)).is_some());
}
