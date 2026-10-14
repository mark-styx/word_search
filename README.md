# Word Search LAN Party

Real-time multiplayer word search built for local-network play, quick player switching, and phone-friendly sessions.

## What It Does

- Server-backed player picker with no password required for normal play
- Solo and multiplayer games with live lobby updates and in-game WebSocket sync
- End-of-board review with score summary and word ownership
- Homepage leaderboard preview plus full leaderboard views for points, wins, points per game, and win rate
- Haptic feedback on supported devices when you claim a word
- Built-in puzzle themes: Animals, Sports, Food, Disney, Cars, Desert, Mountains, Ocean, Weather, Garden, Farm, Shiny Things, Toys, Space, Science, Geography, Technology, Music, History, Nature
- Custom word-bank generation with reuse/coverage tracking
- Multiple visual themes and persisted highlight colors across devices
- LAN discovery via mDNS plus direct host/IP play

## Stack

- Backend: Rust, Axum, SQLite, JWT auth, WebSockets, mDNS
- Frontend: Next.js App Router, React, TypeScript, Tailwind CSS
- Testing: Rust integration tests and Playwright end-to-end tests

## Repo Layout

```text
backend/   Rust API, WebSocket game state, persistence, word banks
frontend/  Next.js client, auth picker, lobby, board UI, e2e tests
data/      Local SQLite database in development
```

## LAN Container Checklist

- [x] Frontend and backend use normal bridge networking instead of host networking
- [x] Stable LAN-facing ports are published: frontend `3089`, backend `3099`
- [x] Browser API calls resolve from the page host, so remote devices do not hit `localhost`
- [x] Frontend server-side requests use an internal container URL instead of `127.0.0.1`
- [x] SQLite persists in a named Docker volume
- [x] Native development stays separate for faster iteration

## LAN Play Via Docker

This is the primary run path for hosting the game on your local network.

```bash
docker compose up --build -d
```

Open:

```text
http://localhost:3089
http://<host-ip>:3089
```

The container stack uses:

- frontend on `3089`
- backend API on `3099`
- persistent SQLite data in the `backend-data` Docker volume

Useful commands:

```bash
docker compose logs -f
docker compose down
```

This setup is for LAN use only. It does not include public-internet hardening such as TLS, a reverse proxy, or rate limiting.

## Blue/Green Preview Flow

The repo now includes a blue/green-lite deployment path for a stable live URL plus a stable candidate URL.

Intended hostnames:

```text
wordsearch.home.arpa
wordsearch-preview.home.arpa
```

Recommended layout:

- `blue` stack: frontend `3089`, backend `3099`
- `green` stack: frontend `4189`, backend `4199`
- Caddy routes the live hostname to the active stack and the preview hostname to the inactive stack
- preview data is a deliberate clone of live data, not an accidental side effect of random compose project names
- if `port-authority` is installed and the blue/green assignments exist, the scripts use those assigned ports before falling back to the defaults above

Key files:

```text
docker-compose.bluegreen.yml
deploy/Caddyfile.home-arpa.example
scripts/bluegreen/status.sh
scripts/bluegreen/deploy-preview.sh
scripts/bluegreen/sync-preview-from-live.sh
scripts/bluegreen/promote.sh
scripts/bluegreen/rollback.sh
```
scripts/backup-db.sh

AdGuard DNS rewrites:

```text
wordsearch.home.arpa          -> 192.168.1.60
wordsearch-preview.home.arpa  -> 192.168.1.60
```

Optional `port-authority` assignments used by the scripts:

```text
word-search-blue-frontend
word-search-blue-backend
word-search-green-frontend
word-search-green-backend
```

The legacy preview compose file also now pins its volume name explicitly, so changing the Compose project name no longer silently creates a fresh empty preview DB.

### Preview Deploy

Deploy the inactive candidate stack:

```bash
./scripts/bluegreen/deploy-preview.sh
```

By default this deploys the current preview stack from `.bluegreen/state.env`.

### Sync Preview Data From Live

Clone the current live SQLite DB into the preview stack:

```bash
./scripts/bluegreen/sync-preview-from-live.sh
```

This:

- copies the DB from the active live backend container
- backs up the preview DB inside the preview container before replacing it
- restarts only the preview backend

During the initial migration period, if a blue/green live backend does not exist yet, the sync script falls back to the current legacy live backend container `word-search-backend`.

### Promote Preview To Live

After validating the preview stack:

```bash
./scripts/bluegreen/promote.sh
```

This:

- backs up the active live DB (db + WAL + SHM) into `.bluegreen/backups/`
- copies the live database into the promoted stack's volume (each stack has its own Docker volume; without this a promote would serve stale/empty data)
- flips the active/preview stack roles in `.bluegreen/state.env`
- renders `.bluegreen/Caddyfile`
- optionally publishes/reloads Caddy if `WORDSEARCH_CADDYFILE_TARGET` and `WORDSEARCH_CADDY_ADMIN` are configured

### Roll Back

```bash
./scripts/bluegreen/rollback.sh
```

This swaps the active and preview roles back and re-renders the Caddy config.

### Status

```bash
./scripts/bluegreen/status.sh
```

This prints the active stack, preview stack, local ports, and the intended live/preview hostnames.

## Native Development

### 1. Backend

```bash
cp .env.example .env
PORT=3001 cargo run --manifest-path backend/Cargo.toml
```

Backend defaults:

- API base: `http://localhost:3001`
- database: `./data/wordsearch.db`

### 2. Frontend

In a second terminal:

```bash
cd frontend
npm install
NEXT_PUBLIC_API_URL=http://localhost:3001 npm run dev
```

Frontend dev server defaults to `http://localhost:3000`.

## LAN / Device Testing

For phones or other devices on the same network, use the host machine IP in the browser:

```text
http://<host-ip>:3000    # next dev
http://<host-ip>:3089    # docker / standalone
```

The frontend resolves API calls against the page host so LAN clients do not fall back to `localhost`.

## Production-Style Local Run Without Docker

This is the safer mode for LAN testing because it avoids `next dev` host/runtime issues.

Backend:

```bash
PORT=3099 cargo run --manifest-path backend/Cargo.toml
```

Frontend:

```bash
cd frontend
npm install
npm run build
PORT=3089 HOSTNAME=0.0.0.0 npm run start:standalone
```

Open:

```text
http://localhost:3089
http://<host-ip>:3089
```

## Tests

Backend:

```bash
cargo test --manifest-path backend/Cargo.toml
```

Frontend build:

```bash
cd frontend
npm run build
```

Playwright:

```bash
cd frontend
npx playwright test
```

Playwright starts its own backend on `3099` and frontend on `3089`.

## Authentication Notes

- The current primary flow is server-backed player creation/picking from `/login`
- Existing password/recovery routes still exist in the backend for compatibility and tests
- Player preferences such as visual theme, preferred puzzle theme, and highlight color are stored on the server

## Git

Primary remote:

```text
git@mkgh:mark-styx/word_search.git
```

Default branch:

```text
main
```
