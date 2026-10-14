import { Page, expect } from "@playwright/test";

const BACKEND_PORT = process.env.E2E_BACKEND_PORT || "41099";
const API_BASE = `http://127.0.0.1:${BACKEND_PORT}`;

/** Create a player via the current no-password API and return the token. */
export async function registerUserApi(
  username: string,
  _password?: string,
  _recoveryPin?: string,
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/auth/players`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.token;
}

/** Pick an existing player via API and return the token. */
export async function loginUserApi(
  username: string,
  _password?: string,
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/auth/players/pick`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.token;
}

/** Inject an auth token into localStorage so the app thinks we're logged in. */
export async function setAuth(page: Page, token: string) {
  await page.addInitScript((nextToken: string) => {
    window.localStorage.setItem("access_token", nextToken);
  }, token);
  await page.goto("/");
  await page.evaluate((nextToken) => window.localStorage.setItem("access_token", nextToken), token);
  await page.reload();
}

/** Register via API, set token, return token. */
export async function registerAndAuth(
  page: Page,
  username: string,
  _password?: string,
): Promise<string> {
  const token = await registerUserApi(username);
  await setAuth(page, token);
  return token;
}

export async function waitForProfilePicker(page: Page) {
  await expect(page.getByRole("heading", { name: "Pick a player" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh" })).toBeEnabled({ timeout: 10_000 });
  await expect(page.getByRole("button", { name: "Create Player" })).toBeEnabled({ timeout: 10_000 });
}

/** Create a multiplayer game via API, return game id. */
export async function createGameApi(
  token: string,
  theme = "science",
  difficulty = "medium",
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/games`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({ theme, difficulty, max_players: 4 }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.id;
}

export async function createGameWithWordBankApi(
  token: string,
  wordBankId: string,
  difficulty = "easy",
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/games`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({ word_bank_id: wordBankId, difficulty, max_players: 4 }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.id;
}

export async function listWordBanksApi(token: string) {
  const res = await fetch(`${API_BASE}/api/word-banks`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(res.ok).toBeTruthy();
  return res.json();
}

/** Join a game via API. */
export async function joinGameApi(token: string, gameId: string) {
  const res = await fetch(`${API_BASE}/api/games/${gameId}/join`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(res.ok).toBeTruthy();
}

/** Start a game via API. */
export async function startGameApi(token: string, gameId: string) {
  const res = await fetch(`${API_BASE}/api/games/${gameId}/start`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(res.ok).toBeTruthy();
}

/** Create a solo game via API, return game id. */
export async function createSoloGameApi(token: string): Promise<string> {
  const res = await fetch(`${API_BASE}/api/games/solo`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.id;
}

/** Get game state via API. */
export async function getGameApi(token: string, gameId: string) {
  const res = await fetch(`${API_BASE}/api/games/${gameId}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(res.ok).toBeTruthy();
  return res.json();
}

/** Unique suffix for test usernames to avoid collisions. */
export function uniqueSuffix(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}
