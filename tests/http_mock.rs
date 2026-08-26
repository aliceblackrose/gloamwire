use std::time::Duration;

use gloamwire::{
    RestClient,
    model::{ChannelId, MessageId},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

const RATE_LIMITED: &str = include_str!("fixtures/http/rate_limited.json");
const CURRENT_USER: &str = include_str!("fixtures/http/current_user.json");

#[tokio::test]
async fn retries_a_discord_rate_limit_against_a_mock_server() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HTTP fixture server");
    let address = listener.local_addr().expect("HTTP fixture address");

    let server = tokio::spawn(async move {
        let mut requests = Vec::new();

        let (mut first, _) = listener.accept().await.expect("first HTTP request");
        requests.push(read_request(&mut first).await);
        write_response(
            &mut first,
            "429 Too Many Requests",
            RATE_LIMITED,
            &["Content-Type: application/json", "Retry-After: 0"],
        )
        .await;

        let (mut second, _) = listener.accept().await.expect("retried HTTP request");
        requests.push(read_request(&mut second).await);
        write_response(
            &mut second,
            "200 OK",
            CURRENT_USER,
            &["Content-Type: application/json"],
        )
        .await;

        requests
    });

    let client = RestClient::new("fixture-token")
        .expect("REST client")
        .with_base_url(format!("http://{address}"));
    let user = timeout(Duration::from_secs(5), client.get_current_user())
        .await
        .expect("REST request timed out")
        .expect("current user");

    assert_eq!(user.id.get(), 1);
    assert_eq!(user.username, "fixture-bot");

    let requests = server.await.expect("HTTP fixture server");
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| request.starts_with("GET /users/@me HTTP/1.1"))
    );
}

#[tokio::test]
async fn accepts_empty_204_responses_against_a_mock_server() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HTTP fixture server");
    let address = listener.local_addr().expect("HTTP fixture address");

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("delete request");
        let request = read_request(&mut stream).await;
        write_response(&mut stream, "204 No Content", "", &[]).await;
        request
    });

    let client = RestClient::new("fixture-token")
        .expect("REST client")
        .with_base_url(format!("http://{address}"));
    timeout(
        Duration::from_secs(5),
        client.delete_message(ChannelId::new(10), MessageId::new(20)),
    )
    .await
    .expect("delete request timed out")
    .expect("empty delete response");

    let request = server.await.expect("HTTP fixture server");
    assert!(request.starts_with("DELETE /channels/10/messages/20 HTTP/1.1"));
}

async fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];

    loop {
        let read = stream.read(&mut buffer).await.expect("read HTTP request");
        assert!(read > 0, "HTTP client closed before completing headers");
        request.extend_from_slice(&buffer[..read]);

        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8(request).expect("HTTP request is UTF-8")
}

async fn write_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    headers: &[&str],
) {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    response.push_str(body);

    stream
        .write_all(response.as_bytes())
        .await
        .expect("write HTTP response");
    stream.shutdown().await.expect("close HTTP fixture stream");
}
