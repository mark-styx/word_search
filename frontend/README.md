# Frontend

Next.js client for the Word Search LAN Party app.

## LAN Docker Runtime

The primary hosted-play path is the root Compose stack:

```bash
cd ..
docker compose up --build -d
```

That serves:

- frontend on `3089`
- backend on `3099`

The browser uses the page host for API calls, while the Next server uses `INTERNAL_API_BASE_URL` inside the container network.

## Development

```bash
cd frontend
npm install
NEXT_PUBLIC_API_URL=http://localhost:3001 npm run dev
```

Open `http://localhost:3000`.

The frontend expects a running backend. For normal local development that backend is usually:

```bash
PORT=3001 cargo run --manifest-path ../backend/Cargo.toml
```

## Production-Style Local Start

Use this for LAN/device testing so the app behaves like a built deployment:

```bash
cd frontend
npm run build
PORT=3089 HOSTNAME=0.0.0.0 npm run start:standalone
```

`start:standalone` copies the built static assets and `public/` files into the Next standalone output before starting `server.js`.

## Test Commands

```bash
npm run build
npx playwright test
```

Playwright uses:

- backend on `3099`
- frontend on `3089`

## Notes

- Browser-side API resolution is LAN-safe and avoids `localhost` when the page is opened from a remote device
- The login flow uses the server-backed player directory
- Service worker/PWA caching is disabled in development to avoid stale client bundles during active iteration

See the repo root [README](../README.md) for the full project overview.
