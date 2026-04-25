# Live Queue With Rust, Dioxus, and WebSockets

This repository contains a small full-stack queue system:

- `crates/server`: Axum WebSocket server with an in-memory queue store
- `crates/web`: Dioxus web frontend for admins and users
- `crates/shared`: Shared protocol and view models used by both sides

## What it does

- A super admin account is bootstrapped from `.env`.
- The super admin can create `admin` and `user` email/password accounts from the dashboard.
- An admin creates a queue with a queue name, any number of required fields, and an `allow guests` setting.
- A user opens the queue link, signs in if required, or joins as a guest if that queue allows it.
- The admin sees the live queue, can inspect individual entries, and can `claim`, `unclaim`, `resolve`, or `deny` them.
- The user sees live status updates and can leave the queue while the request is still active.

All state changes are pushed over WebSockets.

## Run locally

1. Set the super admin credentials in `.env`:

```bash
SUPER_ADMIN_NAME=Super Admin
SUPER_ADMIN_EMAIL=superadmin@example.com
SUPER_ADMIN_PASSWORD=change-me
```

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

- `/` shows the admin sign-in page
- `/admin` shows the admin queue dashboard
- `/admin/queue/<queue-id>` shows the dashboard with a selected queue
- `/queue/<queue-id>` shows the user queue join page

## Notes

- Queue data is stored in memory only.
- Account data is stored in memory only; restarting the server clears created admin and user accounts and reloads only the `.env` super admin.
- Passwords are stored in plaintext for this prototype. For anything real, move to persistent storage and hashed passwords.
