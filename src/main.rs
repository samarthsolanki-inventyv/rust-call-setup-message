use tokio::net::TcpListener;
use tokio_tungstenite::{accept_hdr_async, tungstenite::handshake::server::{Request, Response}};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use url::form_urlencoded;
use std::time::{SystemTime, UNIX_EPOCH};

type Tx = mpsc::UnboundedSender<String>; 
// Sender type for sending messages to a user

type Clients = Arc<Mutex<HashMap<String, Tx>>>; 
// Shared state: userId -> sender channel

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Message {
    #[serde(rename = "type")]
    msg_type: String,
    to: Option<String>,
    from: Option<String>,
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[tokio::main]
async fn main() {
    // Shared storage for connected users
    let state: Clients = Arc::new(Mutex::new(HashMap::new()));

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("[{}] [INFO] Server started", timestamp());

    // Accept incoming connections
    while let Ok((stream, _)) = listener.accept().await {
        let state_clone = state.clone();

        // Create one async task per connected user
        tokio::spawn(async move {
            let mut extracted_user_id: Option<String> = None;

            // Upgrade TCP connection to WebSocket
            let ws_stream = accept_hdr_async(stream, |req: &Request, resp: Response| {
                // Extract userId from query string
                if let Some(query) = req.uri().query() {
                    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
                        if key == "userId" {
                            extracted_user_id = Some(value.into_owned());
                        }
                    }
                }
                Ok(resp)
            }).await.unwrap();

            // Ensure userId exists
            let user_id = match extracted_user_id {
                Some(id) => id,
                None => {
                    println!("[{}] [ERROR] No userId provided", timestamp());
                    return;
                }
            };

            println!("[{}] [INFO] User connected: {}", timestamp(), user_id);

            // Split WebSocket into write and read parts
            let (mut write, mut read) = ws_stream.split();

            // Create mailbox (channel) for this user
            let (tx, mut rx) = mpsc::unbounded_channel();

            // Store user in shared state
            state_clone.lock().unwrap().insert(user_id.clone(), tx);

            // Task responsible for sending messages to this user
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let _ = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                        .await;
                }
            });

            // Listen for incoming messages from this user
            while let Some(Ok(msg)) = read.next().await {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    println!("[{}] [INFO] Received from {}: {}", timestamp(), user_id, text);

                    if let Ok(parsed) = serde_json::from_str::<Message>(&text) {
                        handle_message(parsed, &user_id, &state_clone);
                    }
                }
            }

            // Remove user when disconnected
            state_clone.lock().unwrap().remove(&user_id);
            println!("[{}] [INFO] User disconnected: {}", timestamp(), user_id);
        });
    }
}

// Decide what to do based on message type
fn handle_message(msg: Message, sender: &str, clients: &Clients) {
    match msg.msg_type.as_str() {
        "call-start" => handle_call_start(msg, sender, clients),
        "call-accept" => handle_call_accept(msg, sender, clients),
        "call-reject" => handle_call_reject(msg, sender, clients),
        _ => println!("[{}] [WARN] Unknown message type", timestamp()),
    }
}

// Forward call start to target user
fn handle_call_start(mut msg: Message, sender: &str, clients: &Clients) {
    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}

// Forward call accept to target user
fn handle_call_accept(mut msg: Message, sender: &str, clients: &Clients) {
    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}

// Forward call reject to target user
fn handle_call_reject(mut msg: Message, sender: &str, clients: &Clients) {
    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}

// Send message to the target user's mailbox
fn forward_to_user(msg: Message, clients: &Clients) {
    if let Some(target) = &msg.to {
        let clients_map = clients.lock().unwrap();

        if let Some(tx) = clients_map.get(target) {
            let serialized = serde_json::to_string(&msg).unwrap();
            let _ = tx.send(serialized);
        }
    }
}
