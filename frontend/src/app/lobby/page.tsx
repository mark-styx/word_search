"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  Search,
  Plus,
  QrCode,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { PageContainer } from "@/components/layout";
import ActiveGamesSection from "@/components/lobby/ActiveGamesSection";
import CreateGameModal from "@/components/lobby/CreateGameModal";
import GenerateWordBankModal from "@/components/lobby/GenerateWordBankModal";
import ShareLobbyModal from "@/components/lobby/ShareLobbyModal";
import WordBankLibrarySection from "@/components/lobby/WordBankLibrarySection";
import { Button } from "@/components/ui";
import { api, getAccessToken } from "@/lib/api";
import { useTheme } from "@/lib/themes-provider";
import { difficultyOptions, getThemeLabel } from "@/lib/game-themes";
import type { Difficulty, GameLobby, WordBank } from "@/types";

const DEFAULT_CREATE_THEME = "science";
const bankLabelCollator = new Intl.Collator(undefined, { sensitivity: "base" });

type ToastTone = "success" | "error";

type ToastMessage = {
  id: number;
  message: string;
  tone: ToastTone;
};

export default function LobbyPage() {
  const router = useRouter();
  const { preferredWordTheme, setPreferredWordTheme } = useTheme();
  const pendingGeneratedIds = useRef(new Set<string>());
  const latestFetchId = useRef(0);
  const [games, setGames] = useState<GameLobby[]>([]);
  const [wordBanks, setWordBanks] = useState<WordBank[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [showQr, setShowQr] = useState(false);
  const [showGenerate, setShowGenerate] = useState(false);
  const [showWordBanks, setShowWordBanks] = useState(false);
  const [selectedBankKey, setSelectedBankKey] = useState<string>(preferredWordTheme);
  const [shareTarget, setShareTarget] = useState<string>("server");
  const [newDiff, setNewDiff] = useState<Difficulty>("medium");
  const [generationPrompt, setGenerationPrompt] = useState("");
  const [generateError, setGenerateError] = useState("");
  const [generatingBank, setGeneratingBank] = useState(false);
  const [serverUrl, setServerUrl] = useState("");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [lastSyncedAt, setLastSyncedAt] = useState<number | null>(null);
  const [syncMessage, setSyncMessage] = useState("");

  const builtInReadyWordBanks = useMemo(
    () =>
      wordBanks
        .filter((bank) => bank.status === "ready" && bank.source === "builtin")
        .sort((left, right) => compareWordBanks(left, right)),
    [wordBanks]
  );
  const customReadyWordBanks = useMemo(
    () =>
      wordBanks
        .filter((bank) => bank.status === "ready" && bank.source === "custom")
        .sort((left, right) => compareWordBanks(left, right)),
    [wordBanks]
  );
  const readyWordBanks = useMemo(
    () => [...builtInReadyWordBanks, ...customReadyWordBanks],
    [builtInReadyWordBanks, customReadyWordBanks]
  );
  const sortedWordBanks = useMemo(
    () => [...wordBanks].sort((left, right) => compareWordBanks(left, right)),
    [wordBanks]
  );
  const selectedBank = readyWordBanks.find((bank) => bank.key === selectedBankKey) ?? readyWordBanks[0] ?? null;
  const shareOptions = useMemo(
    () => [
      { value: "server", label: "This Server" },
      ...games.map((game) => ({
        value: game.id,
        label: `${getThemeLabel(game.theme) || "Unknown Theme"} · ${game.host?.trim() || "Unknown host"}`,
      })),
    ],
    [games]
  );
  const shareUrl = shareTarget === "server" ? serverUrl : `${serverUrl}/game/${shareTarget}`;
  const sharingSpecificGame = shareTarget !== "server";

  const readyWordBankOptions = useMemo(
    () => [
      {
        label: "Built-In",
        options: builtInReadyWordBanks.map((bank) => ({
          value: bank.key,
          label: getThemeLabel(bank.label),
        })),
      },
      {
        label: "Custom",
        options: customReadyWordBanks.map((bank) => ({
          value: bank.key,
          label: `${bank.label} (Custom)`,
        })),
      },
    ],
    [builtInReadyWordBanks, customReadyWordBanks]
  );

  const fetchLobbyData = useCallback(async () => {
    const fetchId = latestFetchId.current + 1;
    latestFetchId.current = fetchId;

    try {
      const [gameList, bankList] = await Promise.all([api.listGames(), api.listWordBanks()]);
      if (fetchId !== latestFetchId.current) {
        return;
      }

      setGames(gameList as GameLobby[]);
      reconcileWordBanks(bankList as WordBank[]);
      setLastSyncedAt(Date.now());
      setSyncMessage("");
    } catch {
      setSyncMessage("Trying to re-sync the lobby...");
      // ignore fetch errors; auth failures will redirect via the api client
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!getAccessToken()) {
      if (typeof window !== "undefined") {
        window.location.replace(`/login?next=${encodeURIComponent("/lobby")}`);
      } else {
        router.push("/login");
      }
      return;
    }

    void fetchLobbyData();

    const refresh = () => {
      void fetchLobbyData();
    };
    const handleVisibility = () => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    };
    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        refresh();
      }
    }, 3000);

    window.addEventListener("focus", refresh);
    window.addEventListener("online", refresh);
    document.addEventListener("visibilitychange", handleVisibility);

    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", refresh);
      window.removeEventListener("online", refresh);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [fetchLobbyData, router]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      setServerUrl(window.location.origin);
    }
  }, []);

  useEffect(() => {
    if (!showQr) {
      return;
    }

    if (games.length === 1) {
      setShareTarget(games[0].id);
      return;
    }

    if (shareTarget !== "server" && !games.some((game) => game.id === shareTarget)) {
      setShareTarget("server");
    }
  }, [games, shareTarget, showQr]);

  useEffect(() => {
    if (readyWordBanks.some((bank) => bank.key === selectedBankKey)) {
      return;
    }

    const preferred = readyWordBanks.find((bank) => bank.key === preferredWordTheme);
    const fallback = preferred
      ?? readyWordBanks.find((bank) => bank.key === DEFAULT_CREATE_THEME)
      ?? readyWordBanks[0];

    if (fallback) {
      setSelectedBankKey(fallback.key);
    }
  }, [preferredWordTheme, readyWordBanks, selectedBankKey]);

  useEffect(() => {
    if (toasts.length === 0) return;

    const timeout = window.setTimeout(() => {
      setToasts((current) => current.slice(1));
    }, 4200);

    return () => window.clearTimeout(timeout);
  }, [toasts]);

  function reconcileWordBanks(nextBanks: WordBank[]) {
    setWordBanks(nextBanks);

    for (const id of Array.from(pendingGeneratedIds.current)) {
      const bank = nextBanks.find((entry) => entry.id === id);
      if (!bank) continue;

      if (bank.status === "ready") {
        pendingGeneratedIds.current.delete(id);
        setSelectedBankKey(bank.key);
        pushToast(`"${bank.label}" is ready to use.`, "success");
      }

      if (bank.status === "failed") {
        pendingGeneratedIds.current.delete(id);
        pushToast(bank.error_message || "Word bank generation failed.", "error");
      }
    }
  }

  function pushToast(message: string, tone: ToastTone) {
    setToasts((current) => [
      ...current,
      {
        id: Date.now() + Math.floor(Math.random() * 10_000),
        message,
        tone,
      },
    ]);
  }

  async function handleCreate() {
    if (!selectedBank) {
      setCreateError("No ready word bank is available yet.");
      return;
    }

    setCreateError("");
    setCreating(true);

    try {
      const result = await api.createGame({
        theme: selectedBank.source === "builtin" ? selectedBank.key : undefined,
        word_bank_id: selectedBank.source === "custom" ? selectedBank.id : undefined,
        difficulty: newDiff,
        max_players: 4,
      });
      const id = result?.id;
      if (typeof id !== "string" || id.trim().length === 0) {
        throw new Error("Server returned an invalid game id");
      }

      if (selectedBank.source === "builtin") {
        setPreferredWordTheme(selectedBank.key as typeof preferredWordTheme);
      }

      setShowCreate(false);
      router.push(`/game/${id}`);
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : "Failed to create game");
      void fetchLobbyData();
    } finally {
      setCreating(false);
    }
  }

  async function handleJoin(id: string) {
    try {
      const result = await api.joinGame(id);
      const nextId = typeof result?.id === "string" && result.id.trim().length > 0 ? result.id : id;
      if (!nextId) {
        throw new Error("Server returned an invalid game id");
      }
      router.push(`/game/${nextId}`);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to join game");
      void fetchLobbyData();
    }
  }

  async function handleSolo() {
    try {
      const result = await api.soloGame();
      const id = result?.id;
      if (typeof id !== "string" || id.trim().length === 0) {
        throw new Error("Server returned an invalid game id");
      }
      router.push(`/game/${id}`);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to start solo game");
      void fetchLobbyData();
    }
  }

  async function handleGenerateWordBank() {
    const prompt = generationPrompt.trim();
    if (!prompt) {
      setGenerateError("Enter a word or short phrase.");
      return;
    }

    setGeneratingBank(true);
    setGenerateError("");

    try {
      const result = await api.generateWordBank(prompt);
      pendingGeneratedIds.current.add(result.id);
      setGenerationPrompt("");
      setShowGenerate(false);
      pushToast("Word bank request sent. I’ll add it to the list when it finishes.", "success");
      await fetchLobbyData();
      setShowCreate(true);
    } catch (err) {
      setGenerateError(err instanceof Error ? err.message : "Failed to generate word bank");
    } finally {
      setGeneratingBank(false);
    }
  }

  return (
    <PageContainer className="flex flex-col gap-6 py-8">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-text">Game Lobby</h1>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => setShowQr(true)}>
            <QrCode size={16} /> Share
          </Button>
          <Button variant="ghost" size="sm" onClick={fetchLobbyData}>
            <RefreshCw size={16} /> Refresh
          </Button>
        </div>
      </div>

      {(syncMessage || lastSyncedAt) && (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-border bg-bg-secondary/50 px-4 py-3 text-sm">
          <span className={syncMessage ? "text-text" : "text-text-secondary"}>
            {syncMessage || "Lobby synced"}
          </span>
          {lastSyncedAt && (
            <span className="text-xs uppercase tracking-[0.18em] text-text-secondary">
              {new Date(lastSyncedAt).toLocaleTimeString()}
            </span>
          )}
        </div>
      )}

      <div className="flex flex-wrap gap-3">
        <Button onClick={() => setShowCreate(true)}>
          <Plus size={16} /> Create Game
        </Button>
        <Button variant="secondary" onClick={handleSolo}>
          <Search size={16} /> Solo Game
        </Button>
        <Button variant="ghost" onClick={() => setShowGenerate(true)}>
          <Sparkles size={16} /> Generate Word Bank
        </Button>
      </div>

      <ActiveGamesSection
        loading={loading}
        games={games}
        onJoin={handleJoin}
        onCreate={() => setShowCreate(true)}
      />

      <WordBankLibrarySection
        wordBanks={sortedWordBanks}
        showWordBanks={showWordBanks}
        selectedBankKey={selectedBankKey}
        onToggle={() => setShowWordBanks((current) => !current)}
        onUseBank={(nextBank) => {
          setSelectedBankKey(nextBank.key);
          setShowCreate(true);
          if (nextBank.source === "builtin") {
            setPreferredWordTheme(nextBank.key as typeof preferredWordTheme);
          }
        }}
        renderStatus={renderBankStatus}
        formatError={formatWordBankError}
      />

      <CreateGameModal
        open={showCreate}
        createError={createError}
        readyWordBanks={readyWordBanks}
        readyWordBankOptions={readyWordBankOptions}
        selectedBank={selectedBank}
        newDiff={newDiff}
        creating={creating}
        difficultyOptions={difficultyOptions}
        onClose={() => setShowCreate(false)}
        onSelectBank={(nextValue) => {
          setCreateError("");
          setSelectedBankKey(nextValue);
          if (readyWordBanks.some((bank) => bank.key === nextValue && bank.source === "builtin")) {
            setPreferredWordTheme(nextValue as typeof preferredWordTheme);
          }
        }}
        onSelectDifficulty={(difficulty) => {
          setCreateError("");
          setNewDiff(difficulty);
        }}
        onCreate={handleCreate}
        onOpenGenerate={() => setShowGenerate(true)}
        formatError={formatWordBankError}
      />

      <GenerateWordBankModal
        open={showGenerate}
        error={generateError}
        prompt={generationPrompt}
        generating={generatingBank}
        onClose={() => setShowGenerate(false)}
        onChangePrompt={(value) => {
          setGenerateError("");
          setGenerationPrompt(value);
        }}
        onGenerate={handleGenerateWordBank}
      />

      <ShareLobbyModal
        open={showQr}
        shareOptions={shareOptions}
        shareTarget={shareTarget}
        shareUrl={shareUrl}
        sharingSpecificGame={sharingSpecificGame}
        onClose={() => setShowQr(false)}
        onChangeTarget={setShareTarget}
      />

      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex max-w-sm flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`rounded-xl border px-4 py-3 text-sm shadow-lg ${
              toast.tone === "error"
                ? "border-error/30 bg-error text-white"
                : "border-primary/20 bg-bg text-text"
            }`}
          >
            {toast.message}
          </div>
        ))}
      </div>
    </PageContainer>
  );
}

function renderBankStatus(bank: WordBank) {
  if (bank.status === "generating") return "Generating";
  if (bank.status === "failed") return "Needs Attention";
  if (bank.extension_state === "extending") return "Refreshing";
  return "Ready";
}

function formatWordBankError(message: string) {
  if (message.includes("Failed to launch Codex CLI")) {
    return "The external word-bank generator was unavailable for this request. Try generating it again.";
  }
  if (message.includes("theme-accurate words")) {
    return "The external generator is unavailable right now, so this word bank could not safely create new theme-accurate words.";
  }
  return message;
}

function compareWordBanks(left: WordBank, right: WordBank) {
  const leftLabel = left.source === "custom" ? left.label : getThemeLabel(left.label);
  const rightLabel = right.source === "custom" ? right.label : getThemeLabel(right.label);
  return bankLabelCollator.compare(leftLabel, rightLabel);
}
