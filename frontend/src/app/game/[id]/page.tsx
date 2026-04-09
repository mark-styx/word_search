"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import { ArrowLeft, Crown, Send, Loader2 } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button, Badge, Timer, Card } from "@/components/ui";
import { api, getAccessToken } from "@/lib/api";
import { GameSocket } from "@/lib/ws";
import type { GameState, ServerMessage, PlacedWord } from "@/types";

type CellCoord = { row: number; col: number };

export default function GamePage() {
  const params = useParams();
  const router = useRouter();
  const gameId = params.id as string;
  const [game, setGame] = useState<GameState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selectedCells, setSelectedCells] = useState<CellCoord[]>([]);
  const [foundByMe, setFoundByMe] = useState<Set<string>>(new Set());
  const [foundByOthers, setFoundByOthers] = useState<Set<string>>(new Set());
  const [chatMessages, setChatMessages] = useState<{ from: string; text: string }[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [selecting, setSelecting] = useState(false);
  const [startCell, setStartCell] = useState<CellCoord | null>(null);
  const socketRef = useRef<GameSocket | null>(null);
  const gridRef = useRef<HTMLDivElement>(null);

  const token = getAccessToken() ?? "";

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as ServerMessage;
    if (!msg || typeof msg !== "object") return;

    if ("type" in msg) {
      switch (msg.type) {
        case "game_starting":
        case "player_joined":
        case "player_left":
        case "player_ready":
        case "player_score_update":
          // Refresh game state
          api.getGame(gameId).then((g) => setGame(g as GameState)).catch(() => {});
          break;
        case "word_found":
          if ("player_id" in msg && "word" in msg && "score" in msg) {
            const foundMsg = msg as { player_id: string; word: string; score: number };
            const myId = Object.keys(game?.players ?? {})[0]; // rough check
            if (foundMsg.player_id === myId) {
              setFoundByMe((prev) => new Set(prev).add(foundMsg.word));
            } else {
              setFoundByOthers((prev) => new Set(prev).add(foundMsg.word));
            }
            api.getGame(gameId).then((g) => setGame(g as GameState)).catch(() => {});
          }
          break;
        case "game_finished":
          api.getGame(gameId).then((g) => setGame(g as GameState)).catch(() => {});
          break;
        case "chat_message":
          if ("username" in msg && "text" in msg) {
            setChatMessages((prev) => [
              ...prev,
              { from: (msg as { username: string; text: string }).username, text: (msg as { username: string; text: string }).text },
            ]);
          }
          break;
        case "error":
          if ("message" in msg) {
            setError((msg as { message: string }).message);
            setTimeout(() => setError(""), 3000);
          }
          break;
      }
    }
  }, [gameId, game]);

  useEffect(() => {
    if (!token) {
      router.push("/login");
      return;
    }

    let ws: GameSocket | null = null;

    async function init() {
      try {
        const g = await api.getGame(gameId);
        setGame(g as GameState);
        const wsBase = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";
        ws = new GameSocket({
          url: wsBase,
          token,
          gameId,
          onMessage: handleMessage,
        });
        socketRef.current = ws;
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load game");
      } finally {
        setLoading(false);
      }
    }

    init();
    return () => {
      ws?.close();
    };
  }, [gameId, token, router, handleMessage]);

  // Grid pointer selection
  const getCellFromPoint = useCallback((clientX: number, clientY: number): CellCoord | null => {
    if (!gridRef.current) return null;
    const el = document.elementFromPoint(clientX, clientY) as HTMLElement;
    const row = el?.dataset?.row;
    const col = el?.dataset?.col;
    if (row != null && col != null) return { row: +row, col: +col };
    return null;
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    const cell = getCellFromPoint(e.clientX, e.clientY);
    if (!cell) return;
    setSelecting(true);
    setStartCell(cell);
    setSelectedCells([cell]);
    (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
  }, [getCellFromPoint]);

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!selecting || !startCell) return;
    const cell = getCellFromPoint(e.clientX, e.clientY);
    if (!cell) return;
    // Build line from start to current cell (straight lines only)
    const dr = cell.row - startCell.row;
    const dc = cell.col - startCell.col;
    const steps = Math.max(Math.abs(dr), Math.abs(dc));
    if (steps === 0) {
      setSelectedCells([startCell]);
      return;
    }
    const stepR = dr === 0 ? 0 : dr / Math.abs(dr);
    const stepC = dc === 0 ? 0 : dc / Math.abs(dc);
    // Only allow valid directions (horizontal, vertical, diagonal)
    if (dr !== 0 && dc !== 0 && Math.abs(dr) !== Math.abs(dc)) return;
    const cells: CellCoord[] = [];
    for (let i = 0; i <= steps; i++) {
      cells.push({ row: startCell.row + i * stepR, col: startCell.col + i * stepC });
    }
    setSelectedCells(cells);
  }, [selecting, startCell, getCellFromPoint]);

  const handlePointerUp = useCallback(() => {
    if (!selecting) return;
    setSelecting(false);
    if (selectedCells.length > 1) {
      socketRef.current?.send({
        type: "select_cells",
        cells: selectedCells,
      });
    }
    setSelectedCells([]);
    setStartCell(null);
  }, [selecting, selectedCells]);

  const sendChat = useCallback(() => {
    if (!chatInput.trim()) return;
    socketRef.current?.send({ type: "chat", text: chatInput.trim() });
    setChatInput("");
  }, [chatInput]);

  const handleStart = useCallback(async () => {
    try {
      await api.startGame(gameId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start");
    }
  }, [gameId]);

  const handleReady = useCallback(() => {
    socketRef.current?.send({ type: "ready" });
  }, []);

  // --- RENDER ---

  if (loading) {
    return (
      <PageContainer className="flex items-center justify-center">
        <Loader2 className="animate-spin mr-2" size={24} /> Loading game...
      </PageContainer>
    );
  }

  if (error && !game) {
    return (
      <PageContainer className="flex flex-col items-center justify-center gap-4">
        <p className="text-error">{error}</p>
        <Button onClick={() => router.push("/lobby")}>Back to Lobby</Button>
      </PageContainer>
    );
  }

  if (!game) return null;

  const grid = game.grid;
  const players = Object.values(game.players);
  const myId = Object.keys(game.players)[0]; // simplified

  return (
    <PageContainer className="flex flex-col gap-4 py-4">
      {/* Top bar */}
      <div className="flex items-center justify-between">
        <button
          onClick={() => router.push("/lobby")}
          className="rounded-lg p-2 text-text-secondary hover:bg-bg-secondary"
        >
          <ArrowLeft size={20} />
        </button>
        <div className="flex items-center gap-2">
          <Badge variant="primary" className="capitalize">{game.theme}</Badge>
          <Badge className="uppercase">{game.difficulty}</Badge>
        </div>
        {game.time_limit && game.status === "in_progress" && (
          <Timer seconds={game.time_limit} className="w-32" />
        )}
        {!game.time_limit && <div className="w-32" />}
      </div>

      {error && (
        <div className="rounded-lg bg-error/10 px-3 py-2 text-sm text-error">{error}</div>
      )}

      {/* Lobby state */}
      {game.status === "lobby" && (
        <div className="flex flex-col items-center gap-6 py-8">
          <h2 className="text-xl font-bold text-text">Waiting for players...</h2>
          <div className="flex flex-wrap gap-3">
            {players.map((p) => (
              <Card key={p.id} className="flex items-center gap-2 px-4">
                {p.is_host && <Crown size={14} className="text-yellow-500" />}
                <span className="font-medium text-text">{p.username}</span>
              </Card>
            ))}
          </div>
          <div className="flex gap-3">
            <Button onClick={handleReady}>Ready</Button>
            {players[0]?.is_host && (
              <Button onClick={handleStart}>Start Game</Button>
            )}
          </div>
        </div>
      )}

      {/* In progress */}
      {game.status === "in_progress" && grid && (
        <div className="flex flex-col gap-4 lg:flex-row">
          {/* Grid */}
          <div className="flex-1 flex justify-center">
            <div
              ref={gridRef}
              className="inline-grid gap-0.5 select-none touch-none"
              style={{
                gridTemplateColumns: `repeat(${grid.width}, minmax(0, 1fr))`,
              }}
              onPointerDown={handlePointerDown}
              onPointerMove={handlePointerMove}
              onPointerUp={handlePointerUp}
            >
              {grid.cells.map((row, r) =>
                row.map((cell, c) => {
                  const isSelected = selectedCells.some(
                    (s) => s.row === r && s.col === c
                  );
                  const wordKey = `${r},${c}`;
                  const found = foundByMe.has(wordKey) || foundByOthers.has(wordKey);
                  return (
                    <motion.div
                      key={`${r}-${c}`}
                      data-row={r}
                      data-col={c}
                      className={`flex items-center justify-center rounded text-xs font-bold sm:text-sm md:text-base aspect-square min-w-[28px] ${
                        isSelected
                          ? "bg-cell-selected text-white scale-105"
                          : found
                          ? "bg-cell-found text-white"
                          : "bg-cell-bg text-text border border-border"
                      }`}
                      animate={isSelected ? { scale: 1.05 } : { scale: 1 }}
                      transition={{ duration: 0.1 }}
                    >
                      {cell}
                    </motion.div>
                  );
                })
              )}
            </div>
          </div>

          {/* Sidebar: scores + words + chat */}
          <div className="w-full lg:w-64 flex flex-col gap-4">
            {/* Scores */}
            <div className="flex flex-col gap-2">
              <h3 className="text-sm font-semibold text-text-secondary uppercase">Scores</h3>
              {players
                .sort((a, b) => b.score - a.score)
                .map((p) => (
                  <div key={p.id} className="flex items-center justify-between rounded-lg bg-bg-secondary px-3 py-2">
                    <span className="text-sm font-medium text-text">{p.username}</span>
                    <span className="text-sm font-bold text-primary">{p.score}</span>
                  </div>
                ))}
            </div>

            {/* Word list */}
            <div className="flex flex-col gap-1">
              <h3 className="text-sm font-semibold text-text-secondary uppercase">Words</h3>
              <div className="flex flex-wrap gap-1">
                {grid.placed_words.map((pw: PlacedWord, i: number) => {
                  const found = foundByMe.has(pw.word) || foundByOthers.has(pw.word);
                  return (
                    <span
                      key={i}
                      className={`text-xs px-2 py-0.5 rounded ${
                        found
                          ? "bg-cell-found/20 text-cell-found line-through"
                          : "bg-bg-secondary text-text-secondary"
                      }`}
                    >
                      {pw.word}
                    </span>
                  );
                })}
              </div>
            </div>

            {/* Chat */}
            <div className="flex flex-col gap-1 flex-1 min-h-0">
              <h3 className="text-sm font-semibold text-text-secondary uppercase">Chat</h3>
              <div className="flex flex-col gap-1 max-h-40 overflow-y-auto">
                <AnimatePresence>
                  {chatMessages.map((m, i) => (
                    <motion.div
                      key={i}
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      className="text-xs"
                    >
                      <span className="font-medium text-primary">{m.from}: </span>
                      <span className="text-text-secondary">{m.text}</span>
                    </motion.div>
                  ))}
                </AnimatePresence>
              </div>
              <div className="flex gap-1">
                <input
                  value={chatInput}
                  onChange={(e) => setChatInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && sendChat()}
                  placeholder="Type a message..."
                  className="flex-1 rounded bg-bg-secondary border border-border px-2 py-1 text-xs text-text focus:outline-none focus:ring-1 focus:ring-primary"
                />
                <button
                  onClick={sendChat}
                  className="rounded bg-primary p-1 text-white hover:bg-primary-hover"
                >
                  <Send size={12} />
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Finished */}
      {game.status === "finished" && (
        <div className="flex flex-col items-center gap-6 py-8">
          <h2 className="text-2xl font-bold text-text">Game Over!</h2>
          <div className="flex flex-col gap-2 w-full max-w-sm">
            {players
              .sort((a, b) => b.score - a.score)
              .map((p, i) => (
                <Card key={p.id} className={`flex items-center justify-between ${i === 0 ? "border-primary" : ""}`}>
                  <div className="flex items-center gap-2">
                    <span className="text-lg font-bold text-primary">#{i + 1}</span>
                    <span className="font-medium text-text">{p.username}</span>
                  </div>
                  <span className="font-bold text-text">{p.score} pts</span>
                </Card>
              ))}
          </div>
          <Button onClick={() => router.push("/lobby")}>Back to Lobby</Button>
        </div>
      )}
    </PageContainer>
  );
}
