# Live Queue With Rust, Dioxus, and WebSockets

This repository contains a small full-stack queue system:

- `crates/server`: Axum WebSocket server with an in-memory queue store
- `crates/web`: Dioxus web frontend for admins and users
- `crates/shared`: Shared protocol and view models used by both sides

## What it does

- An admin creates a queue with a queue name and any number of required fields.
- A user opens the queue link, fills in the configured fields, and joins.
- The admin sees the live queue, can inspect individual entries, and can `claim`, `resolve`, or `deny` them.
- The user sees live status updates and can leave the queue while the request is still active.

All state changes are pushed over WebSockets.

## Run locally

1. Start the backend:

```bash
cargo run -p server
```

2. In another terminal, run the Dioxus frontend:

```bash
dx serve --package web
```

The frontend expects the backend WebSocket endpoint at `ws://127.0.0.1:3000/ws`.

## Routes

- `/` creates a new queue as an admin
- `/admin/<admin-token>` shows the admin queue dashboard
- `/queue/<queue-id>` shows the user queue join page

## Notes

- Queue data is stored in memory only.
- There is no persistent auth layer yet; admin access is protected by an opaque admin token embedded in the admin URL.
- This is a solid prototype foundation for later additions such as persistence, auth, metrics, and audit logs.

