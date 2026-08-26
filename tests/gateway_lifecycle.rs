use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use gloamwire::gateway::{GatewayConfig, GatewayConnection, GatewayEvent, GatewayIntents};
use serde_json::Value;
use tokio::{
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

const HELLO: &str = include_str!("fixtures/gateway/hello.json");
const READY: &str = include_str!("fixtures/gateway/ready.json");
const RECONNECT: &str = include_str!("fixtures/gateway/reconnect.json");
const RESUMED: &str = include_str!("fixtures/gateway/resumed.json");

#[tokio::test]
async fn reconnect_opcode_resumes_the_existing_gateway_session() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Gateway fixture server");
    let address = listener.local_addr().expect("Gateway fixture address");
    let gateway_url = format!("ws://{address}");
    let ready = READY.replace("__RESUME_URL__", &gateway_url);

    let server = tokio::spawn(async move {
        let (first_stream, _) = listener.accept().await.expect("first Gateway connection");
        let mut first = accept_async(first_stream).await.expect("first WebSocket");
        send_fixture(&mut first, HELLO).await;

        let identify = read_client_payload(&mut first).await;
        assert_eq!(identify["op"], 2);
        assert_eq!(identify["d"]["token"], "fixture-token");

        send_fixture(&mut first, &ready).await;
        send_fixture(&mut first, RECONNECT).await;
        drop(first);

        let (second_stream, _) = listener.accept().await.expect("resumed Gateway connection");
        let mut second = accept_async(second_stream)
            .await
            .expect("resumed WebSocket");
        send_fixture(&mut second, HELLO).await;

        let resume = read_client_payload(&mut second).await;
        assert_eq!(resume["op"], 6);
        assert_eq!(resume["d"]["token"], "fixture-token");
        assert_eq!(resume["d"]["session_id"], "fixture-session");
        assert_eq!(resume["d"]["seq"], 1);

        send_fixture(&mut second, RESUMED).await;
    });

    let config = GatewayConfig::new("fixture-token", GatewayIntents::empty()).with_url(gateway_url);
    let mut connection = timeout(Duration::from_secs(5), GatewayConnection::connect(config))
        .await
        .expect("Gateway connect timed out")
        .expect("Gateway connection");

    let ready = timeout(Duration::from_secs(5), connection.next_event())
        .await
        .expect("Ready timed out")
        .expect("Ready event");
    let GatewayEvent::Dispatch(ready) = ready else {
        panic!("expected READY dispatch");
    };
    assert_eq!(ready.name, "READY");
    assert_eq!(ready.sequence, 1);
    assert_eq!(
        connection.session().expect("captured session").session_id(),
        "fixture-session"
    );

    let reconnect = timeout(Duration::from_secs(5), connection.next_event())
        .await
        .expect("Reconnect timed out")
        .expect("Reconnect event");
    assert_eq!(reconnect, GatewayEvent::Reconnect);

    let resumed = timeout(Duration::from_secs(5), connection.next_event())
        .await
        .expect("Resumed timed out")
        .expect("Resumed event");
    let GatewayEvent::Dispatch(resumed) = resumed else {
        panic!("expected RESUMED dispatch");
    };
    assert_eq!(resumed.name, "RESUMED");
    assert_eq!(resumed.sequence, 2);
    assert_eq!(connection.sequence(), Some(2));

    server.await.expect("Gateway fixture server");
}

async fn send_fixture(socket: &mut WebSocketStream<TcpStream>, fixture: &str) {
    socket
        .send(Message::Text(fixture.trim().to_owned().into()))
        .await
        .expect("send Gateway fixture");
}

async fn read_client_payload(socket: &mut WebSocketStream<TcpStream>) -> Value {
    loop {
        let message = socket
            .next()
            .await
            .expect("client Gateway payload")
            .expect("valid client WebSocket payload");

        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str()).expect("JSON Gateway payload");
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .expect("reply to client ping"),
            Message::Binary(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {}
        }
    }
}
