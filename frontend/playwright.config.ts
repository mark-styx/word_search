import { defineConfig, devices } from "@playwright/test";

const LIVE_BACKEND_PORT = 3099;
const LIVE_FRONTEND_PORT = 3089;
const BACKEND_PORT = Number(process.env.E2E_BACKEND_PORT || "41099");
const FRONTEND_PORT = Number(process.env.E2E_FRONTEND_PORT || "41089");

if (
  process.env.ALLOW_E2E_LIVE_PORTS !== "1" &&
  (BACKEND_PORT === LIVE_BACKEND_PORT || FRONTEND_PORT === LIVE_FRONTEND_PORT)
) {
  throw new Error(
    `Refusing to run Playwright on live LAN ports ${LIVE_FRONTEND_PORT}/${LIVE_BACKEND_PORT}. ` +
      "Use E2E_FRONTEND_PORT and E2E_BACKEND_PORT, or set ALLOW_E2E_LIVE_PORTS=1 to override."
  );
}

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  retries: 1,
  use: {
    baseURL: `http://127.0.0.1:${FRONTEND_PORT}`,
    headless: true,
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: [
    {
      command: `WORD_BANK_GENERATOR_BIN=../backend/tests/fake_word_bank_generator.sh DATABASE_PATH=:memory: PORT=${BACKEND_PORT} cargo run --manifest-path ../backend/Cargo.toml`,
      port: BACKEND_PORT,
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: `NEXT_PUBLIC_API_URL=http://127.0.0.1:${BACKEND_PORT} npm run dev -- --port ${FRONTEND_PORT}`,
      port: FRONTEND_PORT,
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
