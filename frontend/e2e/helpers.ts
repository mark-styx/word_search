import { Page, expect } from "@playwright/test";

const BACKEND_PORT = 3099;
const API_BASE = `http://localhost:${BACKEND_PORT}`;

/** Register a user via API and return the token. */
export async function registerUserApi(
  username: string,
  password: string,
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.token;
}

/** Login a user via API and return the token. */
export async function loginUserApi(
  username: string,
  password: string,
): Promise<string> {
  const res = await fetch(`${API_BASE}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
  expect(res.ok).toBeTruthy();
  const data = await res.json();
  return data.token;
}

/** Inject an auth token into localStorage so the app thinks we're logged in. */
export async function setAuth(page: Page, token: string) {
  await page.goto("/");
  await page.evaluate((t) => localStorage.setItem("access_token", t), token);
}

/** Register via API, set token, return token. */
export async function registerAndAuth(
  page: Page,
  username: string,
  password: string,
): Promise<string> {
  const token = await registerUserApi(username, password);
  await setAuth(page, token);
  return token;
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
