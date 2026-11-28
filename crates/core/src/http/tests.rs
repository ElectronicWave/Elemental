use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Notify;

use super::{HttpClientConfig, RequestBuilderExt, SwappableClient};

async fn read_request_text(stream: &mut tokio::net::TcpStream) -> String {
    let mut buffer = vec![0u8; 4096];
    let mut data = Vec::new();
    loop {
        let read = stream.read(&mut buffer).await.expect("read request");
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        if data.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}

#[tokio::test]
async fn swap_routes_in_flight_requests_to_old_client_and_new_requests_to_new_client() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");
    let first_connected = Arc::new(Notify::new());
    let release_first = Arc::new(Notify::new());
    let first_connected_task = Arc::clone(&first_connected);
    let release_first_task = Arc::clone(&release_first);

    let server = tokio::spawn(async move {
        let (mut first_stream, _) = listener.accept().await.expect("accept first");
        let first_req = read_request_text(&mut first_stream).await;
        first_connected_task.notify_one();

        let (mut second_stream, _) = listener.accept().await.expect("accept second");
        let second_req = read_request_text(&mut second_stream).await;
        second_stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nnew")
            .await
            .expect("write second response");

        release_first_task.notified().await;
        first_stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nold")
            .await
            .expect("write first response");

        (first_req, second_req)
    });

    let swappable = SwappableClient::new(
        HttpClientConfig::new()
            .no_proxy(true)
            .user_agent("ElementalTest/1.0"),
    )
    .expect("build initial client");

    let old_client = swappable.client();
    let url = format!("http://{addr}/");
    let first_url = url.clone();
    let first_task = tokio::spawn(async move {
        old_client
            .get(first_url)
            .send()
            .await
            .expect("first request send")
            .text()
            .await
            .expect("first response text")
    });

    first_connected.notified().await;

    swappable
        .update(|cfg| {
            cfg.user_agent("ElementalTest/2.0").default_header(
                HeaderName::from_static("x-test-gen"),
                HeaderValue::from_static("two"),
            )
        })
        .expect("swap config");

    let second_body = swappable
        .client()
        .get(&url)
        .send()
        .await
        .expect("second request send")
        .text()
        .await
        .expect("second response text");
    assert_eq!(second_body, "new");

    release_first.notify_one();
    let first_body = first_task.await.expect("join first task");
    assert_eq!(first_body, "old");

    let (first_req, second_req) = server.await.expect("join server");
    let first_lower = first_req.to_ascii_lowercase();
    let second_lower = second_req.to_ascii_lowercase();
    assert!(first_lower.contains("user-agent: elementaltest/1.0"));
    assert!(!first_lower.contains("x-test-gen: two"));
    assert!(second_lower.contains("user-agent: elementaltest/2.0"));
    assert!(second_lower.contains("x-test-gen: two"));
}

#[test]
fn failed_swap_preserves_existing_config_and_client() {
    let swappable = SwappableClient::new(
        HttpClientConfig::new()
            .user_agent("ValidAgent/1.0")
            .connect_timeout(Some(Duration::from_secs(5))),
    )
    .expect("build initial client");

    let result = swappable.swap(HttpClientConfig::new().user_agent("Invalid\r\nHeader"));
    assert!(result.is_err());
    assert_eq!(
        swappable.config().user_agent_value(),
        Some("ValidAgent/1.0")
    );
    assert_eq!(
        swappable.config().connect_timeout_value(),
        Some(Duration::from_secs(5))
    );
}

#[tokio::test]
async fn no_redirect_client_does_not_follow_redirects() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind redirect listener");
    let addr = listener.local_addr().expect("redirect listener addr");

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept redirect request");
        let _ = read_request_text(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 302 Found\r\nLocation: /target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write redirect response");
    });

    let swappable = SwappableClient::new(HttpClientConfig::new().no_proxy(true))
        .expect("build swappable client");
    let response = swappable
        .no_redirect_client()
        .get(format!("http://{addr}/start"))
        .send()
        .await
        .expect("send request");

    assert_eq!(response.status(), reqwest::StatusCode::FOUND);
    server.await.expect("join redirect server");
}

#[tokio::test]
async fn scoped_headers_share_hot_swap_and_send_json_decodes() {
    #[derive(serde::Deserialize)]
    struct Payload {
        ok: bool,
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind scoped listener");
    let addr = listener.local_addr().expect("scoped listener addr");

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept scoped request");
        let request = read_request_text(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
            )
            .await
            .expect("write json response");
        request
    });

    let root =
        SwappableClient::new(HttpClientConfig::new().no_proxy(true)).expect("build root client");
    let scoped = root.clone().with_header(
        HeaderName::from_static("x-mirror-key"),
        HeaderValue::from_static("secret"),
    );

    // Hot-swapping on the root handle updates the scoped handle as well.
    root.swap(
        HttpClientConfig::new()
            .no_proxy(true)
            .user_agent("UpdatedAgent/2.0"),
    )
    .expect("swap root client");

    let payload: Payload = scoped
        .get(format!("http://{addr}/data.json"))
        .send_json()
        .await
        .expect("decode json response");
    assert!(payload.ok);
    assert!(root.scoped_headers().is_empty());

    let request = server.await.expect("join server");
    let lower = request.to_ascii_lowercase();
    assert!(lower.contains("x-mirror-key: secret"));
    assert!(lower.contains("user-agent: updatedagent/2.0"));
}
