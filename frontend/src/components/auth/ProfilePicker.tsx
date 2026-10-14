"use client";

import { FormEvent, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Search, Plus, UserRound, RefreshCw, Sparkles } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button, Input } from "@/components/ui";
import { api, setAccessToken } from "@/lib/api";
import type { PlayerDirectoryEntry } from "@/types";

type ProfilePickerProps = {
  emphasizeCreate?: boolean;
};

export default function ProfilePicker({ emphasizeCreate = false }: ProfilePickerProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [players, setPlayers] = useState<PlayerDirectoryEntry[]>([]);
  const [playerSearch, setPlayerSearch] = useState("");
  const [username, setUsername] = useState("");
  const [interactive, setInteractive] = useState(false);
  const [loadingPlayers, setLoadingPlayers] = useState(true);
  const [directoryError, setDirectoryError] = useState("");
  const [formError, setFormError] = useState("");
  const [creating, setCreating] = useState(false);
  const [creatingGuest, setCreatingGuest] = useState(false);
  const [playingUsername, setPlayingUsername] = useState<string | null>(null);

  const loadPlayers = async (query = "") => {
    setLoadingPlayers(true);
    try {
      const result = await api.listPlayers(query);
      setPlayers(result.players);
      setDirectoryError("");
    } catch (err) {
      setDirectoryError(err instanceof Error ? err.message : "Couldn’t load players");
      setPlayers([]);
    } finally {
      setLoadingPlayers(false);
    }
  };

  const goToDestination = () => {
    const requested = searchParams.get("next") || "/lobby";
    const destination = requested.startsWith("/") ? requested : "/lobby";
    if (typeof window !== "undefined") {
      window.location.href = destination;
      return;
    }
    router.push(destination);
  };

  useEffect(() => {
    setInteractive(true);
    void loadPlayers();
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const result = await api.listPlayers(playerSearch);
          if (!cancelled) {
            setPlayers(result.players);
            setDirectoryError("");
          }
        } catch (err) {
          if (!cancelled) {
            setPlayers([]);
            setDirectoryError(err instanceof Error ? err.message : "Couldn’t load players");
          }
        } finally {
          if (!cancelled) {
            setLoadingPlayers(false);
          }
        }
      })();
    }, 180);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [playerSearch]);

  async function handleCreate(e: FormEvent) {
    e.preventDefault();
    const nextUsername = username.trim();
    setFormError("");

    if (nextUsername.length < 3) {
      setFormError("Username must be at least 3 characters");
      return;
    }

    setCreating(true);
    try {
      const { token } = await api.createPlayer(nextUsername);
      setAccessToken(token);
      goToDestination();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Couldn’t create that player";
      if (message === "Username already taken") {
        setFormError("That player already exists. Pick it from the list on the left.");
      } else {
        setFormError(message);
      }
    } finally {
      setCreating(false);
    }
  }

  async function handlePlay(usernameToOpen: string) {
    setPlayingUsername(usernameToOpen);
    setFormError("");
    try {
      const { token } = await api.pickPlayer(usernameToOpen);
      setAccessToken(token);
      goToDestination();
    } catch (err) {
      setFormError(err instanceof Error ? err.message : "Couldn’t open that player");
    } finally {
      setPlayingUsername(null);
    }
  }

  async function handleGuest() {
    setFormError("");
    setCreatingGuest(true);
    try {
      const { token } = await api.createGuest();
      setAccessToken(token);
      goToDestination();
    } catch (err) {
      setFormError(err instanceof Error ? err.message : "Couldn’t create a guest player");
    } finally {
      setCreatingGuest(false);
    }
  }

  return (
    <PageContainer className="flex items-center justify-center py-14">
      <div className="grid w-full max-w-4xl gap-6 lg:grid-cols-[1.1fr_0.9fr]">
        <section className={`rounded-3xl border border-border bg-bg-secondary/70 p-6 shadow-sm ${emphasizeCreate ? "lg:order-2" : ""}`}>
          <div className="mb-5 flex items-center gap-3">
            <div className="rounded-2xl bg-primary/12 p-3 text-primary">
              <UserRound size={24} />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-text">Pick a player</h1>
              <p className="text-sm text-text-secondary">Live players from this server. Tap one and go straight in.</p>
            </div>
          </div>

          <div className="mb-4 flex items-end gap-2">
            <div className="flex-1">
              <Input
                label="Search Players"
                value={playerSearch}
                onChange={(e) => {
                  setLoadingPlayers(true);
                  setPlayerSearch(e.target.value);
                }}
                placeholder="Search server players"
              />
            </div>
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setDirectoryError("");
                void loadPlayers(playerSearch);
              }}
              disabled={!interactive || loadingPlayers}
            >
              <RefreshCw size={16} />
              Refresh
            </Button>
          </div>

          <div className="flex flex-col gap-3">
            {directoryError && (
              <div className="rounded-2xl bg-error/10 px-3 py-2 text-sm text-error">
                {directoryError}
              </div>
            )}

            {loadingPlayers && (
              <div className="rounded-2xl border border-dashed border-border px-4 py-5 text-sm text-text-secondary">
                Loading players from the server...
              </div>
            )}

            {!loadingPlayers && players.length === 0 && (
              <div className="rounded-2xl border border-dashed border-border px-4 py-5 text-sm text-text-secondary">
                No players found on this server yet. Create one on the right.
              </div>
            )}

            {players.map((player) => {
              const isBusy = playingUsername === player.username;
              return (
                <div
                  key={player.id}
                  className="flex items-center justify-between gap-3 rounded-2xl border border-border bg-bg px-4 py-3"
                >
                  <div className="min-w-0">
                    <div className="truncate font-semibold text-text">{player.username}</div>
                    <div className="text-xs text-text-secondary">
                      Joined {new Date(player.created_at).toLocaleString()}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => void handlePlay(player.username)}
                    disabled={!interactive || isBusy}
                    className="inline-flex min-w-[82px] items-center justify-center rounded-xl bg-primary px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-primary-hover disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    {isBusy ? "Opening..." : "Play"}
                  </button>
                </div>
              );
            })}
          </div>
        </section>

        <section className={`rounded-3xl border border-border bg-bg p-6 shadow-sm ${emphasizeCreate ? "lg:order-1" : ""}`}>
          <div className="mb-5 flex items-center gap-3">
            <div className="rounded-2xl bg-primary/12 p-3 text-primary">
              <Plus size={24} />
            </div>
            <div>
              <h2 className="text-2xl font-bold text-text">Create a player</h2>
              <p className="text-sm text-text-secondary">Choose a new player name. No password screen.</p>
            </div>
          </div>

          <form onSubmit={handleCreate} className="flex flex-col gap-4">
            <Input
              label="Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Pick a player name"
              autoComplete="nickname"
              required
            />

            {formError && (
              <div className="rounded-2xl bg-error/10 px-3 py-2 text-sm text-error">
                {formError}
              </div>
            )}

            <Button type="submit" className="w-full" loading={creating} disabled={!interactive}>
              Create Player
            </Button>
          </form>

          <div className="mt-4">
            <Button
              type="button"
              variant="secondary"
              className="w-full"
              onClick={() => void handleGuest()}
              loading={creatingGuest}
              disabled={!interactive}
            >
              <Sparkles size={16} />
              Play as Guest
            </Button>
          </div>

          <div className="mt-6 rounded-2xl bg-bg-secondary/80 px-4 py-4 text-sm text-text-secondary">
            Players come from the server now, not this device. If you just want to jump in, guest mode creates a temporary player without adding it to the shared picker.
          </div>

          <div className="mt-6 flex items-center gap-2 text-xs uppercase tracking-[0.22em] text-text-secondary">
            <Search size={14} />
            Fast local player switching
          </div>
        </section>
      </div>
    </PageContainer>
  );
}
