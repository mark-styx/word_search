import type { User } from "@/types";

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "http://word-search.local:3001";

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

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
    const res = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: { ...this.getHeaders(), ...options.headers },
    });
    if (res.status === 401) {
      clearTokens();
      if (typeof window !== "undefined") {
        window.location.href = "/login";
      }
      throw new Error("Unauthorized");
    }
    if (!res.ok) {
      const body = await res.json().catch(() => ({}));
      throw new Error(body.message || `HTTP ${res.status}`);
    }
    return res.json();
  }

  // Auth
  async register(username: string, password: string) {
    return this.request<{ token: string; user: User }>("/api/auth/register", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
  }

  async login(username: string, password: string) {
    return this.request<{ token: string; user: User }>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
  }

  async getMe() {
    return this.request<User>("/api/auth/me");
  }

  // Games
  async createGame(options: {
    theme: string;
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
    return this.request<{ id: string; theme: string; difficulty: string; host: string; player_count: number; max_players: number }[]>("/api/games");
  }

  async getGame(id: string) {
    return this.request<Record<string, unknown>>(`/api/games/${id}`);
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

  // Stats
  async getLeaderboard(page = 1, limit = 20) {
    return this.request<{ entries: unknown[]; total: number }>(
      `/api/stats/leaderboard?page=${page}&limit=${limit}`
    );
  }

  async getMyStats() {
    return this.request<Record<string, unknown>>("/api/stats/me");
  }

  async getMyHistory(page = 1, limit = 20) {
    return this.request<{ entries: unknown[]; total: number }>(
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
    return this.request<Record<string, unknown>>("/api/discover/info");
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

export const api = new ApiClient(API_BASE);
