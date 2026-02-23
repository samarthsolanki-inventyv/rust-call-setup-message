# Rust WebSocket Call Setup Signaling Server

A simple real-time WebSocket **call setup signaling server** built with Rust and Tokio.

This project demonstrates how to:

- Accept multiple WebSocket clients
- Route call setup messages between users
- Manage connected users in memory
- Build a minimal signaling layer for call initiation

---

##  What This Project Does

This project implements a **basic call setup signaling system**.

It supports the following call setup messages:

-  `call-start`  → User initiates a call
-  `call-accept` → User accepts a call
-  `call-reject` → User rejects a call

The server acts as a message router between connected users.

> This is a signaling server only.  
> It does NOT transmit audio/video media.

---

## Architecture Overview

Client A → Server → Client B
Each connected user:
- Has their own WebSocket connection
- Is handled by a dedicated async Tokio task
- Has a mailbox (mpsc channel) for receiving outgoing messages

The server stores active users in memory and forwards messages based on `userId`.

---

## Tech Stack

- Tokio (async runtime)
- tokio-tungstenite (WebSocket support)
- Serde (JSON serialization)
- mpsc channels (async message passing)

---

##  Project Structure

```
rust-signaling/
│
├── src/
│   └── main.rs        # WebSocket signaling server
│
├── static/
│   └── index.html     # Frontend demo UI
│
├── Cargo.toml
├── Cargo.lock
└── .gitignore
```
---

## How It Works

### 1️ Client Connection

Users connect to:

ws://127.0.0.1:8080?userId=YOUR_ID


The server extracts `userId` from the query string and stores:

userId → sender channel


---

### 2️ Message Format

Messages are JSON objects:

```json
{
  "type": "call-start",
  "to": "targetUser"
}
The server automatically adds:

{
  "from": "senderUser"
}
```
---
### 3 Message Routing
When a message is received:

The server parses the JSON

Matches the message type

Finds the target user

Sends the message through that user's channel

The user's task writes it to their WebSocket

---
## Running the Project

### 1️-: Start the server

```bash
cargo run


Server runs on:

127.0.0.1:8080
```
### 2️-: Open the frontend
```
Open:

static/index.html


In two different browser tabs.

How to Test

Open Tab 1 → Connect as alice

Open Tab 2 → Connect as bob

From Alice → Call Bob

Bob will see incoming call

Accept or Reject
```

---
