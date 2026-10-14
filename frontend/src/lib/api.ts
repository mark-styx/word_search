import type { User, GameState, GameLobby, UserStats, GameHistoryEntry, LeaderboardEntry, WordBank, PlayerDirectoryEntry } from "@/types";

const DEFAULT_API_PORT = process.env.NEXT_PUBLIC_API_PORT || "3099";
const CONFIGURED_API_BASE = trimTrailingSlash(process.env.NEXT_PUBLIC_API_URL || "");
const INTERNAL_API_BASE = trimTrailingSlash(process.env.INTERNAL_API_BASE_URL || "");
const LOCAL_ONLY_HOSTNAMES = new Set(["localhost", "127.0.0.1", "word-search.local"]);
const API_BASE_OVERRIDE_KEY = "wordsearch_api_base_override_v1";

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, "");
}

function parseUrl(value: string): URL | null {
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

function isLocalOnlyHostname(hostname: string) {
  return LOCAL_ONLY_HOSTNAMES.has(hostname.toLowerCase());
}

function buildLoginRedirectUrl() {
  if (typeof window === "undefined") {
    return "/login";
  }

  const next = `${window.location.pathname}${window.location.search}`;
  return next && next !== "/"
    ? `/login?next=${encodeURIComponent(next)}`
    : "/login";
}

export function getApiBaseUrl() {
  if (typeof window === "undefined") {
    return INTERNAL_API_BASE || CONFIGURED_API_BASE || `http://127.0.0.1:${DEFAULT_API_PORT}`;
  }

  const override = getApiBaseOverride();
  if (override) {
    return override;
  }

  const pageUrl = new URL(window.location.href);
  const configuredUrl = CONFIGURED_API_BASE ? parseUrl(CONFIGURED_API_BASE) : null;

  if (CONFIGURED_API_BASE && configuredUrl) {
    const browserIsRemote = !isLocalOnlyHostname(pageUrl.hostname);
    const apiIsLocalOnly = isLocalOnlyHostname(configuredUrl.hostname);

    if (!browserIsRemote || !apiIsLocalOnly) {
      return CONFIGURED_API_BASE;
    }
  } else if (CONFIGURED_API_BASE) {
    return CONFIGURED_API_BASE;
  }

  return `${pageUrl.protocol}//${pageUrl.hostname}:${DEFAULT_API_PORT}`;
}

export function getApiBaseOverride() {
  if (typeof window === "undefined") {
    return null;
  }

  const raw = window.localStorage.getItem(API_BASE_OVERRIDE_KEY);
  if (!raw) {
    return null;
  }

  return trimTrailingSlash(raw);
}

export function setApiBaseOverride(value: string | null) {
  if (typeof window === "undefined") {
    return;
  }

  if (!value) {
    window.localStorage.removeItem(API_BASE_OVERRIDE_KEY);
    return;
  }

  window.localStorage.setItem(API_BASE_OVERRIDE_KEY, trimTrailingSlash(value));
}

class ApiClient {
  private getHeaders(): HeadersInit {
    const headers: HeadersInit = {
      "Content-Type": "application/json",
    };
    const token = getAccessToken();
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
    return headers;
  }

  private async request<T>(
    path: string,
    options: RequestInit = {}
  ): Promise<T> {
    const res = await fetch(`${getApiBaseUrl()}${path}`, {
      ...options,
      headers: { ...this.getHeaders(), ...options.headers },
    });
    if (res.status === 401) {
      clearTokens();
      if (typeof window !== "undefined" && !window.location.pathname.match(/\/(login|register)$/)) {
        window.location.href = buildLoginRedirectUrl();
      }
      throw new Error("Unauthorized");
    }
    if (!res.ok) {
      const errorMessage = await readErrorMessage(res);
      throw new Error(errorMessage || `HTTP ${res.status}`);
    }
    if (res.status === 204) {
      return undefined as T;
    }
    return res.json();
  }

  // Auth
  async register(username: string, password: string, recovery_pin?: string) {
    return this.request<{ token: string; user: User }>("/api/auth/register", {
      method: "POST",
      body: JSON.stringify({ username, password, recovery_pin }),
    });
  }

  async login(username: string, password: string) {
    return this.request<{ token: string; user: User }>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
  }

  async recover(username: string, recovery_pin: string) {
    return this.request<{ token: string; user: User }>("/api/auth/recover", {
      method: "POST",
      body: JSON.stringify({ username, recovery_pin }),
    });
  }

  async createGuest() {
    return this.request<{ token: string; user: User }>("/api/auth/guest", {
      method: "POST",
    });
  }

  async listPlayers(query?: string, limit = 60) {
    const params = new URLSearchParams({ limit: String(limit) });
    if (query?.trim()) {
      params.set("q", query.trim());
    }
    return this.request<{ players: PlayerDirectoryEntry[] }>(`/api/auth/players?${params.toString()}`);
  }

  async createPlayer(username: string) {
    return this.request<{ token: string; user: User }>("/api/auth/players", {
      method: "POST",
      body: JSON.stringify({ username }),
    });
  }

  async pickPlayer(username: string) {
    return this.request<{ token: string; user: User }>("/api/auth/players/pick", {
      method: "POST",
      body: JSON.stringify({ username }),
    });
  }

  async searchRecoveryUsers(query: string) {
    const params = new URLSearchParams({ q: query });
    return this.request<{ usernames: string[] }>(`/api/auth/recovery/search?${params.toString()}`);
  }

  async getMe() {
    return this.request<User>("/api/auth/me");
  }

  async updateUsername(username: string) {
    return this.request<{ token: string; user: User }>("/api/auth/me/username", {
      method: "PATCH",
      body: JSON.stringify({ username }),
    });
  }

  async updatePreferences(preferences: {
    visual_theme: string;
    preferred_word_theme: string | null;
    highlight_color: string;
  }) {
    return this.request<User>("/api/auth/me/preferences", {
      method: "PATCH",
      body: JSON.stringify(preferences),
    });
  }

  // Games
  async createGame(options: {
    theme?: string;
    word_bank_id?: string;
    difficulty: string;
    time_limit_secs?: number;
    max_players?: number;
  }) {
    return this.request<{ id: string }>("/api/games", {
      method: "POST",
      body: JSON.stringify(options),
    });
  }

  async listGames() {
    return this.request<GameLobby[]>("/api/games");
  }

  async getGame(id: string) {
    return this.request<GameState>(`/api/games/${id}`);
  }

  async joinGame(id: string) {
    return this.request<{ id: string }>(`/api/games/${id}/join`, {
      method: "POST",
    });
  }

  async startGame(id: string) {
    return this.request<{ id: string }>(`/api/games/${id}/start`, {
      method: "POST",
    });
  }

  async soloGame() {
    return this.request<{ id: string }>("/api/games/solo");
  }

  async listWordBanks() {
    return this.request<WordBank[]>("/api/word-banks");
  }

  async generateWordBank(prompt: string) {
    return this.request<{ id: string; status: string }>("/api/word-banks/generate", {
      method: "POST",
      body: JSON.stringify({ prompt }),
    });
  }

  async updateWordBank(id: string, updates: { is_shared: boolean }) {
    return this.request<void>(`/api/word-banks/${id}`, {
      method: "PATCH",
      body: JSON.stringify(updates),
    });
  }

  async deleteWordBank(id: string) {
    return this.request<void>(`/api/word-banks/${id}`, {
      method: "DELETE",
    });
  }

  // Stats
  async getLeaderboard(page = 1, limit = 25) {
    return this.request<{ entries: LeaderboardEntry[]; total: number }>(
      `/api/stats/leaderboard?page=${page}&limit=${limit}`
    );
  }

  async getMyStats() {
    return this.request<UserStats>("/api/stats/me");
  }

  async getMyHistory(page = 1, limit = 20) {
    return this.request<{ entries: GameHistoryEntry[]; total: number }>(
      `/api/stats/me/history?page=${page}&limit=${limit}`
    );
  }

  // Discovery
  async ping() {
    return this.request<{ service: string; version: string }>(
      "/api/discover/ping"
    );
  }

  async serverInfo() {
    return this.request<{
      server_name: string;
      hostname: string;
      port: number;
      api_url: string;
      frontend_url: string;
      active_games: number;
      players_online: number;
    }>("/api/discover/info");
  }
}

async function readErrorMessage(res: Response): Promise<string | null> {
  const contentType = res.headers.get("content-type") || "";

  if (contentType.includes("application/json")) {
    const body = await res.json().catch(() => null);
    if (body && typeof body === "object" && "message" in body && typeof body.message === "string") {
      return body.message;
    }
    return null;
  }

  const text = await res.text().catch(() => "");
  const trimmed = text.trim();
  return trimmed.length > 0 ? trimmed : null;
}

type TokenClaims = {
  sub?: string;
  username?: string;
};

function decodeBase64Url(value: string): string | null {
  try {
    const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), "=");
    return atob(padded);
  } catch {
    return null;
  }
}

function getTokenClaims(): TokenClaims | null {
  if (typeof window === "undefined") return null;

  const token = getAccessToken();
  if (!token) return null;

  const parts = token.split(".");
  if (parts.length < 2) return null;

  const payload = decodeBase64Url(parts[1]);
  if (!payload) return null;

  try {
    return JSON.parse(payload) as TokenClaims;
  } catch {
    return null;
  }
}

export function getAccessToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("access_token");
}

export function setAccessToken(token: string) {
  localStorage.setItem("access_token", token);
}

export function clearTokens() {
  localStorage.removeItem("access_token");
}

export function getCurrentUserId(): string | null {
  return getTokenClaims()?.sub ?? null;
}

export function getCurrentUsername(): string | null {
  return getTokenClaims()?.username ?? null;
}

export const api = new ApiClient();
