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
type Clients = Arc<Mutex<HashMap<String, Tx>>>;

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
    let state: Clients = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    println!("[{}] [INFO] Server started", timestamp());

    while let Ok((stream, _)) = listener.accept().await {
        let state_clone = state.clone();

        tokio::spawn(async move {
            let mut extracted_user_id: Option<String> = None;

            let ws_stream = accept_hdr_async(stream, |req: &Request, resp: Response| {
                if let Some(query) = req.uri().query() {
                    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
                        if key == "userId" {
                            extracted_user_id = Some(value.into_owned());
                        }
                    }
                }
                Ok(resp)
            }).await.unwrap();

            let user_id = match extracted_user_id {
                Some(id) => id,
                None => {
                    println!("[{}] [ERROR] No userId provided", timestamp());
                    return;
                }
            };

            println!("[{}] [INFO] User connected: {}", timestamp(), user_id);

            let (mut write, mut read) = ws_stream.split();
            let (tx, mut rx) = mpsc::unbounded_channel();

            state_clone.lock().unwrap().insert(user_id.clone(), tx);

            // Writer task
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let _ = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                        .await;
                }
            });

            // Reader loop
            while let Some(Ok(msg)) = read.next().await {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    println!("[{}] [INFO] Received from {}: {}", timestamp(), user_id, text);

                    if let Ok(parsed) = serde_json::from_str::<Message>(&text) {
                        handle_message(parsed, &user_id, &state_clone);
                    }
                }
            }

            state_clone.lock().unwrap().remove(&user_id);
            println!("[{}] [INFO] User disconnected: {}", timestamp(), user_id);
        });
    }
}
fn handle_message(msg: Message, sender: &str, clients: &Clients) {
    match msg.msg_type.as_str() {
        "call-start" => handle_call_start(msg, sender, clients),
        "call-accept" => handle_call_accept(msg, sender, clients),
        "call-reject" => handle_call_reject(msg, sender, clients),
        _ => println!("[{}] [WARN] Unknown message type", timestamp()),
    }
}
fn handle_call_start(mut msg: Message, sender: &str, clients: &Clients) {
    println!(
        "[{}] [CALL_START] {} is calling {:?}",
        timestamp(),
        sender,
        msg.to
    );

    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}
fn handle_call_accept(mut msg: Message, sender: &str, clients: &Clients) {
    println!(
        "[{}] [CALL_ACCEPT] {} accepted call from {:?}",
        timestamp(),
        sender,
        msg.to
    );

    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}
fn handle_call_reject(mut msg: Message, sender: &str, clients: &Clients) {
    println!(
        "[{}] [CALL_REJECT] {} rejected call from {:?}",
        timestamp(),
        sender,
        msg.to
    );

    msg.from = Some(sender.to_string());
    forward_to_user(msg, clients);
}
fn forward_to_user(msg: Message, clients: &Clients) {
    if let Some(target) = &msg.to {
        let clients_map = clients.lock().unwrap();

        if let Some(tx) = clients_map.get(target) {
            let serialized = serde_json::to_string(&msg).unwrap();
            let _ = tx.send(serialized);

            println!(
                "[{}] [ROUTE] Message forwarded to {}",
                timestamp(),
                target
            );
        } else {
            println!(
                "[{}] [WARN] Target user {} not connected",
                timestamp(),
                target
            );
        }
    }
}