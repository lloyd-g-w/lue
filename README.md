# Live Queue With Rust, Dioxus, and WebSockets

This repository contains a small full-stack queue system:

- `crates/server`: Axum WebSocket server with an in-memory queue store
- `crates/web`: Dioxus web frontend for admins and users
- `crates/shared`: Shared protocol and view models used by both sides

## What it does

- The first deployment prompts you to create the initial super admin account.
- The super admin can create `admin` and `user` email/password accounts from the dashboard.
  Admin Microsoft SSO requires an existing admin account, while user Microsoft SSO can create a
  local user account from the verified Microsoft profile.
- The super admin can enable or disable password and Microsoft sign-in separately for admins and
  users from Site management.
- An admin creates a queue with a queue name, any number of required fields, and an `allow guests` setting.
- A user opens the queue link, signs in if required, or joins as a guest if that queue allows it.
- The admin sees the live queue, can inspect individual entries, and can `claim`, `unclaim`, `resolve`, or `deny` them.
- The user sees live status updates and can leave the queue while the request is still active.

All state changes are pushed over WebSockets.

Accounts, login sessions, queues, and queue entries are saved to disk as JSON so reconnects can
resume after a backend restart.

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
If the persistent store has no super admin yet, `/` shows the initial setup form. After that,
the account is saved to the persistent store and future visits show the normal sign-in flow.

Persistent server data is stored at `data/store.json` by default. Set `DATA_PATH` to use a
different file:

```bash
DATA_PATH=/path/to/store.json cargo run -p server
```

## Microsoft SSO

Microsoft SSO is enabled when the backend has these environment variables. Local `cargo run -p server`
loads them from `.env` if that file exists:

```bash
MICROSOFT_CLIENT_ID=<application-client-id>
MICROSOFT_CLIENT_SECRET=<client-secret>
MICROSOFT_TENANT_ID=<tenant-id-or-common>
SERVER_PUBLIC_BASE_URL=http://127.0.0.1:3000
FRONTEND_BASE_URL=http://127.0.0.1:8080
```

Register this redirect URI in Microsoft Entra ID:

```text
${SERVER_PUBLIC_BASE_URL}/auth/microsoft/callback
```

For Docker Compose on `http://127.0.0.1:8081`, set `SERVER_PUBLIC_BASE_URL` and
`FRONTEND_BASE_URL` to `http://127.0.0.1:8081` and use
`http://127.0.0.1:8081/auth/microsoft/callback` as the redirect URI. Microsoft SSO does not
auto-provision accounts; create the account first in Lue, then users can sign in with Microsoft
using that same email address.

## Run with Docker Compose

Build and start the full app:

```bash
docker compose up --build -d
```

Open the app at:

```text
http://127.0.0.1:8081
```

Compose runs two containers:

- `server`: the Axum backend, listening inside Docker on `0.0.0.0:3000`
- `web`: nginx serving the Dioxus static build and proxying `/ws`, `/health`, and
  `/auth/microsoft/*` to `server`

Persistent data is stored in the named Docker volume `lue-data` at `/data/store.json` inside
the server container. To stop the app without deleting data:

```bash
docker compose down
```

To remove the persisted queue store as well:

```bash
docker compose down -v
```

## Routes

- `/` shows the admin sign-in page
- `/admin` shows the admin queue dashboard
- `/admin/queue/<queue-id>` shows the dashboard with a selected queue
- `/queue/<queue-id>` shows the user queue join page

## Notes

- Initial setup is only available while the persistent store has no super admin account.
- Passwords are stored as salted Argon2 hashes.
