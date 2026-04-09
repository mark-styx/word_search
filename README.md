# Word Search LAN Party Game

Real-time multiplayer word search game with LAN auto-discovery.

## Quick Start

```bash
# Copy environment config
cp .env.example .env

# Start with Docker Compose
docker compose up --build

# Open http://localhost:3000
```

## Development

### Backend (Rust / Axum)

```bash
cd backend
cp .env.example .env
cargo run
```

Server starts on `http://0.0.0.0:3001`.

### Frontend (Next.js / TypeScript / Tailwind)

```bash
cd frontend
npm install
npm run dev
```

Frontend starts on `http://localhost:3000`.

## Features

- Solo and multiplayer word search
- 10 word themes with 600+ words
- Grid sizes: 8x8 (Easy) to 20x20 (Expert)
- Live opponent word visibility via WebSocket
- LAN auto-discovery via mDNS
- Persistent player profiles with SQLite
- 7 visual themes
- QR code for mobile join
- Docker Compose deployment

## Architecture

- **Backend**: Rust (Axum, tokio, sqlx SQLite, JWT auth, WebSocket, mDNS)
- **Frontend**: Next.js (TypeScript, Tailwind CSS, zustand, framer-motion)
- **Persistence**: SQLite
- **Discovery**: mDNS (`_wordsearch._tcp.local.`)
- **Deployment**: Docker Compose

## LAN Play

Other devices on the same network can join at `http://<host-ip>:3000`. Auto-discovery attempts `http://word-search.local:3001` first.
