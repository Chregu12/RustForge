//! Realtime chat: a shipped, runnable RustForge broadcasting/WebSocket example.
//!
//! This mirrors the proven end-to-end shape from the `broadcast_ws_transport`
//! sandbox probe, but as a real workspace example a reader can run:
//!
//! ```text
//! cargo run -p realtime-chat
//! ```
//!
//! It constructs a real [`MemoryBroadcaster`], mounts the real
//! [`websocket_router`] (axum 0.8) into an axum [`Router`], and serves it on
//! `127.0.0.1:3030`. Clients connect to `ws://127.0.0.1:3030/ws`, send a
//! `{"type":"subscribe","channel":"room-1"}` frame, and receive every event
//! broadcast to that channel.
//!
//! A background task periodically broadcasts a "chat message" to `room-1` from a
//! code path entirely independent of the WebSocket handler, so you can watch
//! events flow over the wire with any WebSocket client, e.g.:
//!
//! ```text
//! websocat ws://127.0.0.1:3030/ws
//! {"type":"subscribe","channel":"room-1"}
//! ```
//!
//! The `#[tokio::test]` at the bottom is the real proof: it binds an ephemeral
//! port, connects three real `tokio-tungstenite` clients, subscribes two of them
//! to `room-1` and one to `room-2`, fires a real broadcast, and asserts the two
//! `room-1` subscribers receive the event frame while the `room-2` client does
//! not.

use axum::Router;
use rf_broadcast::{websocket_router, Broadcaster, Channel, MemoryBroadcaster, SimpleEvent};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Build the app: the real WebSocket router mounted into an axum Router.
///
/// Returns the router plus the server-side broadcaster clone that application
/// code uses to broadcast events (independent of any connected socket).
fn build_app() -> (Router, Arc<MemoryBroadcaster>) {
    let broadcaster = Arc::new(MemoryBroadcaster::new());
    let app = Router::new().merge(websocket_router(Arc::clone(&broadcaster)));
    (app, broadcaster)
}

#[tokio::main]
async fn main() {
    let (app, broadcaster) = build_app();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3030")
        .await
        .expect("bind 127.0.0.1:3030");
    let addr = listener.local_addr().expect("local_addr");
    println!("realtime-chat listening on ws://{addr}/ws");
    println!("Subscribe with a WebSocket client by sending:");
    println!(r#"  {{"type":"subscribe","channel":"room-1"}}"#);
    println!("The server broadcasts a message to room-1 every 2 seconds.");

    // Independent producer: broadcast a chat message to room-1 on a timer, from a
    // code path that never touches the WebSocket handler.
    let producer = Arc::clone(&broadcaster);
    tokio::spawn(async move {
        let room = Channel::public("room-1");
        let mut n = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            n += 1;
            let event = SimpleEvent::new(
                "message.posted",
                json!({ "id": n, "text": format!("hello from the server #{n}") }),
                vec![room.clone()],
            );
            let _ = producer.broadcast(&room, &event).await;
        }
    });

    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::Message as TMsg;
    use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

    type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

    /// Receive the next text frame, with a timeout so the test never hangs.
    async fn recv_text(ws: &mut Ws) -> Option<String> {
        match tokio::time::timeout(Duration::from_secs(3), ws.next()).await {
            Ok(Some(Ok(TMsg::Text(t)))) => Some(t.as_str().to_string()),
            _ => None,
        }
    }

    /// Assert that no frame arrives within a short window.
    async fn expect_silence(ws: &mut Ws) -> bool {
        matches!(
            tokio::time::timeout(Duration::from_millis(400), ws.next()).await,
            Err(_) // timed out => nothing arrived => good
        )
    }

    /// Poll the broadcaster until `channel` has `want` subscribers (or time out).
    async fn wait_for_subs(b: &MemoryBroadcaster, channel: &Channel, want: usize) -> usize {
        for _ in 0..60 {
            let n = b.connections(channel).await.map(|c| c.len()).unwrap_or(0);
            if n >= want {
                return n;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        b.connections(channel).await.map(|c| c.len()).unwrap_or(0)
    }

    /// End-to-end: real WebSocket clients over a real TCP socket, a real
    /// broadcast, real received frames. No in-memory shortcut — every event
    /// travels through the actual `websocket_router` handler.
    #[tokio::test]
    async fn broadcast_reaches_subscribers_over_the_wire() {
        let (app, broadcaster) = build_app();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let url = format!("ws://{addr}/ws");

        // Three real WebSocket clients.
        let (mut c1, _) = connect_async(&url).await.expect("client 1 connect");
        let (mut c2, _) = connect_async(&url).await.expect("client 2 connect");
        let (mut c3, _) = connect_async(&url).await.expect("client 3 connect");

        let room1 = Channel::public("room-1");
        let room2 = Channel::public("room-2");

        // c1 and c2 subscribe to room-1; c3 subscribes to room-2.
        let sub1 = json!({"type":"subscribe","channel":"room-1"}).to_string();
        let sub2 = json!({"type":"subscribe","channel":"room-2"}).to_string();
        c1.send(TMsg::Text(sub1.clone().into())).await.unwrap();
        c2.send(TMsg::Text(sub1.into())).await.unwrap();
        c3.send(TMsg::Text(sub2.into())).await.unwrap();

        // Wait until the handler registered both room-1 subscriptions server-side.
        let n1 = wait_for_subs(&broadcaster, &room1, 2).await;
        assert_eq!(n1, 2, "handler must register 2 room-1 subscriptions");
        let _ = wait_for_subs(&broadcaster, &room2, 1).await;

        // Broadcast from an independent code path.
        let event = SimpleEvent::new(
            "message.posted",
            json!({"id": 7, "text": "hello over the wire"}),
            vec![room1.clone()],
        );
        broadcaster
            .broadcast(&room1, &event)
            .await
            .expect("broadcast room-1");

        // Both room-1 clients must RECEIVE the event frame over the socket.
        let r1 = recv_text(&mut c1).await;
        let r2 = recv_text(&mut c2).await;

        let payload_ok = |t: &Option<String>| {
            t.as_ref()
                .map(|s| {
                    let v: serde_json::Value =
                        serde_json::from_str(s).unwrap_or(json!(null));
                    v["type"] == "event"
                        && v["channel"] == "room-1"
                        && v["event"] == "message.posted"
                        && v["data"]["text"] == "hello over the wire"
                        && v["data"]["id"] == 7
                })
                .unwrap_or(false)
        };

        assert!(payload_ok(&r1), "c1 must receive the event frame, got {r1:?}");
        assert!(payload_ok(&r2), "c2 must receive the same broadcast, got {r2:?}");

        // c3 (different channel) must receive NOTHING.
        assert!(
            expect_silence(&mut c3).await,
            "non-subscribed client (room-2) must not receive a room-1 broadcast"
        );

        // Disconnect c1; the handler must drop its subscription.
        c1.close(None).await.ok();
        drop(c1);
        let mut after = 99;
        for _ in 0..60 {
            after = broadcaster.connections(&room1).await.unwrap().len();
            if after == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(after, 1, "disconnect must drop c1's subscription (2 -> 1)");

        // Broadcast again; only c2 should receive it now.
        let event2 = SimpleEvent::new(
            "message.posted",
            json!({"id": 8, "text": "second round"}),
            vec![room1.clone()],
        );
        broadcaster
            .broadcast(&room1, &event2)
            .await
            .expect("broadcast 2");

        let r2b = recv_text(&mut c2).await;
        assert!(
            r2b.as_ref().map(|s| s.contains("second round")).unwrap_or(false),
            "remaining subscriber c2 must still receive after the other disconnected, got {r2b:?}"
        );
    }
}
