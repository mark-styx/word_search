"use client";

import { useEffect, useState } from "react";
import { CheckCircle2, Radio, Router, WifiOff } from "lucide-react";

import { Button, Card, Input } from "@/components/ui";
import { api, getApiBaseOverride, getApiBaseUrl, setApiBaseOverride } from "@/lib/api";

type ServerInfo = Awaited<ReturnType<typeof api.serverInfo>>;

type SavedServer = {
  apiUrl: string;
  frontendUrl: string;
  serverName: string;
  lastUsedAt: string;
};

const RECENT_SERVERS_KEY = "wordsearch_recent_servers_v1";
const MAX_RECENT_SERVERS = 8;

function loadRecentServers(): SavedServer[] {
  if (typeof window === "undefined") {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(RECENT_SERVERS_KEY);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed
      .filter((entry): entry is SavedServer => {
        return !!entry
          && typeof entry.apiUrl === "string"
          && typeof entry.frontendUrl === "string"
          && typeof entry.serverName === "string"
          && typeof entry.lastUsedAt === "string";
      })
      .sort((left, right) => {
        const leftTime = Date.parse(left.lastUsedAt) || 0;
        const rightTime = Date.parse(right.lastUsedAt) || 0;
        return rightTime - leftTime;
      })
      .slice(0, MAX_RECENT_SERVERS);
  } catch {
    return [];
  }
}

function saveRecentServer(entry: SavedServer) {
  if (typeof window === "undefined") {
    return;
  }

  const next = [
    entry,
    ...loadRecentServers().filter((current) => current.apiUrl !== entry.apiUrl),
  ].slice(0, MAX_RECENT_SERVERS);

  window.localStorage.setItem(RECENT_SERVERS_KEY, JSON.stringify(next));
}

function normalizeServerInput(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const withScheme = /^[a-z]+:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`;

  try {
    return new URL(withScheme);
  } catch {
    return null;
  }
}

async function probeServer(input: string): Promise<ServerInfo> {
  const parsed = normalizeServerInput(input);
  if (!parsed) {
    throw new Error("Enter a valid server address.");
  }

  const candidates = new Set<string>();
  const baseOrigin = `${parsed.protocol}//${parsed.hostname}`;
  const explicitPort = parsed.port;

  if (explicitPort === "3099") {
    candidates.add(`${baseOrigin}:3099`);
  }
  if (explicitPort === "3089" || !explicitPort) {
    candidates.add(`${baseOrigin}:3099`);
  }
  if (explicitPort) {
    candidates.add(`${baseOrigin}:${explicitPort}`);
  } else {
    candidates.add(baseOrigin);
  }

  let lastError: Error | null = null;
  for (const apiBase of candidates) {
    try {
      const pingResponse = await fetch(`${apiBase}/api/discover/ping`, { cache: "no-store" });
      if (!pingResponse.ok) {
        throw new Error(`Discovery ping failed (${pingResponse.status})`);
      }

      const infoResponse = await fetch(`${apiBase}/api/discover/info`, { cache: "no-store" });
      if (!infoResponse.ok) {
        throw new Error(`Discovery info failed (${infoResponse.status})`);
      }

      return await infoResponse.json() as ServerInfo;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error("Failed to reach that server.");
    }
  }

  throw lastError ?? new Error("Failed to reach that server.");
}

export default function ServerDiscoveryCard() {
  const [currentServer, setCurrentServer] = useState<ServerInfo | null>(null);
  const [recentServers, setRecentServers] = useState<SavedServer[]>([]);
  const [serverInput, setServerInput] = useState("");
  const [loadingCurrent, setLoadingCurrent] = useState(true);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState("");

  const overrideActive = Boolean(getApiBaseOverride());
  const effectiveApiBase = getApiBaseUrl();

  useEffect(() => {
    setRecentServers(loadRecentServers());

    let cancelled = false;
    setLoadingCurrent(true);
    void api.serverInfo()
      .then((info) => {
        if (!cancelled) {
          setCurrentServer(info);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCurrentServer(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingCurrent(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const connectToServer = async (input: string) => {
    setConnecting(true);
    setError("");
    try {
      const info = await probeServer(input);
      setApiBaseOverride(info.api_url);
      const saved = {
        apiUrl: info.api_url,
        frontendUrl: info.frontend_url,
        serverName: info.server_name,
        lastUsedAt: new Date().toISOString(),
      };
      saveRecentServer(saved);
      setRecentServers(loadRecentServers());
      setCurrentServer(info);
      setServerInput("");
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "Failed to connect to that server.");
    } finally {
      setConnecting(false);
    }
  };

  const resetToLocalServer = async () => {
    setApiBaseOverride(null);
    setConnecting(true);
    setError("");
    try {
      const info = await api.serverInfo();
      setCurrentServer(info);
    } catch (nextError) {
      setCurrentServer(null);
      setError(nextError instanceof Error ? nextError.message : "Failed to reach the default server.");
    } finally {
      setConnecting(false);
    }
  };

  return (
    <Card className="w-full max-w-3xl">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="flex items-center gap-2">
              <Router size={18} className="text-primary" />
              <h2 className="text-lg font-semibold text-text">Server Discovery</h2>
            </div>
            <p className="mt-1 text-sm text-text-secondary">
              Auto-connect to this server, or switch to another LAN server by address and keep it in your recent list.
            </p>
          </div>
          {overrideActive && (
            <Button variant="ghost" size="sm" onClick={() => void resetToLocalServer()} loading={connecting}>
              Use This Device
            </Button>
          )}
        </div>

        {loadingCurrent ? (
          <div className="rounded-xl border border-border bg-bg-secondary/60 px-4 py-3 text-sm text-text-secondary">
            Checking the current server...
          </div>
        ) : currentServer ? (
          <div className="rounded-xl border border-primary/20 bg-primary/5 px-4 py-3">
            <div className="flex flex-wrap items-center gap-2">
              <CheckCircle2 size={16} className="text-primary" />
              <span className="font-medium text-text">{currentServer.server_name}</span>
              <span className="text-sm text-text-secondary">
                {currentServer.active_games} active games · {currentServer.players_online} players online
              </span>
            </div>
            <p className="mt-2 text-xs text-text-secondary">
              API: {currentServer.api_url}
              {" · "}
              Frontend: {currentServer.frontend_url}
              {" · "}
              Effective connection: {effectiveApiBase}
            </p>
          </div>
        ) : (
          <div className="rounded-xl border border-border bg-bg-secondary/60 px-4 py-3 text-sm text-text-secondary">
            <div className="flex items-center gap-2">
              <WifiOff size={16} />
              <span>Could not verify the current server automatically. Enter a LAN address below.</span>
            </div>
          </div>
        )}

        <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
          <div className="flex-1">
            <Input
              label="Connect To Another Server"
              placeholder="Examples: 192.168.1.147, 192.168.1.147:3089, http://192.168.1.147:3099"
              value={serverInput}
              onChange={(event) => {
                setError("");
                setServerInput(event.target.value);
              }}
            />
          </div>
          <Button
            onClick={() => void connectToServer(serverInput)}
            loading={connecting}
            disabled={!serverInput.trim()}
          >
            <Radio size={16} /> Connect
          </Button>
        </div>

        {error && (
          <div className="rounded-lg bg-error/10 px-3 py-2 text-sm text-error">{error}</div>
        )}

        {recentServers.length > 0 && (
          <div className="flex flex-col gap-2">
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-text-secondary">
              Recent Servers
            </p>
            <div className="grid gap-2 sm:grid-cols-2">
              {recentServers.map((server) => (
                <button
                  key={server.apiUrl}
                  type="button"
                  onClick={() => void connectToServer(server.apiUrl)}
                  className="rounded-xl border border-border bg-bg-secondary/60 px-4 py-3 text-left transition hover:border-primary/40 hover:bg-primary/5"
                >
                  <p className="font-medium text-text">{server.serverName}</p>
                  <p className="mt-1 text-xs text-text-secondary">{server.apiUrl}</p>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}
