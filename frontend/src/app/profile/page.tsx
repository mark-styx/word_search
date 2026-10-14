"use client";

import type { FormEvent } from "react";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Trophy, ArrowLeft, Palette, BarChart3, History, Globe2, Lock, Trash2, Sparkles, UserRound } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button, Card, Select, Badge, Input } from "@/components/ui";
import { useTheme } from "@/lib/themes-provider";
import { highlightSwatches, themes } from "@/lib/themes";
import { difficultyLabels, getThemeLabel, themeOptions } from "@/lib/game-themes";
import { api, getAccessToken, setAccessToken } from "@/lib/api";
import { rememberCurrentSession } from "@/lib/profiles";
import type { User, UserStats, GameHistoryEntry, WordTheme, WordBank } from "@/types";

export default function ProfilePage() {
  const router = useRouter();
  const {
    theme,
    setTheme,
    highlightColor,
    setHighlightColor,
    preferredWordTheme,
    setPreferredWordTheme,
  } = useTheme();
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [stats, setStats] = useState<UserStats | null>(null);
  const [history, setHistory] = useState<GameHistoryEntry[]>([]);
  const [customBanks, setCustomBanks] = useState<WordBank[]>([]);
  const [loading, setLoading] = useState(true);
  const [updatingBankId, setUpdatingBankId] = useState<string | null>(null);
  const [deletingBankId, setDeletingBankId] = useState<string | null>(null);
  const [usernameDraft, setUsernameDraft] = useState("");
  const [renaming, setRenaming] = useState(false);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renameNotice, setRenameNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!getAccessToken()) {
      if (typeof window !== "undefined") {
        window.location.replace("/login");
      } else {
        router.push("/login");
      }
      return;
    }

    Promise.all([api.getMe(), api.getMyStats(), api.getMyHistory(1, 10), api.listWordBanks()])
      .then(([me, s, h, banks]) => {
        setCurrentUser(me as User);
        setUsernameDraft((me as User).username);
        setStats(s as UserStats);
        setHistory((h as { entries: GameHistoryEntry[] }).entries ?? []);
        setCustomBanks(
          (banks as WordBank[]).filter((bank) => bank.source === "custom" && bank.is_owner)
        );
      })
      .catch(() => {
        // will redirect via 401 handler
      })
      .finally(() => setLoading(false));
  }, [router]);

  if (loading) {
    return (
      <PageContainer className="flex items-center justify-center">
        <div className="text-text-secondary">Loading profile...</div>
      </PageContainer>
    );
  }

  async function refreshCustomBanks() {
    const banks = (await api.listWordBanks()) as WordBank[];
    setCustomBanks(banks.filter((bank) => bank.source === "custom" && bank.is_owner));
  }

  async function handleToggleSharing(bank: WordBank) {
    setUpdatingBankId(bank.id);
    try {
      await api.updateWordBank(bank.id, { is_shared: !bank.is_shared });
      await refreshCustomBanks();
    } finally {
      setUpdatingBankId(null);
    }
  }

  async function handleDeleteWordBank(bank: WordBank) {
    if (typeof window !== "undefined") {
      const confirmed = window.confirm(`Delete "${bank.label}"? Existing games will keep working, but this bank will stop showing up for new games.`);
      if (!confirmed) return;
    }

    setDeletingBankId(bank.id);
    try {
      await api.deleteWordBank(bank.id);
      await refreshCustomBanks();
    } finally {
      setDeletingBankId(null);
    }
  }

  async function handleRenameSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!currentUser) return;

    const nextUsername = usernameDraft.trim();
    if (nextUsername.length < 3) {
      setRenameError("Screen name must be at least 3 characters.");
      setRenameNotice(null);
      return;
    }

    if (nextUsername === currentUser.username) {
      setRenameError(null);
      setRenameNotice("Your screen name is already up to date.");
      return;
    }

    setRenaming(true);
    setRenameError(null);
    setRenameNotice(null);

    try {
      const { token, user } = await api.updateUsername(nextUsername);
      setAccessToken(token);
      rememberCurrentSession(user);
      setCurrentUser(user);
      setUsernameDraft(user.username);
      setRenameNotice("Screen name updated everywhere for future games and stats.");
    } catch (error) {
      setRenameError(error instanceof Error ? error.message : "Could not update your screen name.");
    } finally {
      setRenaming(false);
    }
  }

  return (
    <PageContainer className="flex flex-col gap-6 py-8">
      <div className="flex items-center gap-3">
        <button
          onClick={() => router.back()}
          className="rounded-lg p-2 text-text-secondary hover:bg-bg-secondary transition-colors"
        >
          <ArrowLeft size={20} />
        </button>
        <h1 className="text-2xl font-bold text-text">Profile & Stats</h1>
      </div>

      <Card>
        <div className="mb-4 flex items-center gap-2">
          <UserRound size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Account</h2>
        </div>
        <form className="flex flex-col gap-3" onSubmit={(event) => void handleRenameSubmit(event)}>
          <Input
            label="Screen Name"
            value={usernameDraft}
            onChange={(e) => {
              setUsernameDraft(e.target.value);
              if (renameError) setRenameError(null);
              if (renameNotice) setRenameNotice(null);
            }}
            error={renameError ?? undefined}
            placeholder="Pick the name other players will see"
            maxLength={32}
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" loading={renaming}>
              Save Screen Name
            </Button>
            <span className="text-sm text-text-secondary">
              {currentUser?.is_guest
                ? "Guest names stay private to this guest account but still need to be unique."
                : "Leaderboards, history, lobby, and player picking will use your new name."}
            </span>
          </div>
          {renameNotice && <p className="text-sm text-success">{renameNotice}</p>}
        </form>
      </Card>

      {/* Theme selection */}
      <Card>
        <div className="flex items-center gap-2 mb-3">
          <Palette size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Visual Preferences</h2>
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          <div>
            <Select
              label="App Theme"
              value={theme.id}
              onChange={(e) => setTheme(e.target.value)}
              options={themes.map((t) => ({ value: t.id, label: t.name }))}
            />
            <div className="mt-3 flex gap-2">
              {themes.map((t) => (
                <button
                  key={t.id}
                  onClick={() => setTheme(t.id)}
                  className={`h-8 w-8 rounded-full border-2 transition-all ${
                    t.id === theme.id ? "border-primary scale-110" : "border-border"
                  }`}
                  style={{ background: t.colors.primary }}
                  title={t.name}
                />
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-3">
            <label className="text-sm font-medium text-text">Highlight Color</label>
            <div className="flex items-center gap-3">
              <input
                type="color"
                value={highlightColor}
                onChange={(e) => setHighlightColor(e.target.value)}
                className="h-11 w-16 cursor-pointer rounded-lg border border-border bg-bg p-1"
                aria-label="Pick your word highlight color"
              />
              <span className="text-sm text-text-secondary">
                Used for your found words, active tile selection, and what other players see in multiplayer.
              </span>
            </div>
            <div className="flex flex-wrap gap-2">
              {highlightSwatches.map((swatch) => (
                <button
                  key={swatch}
                  onClick={() => setHighlightColor(swatch)}
                  className={`h-8 w-8 rounded-full border-2 transition-transform ${
                    highlightColor.toLowerCase() === swatch.toLowerCase()
                      ? "scale-110 border-text"
                      : "border-border"
                  }`}
                  style={{ background: swatch }}
                  title={swatch}
                />
              ))}
            </div>
          </div>
        </div>

        <div className="mt-4">
          <Select
            label="Default Puzzle Theme"
            value={preferredWordTheme}
            onChange={(e) => setPreferredWordTheme(e.target.value as WordTheme)}
            options={themeOptions}
          />
          <p className="mt-2 text-sm text-text-secondary">
            New games will open with this theme preselected.
          </p>
        </div>
      </Card>

      <Card>
        <div className="flex items-center gap-2 mb-4">
          <Sparkles size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Your Custom Word Banks</h2>
        </div>
        {customBanks.length > 0 ? (
          <div className="flex flex-col gap-3">
            {customBanks.map((bank) => (
              <div
                key={bank.id}
                className="rounded-xl border border-border bg-bg-secondary/40 p-4"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="font-semibold text-text">{bank.label}</h3>
                      <Badge variant={bank.status === "ready" ? "success" : bank.status === "failed" ? "error" : "default"}>
                        {bank.status === "ready" ? "Ready" : bank.status === "failed" ? "Failed" : "Generating"}
                      </Badge>
                      <Badge variant={bank.is_shared ? "primary" : "default"}>
                        {bank.is_shared ? "Shared" : "Private"}
                      </Badge>
                    </div>
                    <p className="mt-2 text-sm text-text-secondary">
                      {bank.used_words_count}/{bank.total_words || 0} unique words used · {bank.total_usage_count} total placements
                    </p>
                    {(bank.error_message || bank.last_extension_error) && (
                      <p className="mt-2 text-sm text-error">{bank.error_message || bank.last_extension_error}</p>
                    )}
                  </div>

                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => handleToggleSharing(bank)}
                      loading={updatingBankId === bank.id}
                    >
                      {bank.is_shared ? <Lock size={16} /> : <Globe2 size={16} />}
                      {bank.is_shared ? "Make Private" : "Share With Others"}
                    </Button>
                    {bank.status === "ready" && (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => router.push("/lobby")}
                      >
                        Use in Lobby
                      </Button>
                    )}
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => handleDeleteWordBank(bank)}
                      loading={deletingBankId === bank.id}
                    >
                      <Trash2 size={16} />
                      Delete
                    </Button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-text-secondary">No custom word banks yet. Generate one from the lobby when you want a new topic.</p>
        )}
      </Card>

      {/* Stats */}
      <Card>
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Your Stats</h2>
        </div>
        {stats ? (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
            <StatItem label="Games Played" value={stats.total_games} />
            <StatItem label="Games Won" value={stats.games_won} />
            <StatItem label="Total Score" value={stats.total_score} />
            <StatItem label="Best Score" value={stats.best_single_game_score} />
            <StatItem label="Words Found" value={stats.total_words_found} />
            <StatItem
              label="Win Rate"
              value={stats.total_games > 0 ? `${Math.round((stats.games_won / stats.total_games) * 100)}%` : "-"}
            />
            <StatItem label="Contested Wins" value={stats.contested_words_won} />
            <StatItem label="Contested Losses" value={stats.contested_words_lost} />
            <StatItem
              label="Closest Contest"
              value={stats.closest_contest_ms != null ? `${stats.closest_contest_ms} ms` : "-"}
            />
            <StatItem
              label="Fastest Find"
              value={stats.fastest_word_find_ms != null ? `${stats.fastest_word_find_ms} ms` : "-"}
            />
            <StatItem
              label="Avg Find Time"
              value={stats.average_word_find_ms != null ? `${stats.average_word_find_ms} ms` : "-"}
            />
          </div>
        ) : (
          <p className="text-text-secondary">No stats yet. Play some games!</p>
        )}
      </Card>

      {/* History */}
      <Card>
        <div className="flex items-center gap-2 mb-4">
          <History size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Recent Games</h2>
        </div>
        {history.length > 0 ? (
          <div className="flex flex-col gap-2">
            {history.map((g) => (
              <div
                key={g.game_id}
                className="flex items-center justify-between rounded-lg border border-border px-3 py-2"
              >
                <div className="flex items-center gap-2">
                  <Badge variant={g.rank === 1 ? "success" : "default"}>
                    #{g.rank}
                  </Badge>
                  <span className="text-sm text-text">{getThemeLabel(g.theme)}</span>
                  <Badge variant="primary">{difficultyLabels[g.difficulty] || g.difficulty}</Badge>
                </div>
                <div className="flex items-center gap-3 text-sm text-text-secondary">
                  <span>{g.words_found} words</span>
                  <span className="font-medium text-text">{g.score} pts</span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-text-secondary">No games played yet.</p>
        )}
      </Card>

      <Link href="/leaderboard">
        <Button variant="secondary" className="w-full">
          <Trophy size={16} />
          View Full Leaderboard
        </Button>
      </Link>
    </PageContainer>
  );
}

function StatItem({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex flex-col">
      <span className="text-2xl font-bold text-text">{value}</span>
      <span className="text-xs text-text-secondary">{label}</span>
    </div>
  );
}
