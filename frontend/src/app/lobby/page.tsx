"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Search, Plus, Loader2, QrCode, RefreshCw } from "lucide-react";
import { QRCodeSVG } from "qrcode.react";
import { PageContainer } from "@/components/layout";
import { Button, Card, Badge, Modal, Select } from "@/components/ui";
import { api, getAccessToken } from "@/lib/api";
import type { GameLobby, Difficulty, WordTheme } from "@/types";

const themeLabels: Record<WordTheme, string> = {
  animals: "Animals", sports: "Sports", food: "Food", movies: "Movies",
  science: "Science", geography: "Geography", technology: "Technology",
  music: "Music", history: "History", nature: "Nature",
};

const diffLabels: Record<Difficulty, string> = {
  easy: "Easy (8x8)", medium: "Medium (12x12)", hard: "Hard (15x15)", expert: "Expert (20x20)",
};

const themeOptions = Object.entries(themeLabels).map(([value, label]) => ({ value, label }));
const diffOptions = Object.entries(diffLabels).map(([value, label]) => ({ value, label }));

export default function LobbyPage() {
  const router = useRouter();
  const [games, setGames] = useState<GameLobby[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [showQr, setShowQr] = useState(false);
  const [newTheme, setNewTheme] = useState<WordTheme>("science");
  const [newDiff, setNewDiff] = useState<Difficulty>("medium");
  const [serverUrl, setServerUrl] = useState("");

  const fetchGames = useCallback(async () => {
    try {
      const list = await api.listGames();
      setGames(list as GameLobby[]);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!getAccessToken()) {
      router.push("/login");
      return;
    }
    fetchGames();
    const interval = setInterval(fetchGames, 5000);
    return () => clearInterval(interval);
  }, [fetchGames, router]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      setServerUrl(window.location.origin);
    }
  }, []);

  async function handleCreate() {
    setCreating(true);
    try {
      const { id } = await api.createGame({
        theme: newTheme,
        difficulty: newDiff,
        max_players: 4,
      });
      setShowCreate(false);
      router.push(`/game/${id}`);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to create game");
    } finally {
      setCreating(false);
    }
  }

  async function handleJoin(id: string) {
    try {
      await api.joinGame(id);
      router.push(`/game/${id}`);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to join game");
    }
  }

  async function handleSolo() {
    try {
      const { id } = await api.soloGame();
      router.push(`/game/${id}`);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to start solo game");
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
          <Button variant="ghost" size="sm" onClick={fetchGames}>
            <RefreshCw size={16} /> Refresh
          </Button>
        </div>
      </div>

      {/* Quick actions */}
      <div className="flex flex-wrap gap-3">
        <Button onClick={() => setShowCreate(true)}>
          <Plus size={16} /> Create Game
        </Button>
        <Button variant="secondary" onClick={handleSolo}>
          <Search size={16} /> Solo Game
        </Button>
      </div>

      {/* Game list */}
      {loading ? (
        <div className="flex items-center justify-center py-12 text-text-secondary">
          <Loader2 className="animate-spin mr-2" size={20} /> Loading games...
        </div>
      ) : games.length === 0 ? (
        <Card className="flex flex-col items-center gap-3 py-12 text-center">
          <Search size={40} className="text-text-secondary" />
          <p className="text-text-secondary">No active games right now</p>
          <p className="text-sm text-text-secondary">Create one and invite your friends!</p>
        </Card>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {games.map((g) => (
            <Card key={g.id} hoverable onClick={() => handleJoin(g.id)}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Badge variant="primary">{themeLabels[g.theme]}</Badge>
                  <Badge>{diffLabels[g.difficulty]}</Badge>
                </div>
                <span className="text-sm text-text-secondary">
                  {g.player_count}/{g.max_players}
                </span>
              </div>
              <p className="mt-2 text-xs text-text-secondary">
                Host: {g.host}
              </p>
            </Card>
          ))}
        </div>
      )}

      {/* Create game modal */}
      <Modal open={showCreate} onClose={() => setShowCreate(false)} title="Create Game">
        <div className="flex flex-col gap-4">
          <Select
            label="Word Theme"
            options={themeOptions}
            value={newTheme}
            onChange={(e) => setNewTheme(e.target.value as WordTheme)}
          />
          <Select
            label="Difficulty"
            options={diffOptions}
            value={newDiff}
            onChange={(e) => setNewDiff(e.target.value as Difficulty)}
          />
          <div className="flex gap-2 mt-2">
            <Button onClick={handleCreate} loading={creating} className="flex-1">
              Create
            </Button>
            <Button variant="ghost" onClick={() => setShowCreate(false)}>
              Cancel
            </Button>
          </div>
        </div>
      </Modal>

      {/* QR code modal */}
      <Modal open={showQr} onClose={() => setShowQr(false)} title="Share Server">
        <div className="flex flex-col items-center gap-4">
          {serverUrl && <QRCodeSVG value={serverUrl} size={180} />}
          <p className="text-sm text-text-secondary text-center">
            Scan to open this server on another device
          </p>
          <p className="text-xs font-mono text-text-secondary">{serverUrl}</p>
        </div>
      </Modal>
    </PageContainer>
  );
}
