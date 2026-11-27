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

#[test]
fn member_update_preserves_omitted_timeout_and_clears_explicit_null() {
    use gloamwire::model::UserId;
    let mut cache = Cache::default();
    let dispatch = |name: &str, data: serde_json::Value| DispatchEvent {
        name: name.into(),
        sequence: 1,
        data,
    };
    cache.update_dispatch(&dispatch("GUILD_CREATE", json!({
        "id":"1","name":"guild","owner_id":"2","member_count":1,
        "members":[{"user":{"id":"2","username":"user"},"communication_disabled_until":"future"}]
    }))).expect("guild");
    cache
        .update_dispatch(&dispatch(
            "GUILD_MEMBER_UPDATE",
            json!({
                "guild_id":"1","user":{"id":"2","username":"user"},"nick":"new"
            }),
        ))
        .expect("update");
    assert_eq!(
        cache
            .member(GuildId::new(1), UserId::new(2))
            .unwrap()
            .communication_disabled_until
            .as_deref(),
        Some("future")
    );
    cache.update_dispatch(&dispatch("GUILD_MEMBER_UPDATE", json!({
        "guild_id":"1","user":{"id":"2","username":"user"},"communication_disabled_until":null
    }))).expect("clear");
    assert!(
        cache
            .member(GuildId::new(1), UserId::new(2))
            .unwrap()
            .communication_disabled_until
            .is_none()
    );
}

#[test]
fn unavailable_guild_and_initial_presence_are_supported() {
    use gloamwire::model::UserId;
    let mut cache = Cache::default();
    cache
        .update_dispatch(&DispatchEvent {
            name: "GUILD_CREATE".into(),
            sequence: 1,
            data: json!({"id":"1","unavailable":true}),
        })
        .expect("unavailable");
    assert!(cache.is_guild_unavailable(GuildId::new(1)));
    cache.update_dispatch(&DispatchEvent { name:"GUILD_CREATE".into(), sequence:2,
        data:json!({"id":"1","name":"guild","owner_id":"2","presences":[{"user":{"id":"2"},"status":"online"}]}) }).expect("available");
    assert!(cache.presence(GuildId::new(1), UserId::new(2)).is_some());
}

#[test]
fn presence_updates_keep_embedded_guild_snapshot_consistent() {
    use gloamwire::model::UserId;
    let mut cache = Cache::default();
    let events = [
        (
            "GUILD_CREATE",
            json!({"id":"1","name":"guild","owner_id":"2","presences":[{"user":{"id":"2"},"status":"online"}]}),
        ),
        (
            "PRESENCE_UPDATE",
            json!({"guild_id":"1","user":{"id":"2"},"status":"idle"}),
        ),
    ];
    for (index, (name, data)) in events.into_iter().enumerate() {
        cache
            .update_dispatch(&DispatchEvent {
                name: name.into(),
                sequence: index as u64 + 1,
                data,
            })
            .expect("dispatch");
    }
    let guild = cache.guild(GuildId::new(1)).unwrap();
    assert_eq!(guild.presences.len(), 1);
    assert_eq!(guild.presences[0]["status"], "idle");
    assert_eq!(
        cache
            .presence(GuildId::new(1), UserId::new(2))
            .unwrap()
            .status
            .0,
        "idle"
    );
}

#[test]
fn channel_removal_cascades_to_threads_and_guild_reseed_discards_stale_messages() {
    let mut cache = Cache::new(gloamwire::CacheConfig::new().message_capacity(8));
    let events = [
        (
            "GUILD_CREATE",
            json!({"id":"1","name":"guild","owner_id":"2","channels":[{"id":"10","type":0},{"id":"20","type":0}],"threads":[{"id":"11","type":11,"parent_id":"10"}]}),
        ),
        (
            "MESSAGE_CREATE",
            json!({"id":"50","guild_id":"1","channel_id":"20","content":"cached message","author":{"id":"2","username":"user"}}),
        ),
        ("CHANNEL_DELETE", json!({"id":"10","type":0,"guild_id":"1"})),
        (
            "GUILD_CREATE",
            json!({"id":"1","name":"guild","owner_id":"2","channels":[]}),
        ),
    ];
    for (i, (name, data)) in events.into_iter().enumerate() {
        cache
            .update_dispatch(&DispatchEvent {
                name: name.into(),
                sequence: i as u64 + 1,
                data,
            })
            .unwrap();
        if i == 0 {
            assert!(cache.channel(ChannelId::new(11)).is_some());
        }
        if i == 1 {
            assert!(cache.message(MessageId::new(50)).is_some());
        }
        if i == 2 {
            assert!(cache.channel(ChannelId::new(11)).is_none());
            assert!(cache.message(MessageId::new(50)).is_some());
        }
    }
    assert!(cache.message(MessageId::new(50)).is_none());
}

#[test]
fn tombstone_eviction_invalidates_exact_member_count() {
    let mut cache = Cache::new(gloamwire::CacheConfig::new().membership_tombstone_capacity(0));
    cache.update_dispatch(&DispatchEvent {name:"GUILD_CREATE".into(),sequence:1,data:json!({"id":"1","name":"guild","owner_id":"2","member_count":1,"members":[{"user":{"id":"2","username":"user"}}]})}).unwrap();
    cache
        .update_dispatch(&DispatchEvent {
            name: "GUILD_MEMBER_REMOVE".into(),
            sequence: 2,
            data: json!({"guild_id":"1","user":{"id":"2","username":"user"}}),
        })
        .unwrap();
    assert!(cache.guild(GuildId::new(1)).unwrap().member_count.is_none());
}

#[test]
fn guild_delete_removes_messages_without_explicit_guild_id() {
    let mut cache = Cache::new(gloamwire::CacheConfig::new().message_capacity(8));
    let events = [
        (
            "GUILD_CREATE",
            json!({"id":"1","name":"guild","owner_id":"2","channels":[{"id":"10","type":0}]}),
        ),
        (
            "MESSAGE_CREATE",
            json!({"id":"50","channel_id":"10","content":"cached message","author":{"id":"2","username":"user"}}),
        ),
        ("GUILD_DELETE", json!({"id":"1","unavailable":false})),
    ];
    for (i, (name, data)) in events.into_iter().enumerate() {
        cache
            .update_dispatch(&DispatchEvent {
                name: name.into(),
                sequence: i as u64 + 1,
                data,
            })
            .unwrap();
        if i == 1 {
            assert!(cache.message(MessageId::new(50)).is_some());
        }
    }
    assert!(cache.message(MessageId::new(50)).is_none());
}

#[test]
fn shard_zero_ready_preserves_messages_in_other_shard_channels() {
    use gloamwire::gateway::{GatewayEvent, ShardCount, ShardEvent, ShardId};
    let mut cache = Cache::new(gloamwire::CacheConfig::new().message_capacity(4));
    let count = ShardCount::new(2).unwrap();
    let events = [
        (
            1,
            "GUILD_CREATE",
            json!({"id":"4194304","name":"guild","owner_id":"2","channels":[{"id":"10","type":0}]}),
        ),
        (
            1,
            "MESSAGE_CREATE",
            json!({"id":"50","channel_id":"10","author":{"id":"2","username":"user"},"content":"cached"}),
        ),
        (
            0,
            "READY",
            json!({"v":10,"user":{"id":"2","username":"bot"},"guilds":[],"session_id":"session","resume_gateway_url":"wss://gateway.discord.gg","application":{"id":"2","flags":0}}),
        ),
    ];
    for (i, (shard, name, data)) in events.into_iter().enumerate() {
        cache
            .update_shard_event(
                &ShardEvent {
                    shard_id: ShardId::new(shard),
                    event: GatewayEvent::Dispatch(DispatchEvent {
                        name: name.into(),
                        sequence: i as u64 + 1,
                        data,
                    }),
                },
                count,
            )
            .unwrap();
    }
    assert!(cache.message(MessageId::new(50)).is_some());
}

#[test]
fn presence_null_clears_avatar_but_omitted_name_is_preserved() {
    use gloamwire::model::UserId;
    let mut cache = Cache::default();
    let events = [
        (
            "USER_UPDATE",
            json!({"id":"2","username":"user","avatar":"hash","global_name":"name"}),
        ),
        (
            "PRESENCE_UPDATE",
            json!({"guild_id":"1","user":{"id":"2","avatar":null},"status":"online"}),
        ),
    ];
    for (i, (name, data)) in events.into_iter().enumerate() {
        cache
            .update_dispatch(&DispatchEvent {
                name: name.into(),
                sequence: i as u64 + 1,
                data,
            })
            .unwrap();
    }
    let user = cache.user(UserId::new(2)).unwrap();
    assert!(user.avatar.is_none());
    assert_eq!(user.global_name.as_deref(), Some("name"));
}

#[test]
fn deleted_voice_channel_removes_normalized_and_embedded_states() {
    use gloamwire::model::UserId;
    let mut cache = Cache::default();
    let events = [
        (
            "GUILD_CREATE",
            json!({"id":"1","name":"guild","owner_id":"2","channels":[{"id":"10","type":2}]}),
        ),
        (
            "VOICE_STATE_UPDATE",
            json!({"guild_id":"1","channel_id":"10","user_id":"2","session_id":"session","deaf":false,"mute":false,"self_deaf":false,"self_mute":false,"suppress":false}),
        ),
        ("CHANNEL_DELETE", json!({"id":"10","guild_id":"1","type":2})),
    ];
    for (i, (name, data)) in events.into_iter().enumerate() {
        cache
            .update_dispatch(&DispatchEvent {
                name: name.into(),
                sequence: i as u64 + 1,
                data,
            })
            .unwrap();
        if i == 1 {
            assert!(cache.voice_state(GuildId::new(1), UserId::new(2)).is_some());
        }
    }
    assert!(cache.voice_state(GuildId::new(1), UserId::new(2)).is_none());
    assert!(
        cache
            .guild(GuildId::new(1))
            .unwrap()
            .voice_states
            .is_empty()
    );
}
