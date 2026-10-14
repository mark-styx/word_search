"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import { ArrowLeft, Loader2 } from "lucide-react";
import { PageContainer } from "@/components/layout";
import GameBoard from "@/components/board/GameBoard";
import ChatPanel from "@/components/board/ChatPanel";
import GameInfoTabs from "@/components/board/GameInfoTabs";
import LobbyRoster from "@/components/board/LobbyRoster";
import ScoreboardPanel from "@/components/board/ScoreboardPanel";
import WordBankPanel from "@/components/board/WordBankPanel";
import { Button, Badge, Timer, Card } from "@/components/ui";
import { api, getAccessToken, getApiBaseUrl, getCurrentUserId } from "@/lib/api";
import { readableTextColor, darken, lighten, normalizeHexColor, withAlpha } from "@/lib/colors";
import { getThemeLabel } from "@/lib/game-themes";
import { scoreWord } from "@/lib/scoring";
import { useTheme } from "@/lib/themes-provider";
import { GameSocket } from "@/lib/ws";
import type { GameState, ServerMessage, WordDirection } from "@/types";

type CellCoord = { row: number; col: number };
type BoundaryBreaks = {
  top: boolean;
  right: boolean;
  bottom: boolean;
  left: boolean;
};

const DIRECTION_STEPS: Record<WordDirection, [number, number]> = {
  horizontal: [0, 1],
  vertical: [1, 0],
  diagonal_down_right: [1, 1],
  diagonal_down_left: [1, -1],
  reverse_horizontal: [0, -1],
  reverse_vertical: [-1, 0],
  reverse_diagonal_down_right: [-1, -1],
  reverse_diagonal_down_left: [-1, 1],
};

const PLAYER_COLOR_FALLBACKS = [
  "#0891b2",
  "#16a34a",
  "#d97706",
  "#7c3aed",
  "#dc2626",
  "#0f766e",
];

function hashString(value: string) {
  let hash = 0;
  for (const char of value) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return hash;
}

function hashToPaletteIndex(value: string) {
  return hashString(value) % PLAYER_COLOR_FALLBACKS.length;
}

function getPlayerColor(
  playerId: string,
  playersById: Map<string, GameState["players"][number]>,
  currentUserId: string | null,
  currentUserColor: string
) {
  if (playerId === currentUserId) {
    return currentUserColor;
  }
  const syncedColor = playersById.get(playerId)?.highlight_color;
  if (syncedColor) {
    return syncedColor;
  }
  return PLAYER_COLOR_FALLBACKS[hashToPaletteIndex(playerId)];
}

function buildCellGradient(color: string, seed: string) {
  const angle = 115 + (hashString(seed) % 70);
  return `linear-gradient(${angle}deg, ${lighten(color, 0.16)} 0%, ${color} 50%, ${darken(color, 0.1)} 100%)`;
}

function buildStackedMonoHighlight(color: string, overlapCount: number, seed: string) {
  const angle = 108 + (hashString(seed) % 72);
  const deep = darken(color, Math.min(0.18 + (overlapCount - 2) * 0.07, 0.36));
  const deepest = darken(color, Math.min(0.26 + (overlapCount - 2) * 0.08, 0.48));
  return `linear-gradient(${angle}deg, ${lighten(color, 0.18)} 0%, ${color} 34%, ${deep} 66%, ${deepest} 100%)`;
}

function getBoundaryBreaks(
  row: number,
  col: number,
  currentWordKeys: string[],
  wordOwnership: Map<string, string[]>
): BoundaryBreaks {
  const hasBoundaryBreak = (nextRow: number, nextCol: number) => {
    const neighborWordKeys = wordOwnership.get(`${nextRow},${nextCol}`) ?? [];
    if (neighborWordKeys.length === 0) {
      return false;
    }

    return currentWordKeys.every((wordKey) => !neighborWordKeys.includes(wordKey));
  };

  return {
    top: hasBoundaryBreak(row - 1, col),
    right: hasBoundaryBreak(row, col + 1),
    bottom: hasBoundaryBreak(row + 1, col),
    left: hasBoundaryBreak(row, col - 1),
  };
}

function buildBoundaryOverlay(baseBackground: string, color: string, breaks: BoundaryBreaks) {
  const overlays: string[] = [];

  if (breaks.top) {
    overlays.push(
      `linear-gradient(180deg, ${withAlpha(lighten(color, 0.32), 0.92)} 0%, ${withAlpha(lighten(color, 0.18), 0.46)} 16%, transparent 30%)`
    );
  }
  if (breaks.right) {
    overlays.push(
      `linear-gradient(270deg, ${withAlpha(darken(color, 0.34), 0.28)} 0%, transparent 24%)`
    );
  }
  if (breaks.bottom) {
    overlays.push(
      `linear-gradient(0deg, ${withAlpha(darken(color, 0.3), 0.26)} 0%, transparent 24%)`
    );
  }
  if (breaks.left) {
    overlays.push(
      `linear-gradient(90deg, ${withAlpha(lighten(color, 0.24), 0.7)} 0%, transparent 24%)`
    );
  }

  return overlays.length > 0 ? `${overlays.join(", ")}, ${baseBackground}` : baseBackground;
}

export default function GamePage() {
  const params = useParams();
  const router = useRouter();
  const gameId = params.id as string;
  const [game, setGame] = useState<GameState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selectedCells, setSelectedCells] = useState<CellCoord[]>([]);
  const [chatMessages, setChatMessages] = useState<{ from: string; text: string }[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [selecting, setSelecting] = useState(false);
  const [startCell, setStartCell] = useState<CellCoord | null>(null);
  const socketRef = useRef<GameSocket | null>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const gameRef = useRef<GameState | null>(null);
  const latestRevisionRef = useRef<number>(0);

  const token = getAccessToken() ?? "";
  const currentUserId = getCurrentUserId();
  const { highlightColor } = useTheme();
  const [socketConnected, setSocketConnected] = useState(false);
  const [syncMessage, setSyncMessage] = useState("");
  const [activeMobilePanel, setActiveMobilePanel] = useState<"scores" | "words" | "chat">("words");

  const applyGameState = useCallback((nextGame: GameState) => {
    const revision = typeof nextGame.revision === "number" ? nextGame.revision : 0;
    if (revision < latestRevisionRef.current) {
      return null;
    }

    latestRevisionRef.current = revision;
    setGame(nextGame);
    setSyncMessage("");
    return nextGame;
  }, []);

  const patchGameState = useCallback((updater: (current: GameState) => GameState) => {
    setGame((current) => {
      if (!current) {
        return current;
      }

      const nextGame = updater(current);
      gameRef.current = nextGame;
      return nextGame;
    });
  }, []);

  const loadGame = useCallback(async () => {
    const nextGame = await api.getGame(gameId);
    return applyGameState(nextGame as GameState);
  }, [applyGameState, gameId]);

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as ServerMessage;
    if (!msg || typeof msg !== "object" || !("type" in msg)) return;

    switch (msg.type) {
      case "game_state_sync":
        applyGameState(msg.game);
        break;
      case "game_starting":
      case "player_joined":
      case "player_left":
        break;
      case "player_ready":
        patchGameState((current) => ({
          ...current,
          players: current.players.map((player) =>
            player.id === msg.player_id ? { ...player, ready: true } : player
          ),
        }));
        break;
      case "player_score_update":
        patchGameState((current) => ({
          ...current,
          players: current.players.map((player) =>
            player.id === msg.player_id ? { ...player, score: msg.score } : player
          ),
        }));
        break;
      case "word_found":
        patchGameState((current) => {
          const existingWords = current.found_words[msg.player_id] ?? [];
          if (existingWords.includes(msg.word)) {
            return current;
          }

          return {
            ...current,
            found_words: {
              ...current.found_words,
              [msg.player_id]: [...existingWords, msg.word],
            },
          };
        });
        if (msg.player_id === currentUserId && typeof navigator !== "undefined" && "vibrate" in navigator) {
          navigator.vibrate?.([18, 20, 32]);
        }
        break;
      case "game_finished":
        patchGameState((current) => ({
          ...current,
          status: "finished",
          finished_reason: msg.reason,
          players: msg.rankings,
        }));
        break;
      case "chat_message": {
        const username =
          gameRef.current?.players.find((player) => player.id === msg.player_id)?.username ||
          "Player";
        setChatMessages((prev) => [...prev, { from: username, text: msg.message }]);
        break;
      }
      case "error":
        setError(msg.message);
        setTimeout(() => setError(""), 3000);
        break;
    }
  }, [applyGameState, currentUserId, patchGameState]);

  useEffect(() => {
    if (!token) {
      if (typeof window !== "undefined") {
        const next = `${window.location.pathname}${window.location.search}`;
        window.location.replace(`/login?next=${encodeURIComponent(next)}`);
      } else {
        router.push("/login");
      }
      return;
    }

    let ws: GameSocket | null = null;

    async function init() {
      try {
        let currentGame = await loadGame();
        if (
          currentGame &&
          currentGame.status === "lobby" &&
          currentUserId &&
          !currentGame.players.some((player) => player.id === currentUserId)
        ) {
          await api.joinGame(gameId);
          currentGame = await loadGame();
        }
        ws = new GameSocket({
          url: getApiBaseUrl(),
          token,
          gameId,
          onMessage: handleMessage,
          onOpen: () => {
            setSocketConnected(true);
            setSyncMessage("");
          },
          onClose: () => {
            setSocketConnected(false);
            setSyncMessage("Re-syncing game state...");
          },
          onError: () => {
            setSyncMessage("Re-syncing game state...");
          },
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
  }, [gameId, token, router, handleMessage, loadGame]);

  useEffect(() => {
    if (!token) return;

    const reconnect = () => {
      socketRef.current?.reconnectNow();
    };

    window.addEventListener("online", reconnect);
    window.addEventListener("focus", reconnect);

    return () => {
      window.removeEventListener("online", reconnect);
      window.removeEventListener("focus", reconnect);
    };
  }, [token]);

  const getCellFromPoint = useCallback((clientX: number, clientY: number): CellCoord | null => {
    if (!gridRef.current) return null;
    const el = document.elementFromPoint(clientX, clientY) as HTMLElement;
    const row = el?.dataset?.row;
    const col = el?.dataset?.col;
    if (row != null && col != null) return { row: Number(row), col: Number(col) };
    return null;
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    const cell = getCellFromPoint(e.clientX, e.clientY);
    if (!cell) return;
    setSelecting(true);
    setStartCell(cell);
    setSelectedCells([cell]);
    e.currentTarget.setPointerCapture?.(e.pointerId);
  }, [getCellFromPoint]);

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!selecting || !startCell) return;
    const cell = getCellFromPoint(e.clientX, e.clientY);
    if (!cell) return;

    const dr = cell.row - startCell.row;
    const dc = cell.col - startCell.col;
    const steps = Math.max(Math.abs(dr), Math.abs(dc));
    if (steps === 0) {
      setSelectedCells([startCell]);
      return;
    }

    const stepR = dr === 0 ? 0 : dr / Math.abs(dr);
    const stepC = dc === 0 ? 0 : dc / Math.abs(dc);
    if (dr !== 0 && dc !== 0 && Math.abs(dr) !== Math.abs(dc)) return;

    const cells: CellCoord[] = [];
    for (let i = 0; i <= steps; i++) {
      cells.push({ row: startCell.row + i * stepR, col: startCell.col + i * stepC });
    }
    setSelectedCells(cells);
  }, [selecting, startCell, getCellFromPoint]);

  const handlePointerEnd = useCallback((submitSelection: boolean) => {
    if (!selecting) return;

    if (submitSelection && selectedCells.length > 1) {
      socketRef.current?.send({
        type: "select_cells",
        cells: selectedCells.map(({ row, col }) => [row, col]),
      });
    }

    setSelecting(false);
    setSelectedCells([]);
    setStartCell(null);
  }, [selecting, selectedCells]);

  const handlePointerUp = useCallback(() => {
    handlePointerEnd(true);
  }, [handlePointerEnd]);

  const handlePointerCancel = useCallback(() => {
    handlePointerEnd(false);
  }, [handlePointerEnd]);

  const sendChat = useCallback(() => {
    if (!chatInput.trim()) return;
    socketRef.current?.send({ type: "chat", message: chatInput.trim() });
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
  const players = game.players;
  const playersById = new Map(players.map((player) => [player.id, player]));
  const me = players.find((player) => player.id === currentUserId) ?? null;
  const foundWords = new Set(Object.values(game.found_words).flat());
  const myFoundWords = new Set(currentUserId ? game.found_words[currentUserId] ?? [] : []);
  const totalWords = grid?.placed_words.length ?? 0;
  const foundWordCount = foundWords.size;
  const myFoundCount = currentUserId ? game.found_words[currentUserId]?.length ?? 0 : 0;
  const completion = totalWords > 0 ? Math.round((foundWordCount / totalWords) * 100) : 0;
  const finishSummary =
    game.finished_reason === "timeout"
      ? "Time expired before the board was fully cleared."
      : game.finished_reason === "abandoned"
        ? "Everyone left the game before it was completed."
        : "Review the whole board before heading back.";
  const boardPointTotal = grid
    ? grid.placed_words.reduce((sum, placedWord) => sum + scoreWord(placedWord.word, game.difficulty), 0)
    : 0;
  const foundCellOwners = new Map<string, string[]>();
  const foundCellWordKeys = new Map<string, string[]>();
  const foundWordOwners = new Map<string, string>();

  if (grid) {
    for (const [playerId, words] of Object.entries(game.found_words)) {
      for (const word of words) {
        if (!foundWordOwners.has(word) || playerId === currentUserId) {
          foundWordOwners.set(word, playerId);
        }
      }
    }

    for (const placedWord of grid.placed_words) {
      if (!foundWords.has(placedWord.word)) continue;

      const ownerId = foundWordOwners.get(placedWord.word);
      if (!ownerId) continue;
      const wordKey = `${placedWord.word}:${placedWord.start[0]},${placedWord.start[1]}:${placedWord.direction}`;

      const [dr, dc] = DIRECTION_STEPS[placedWord.direction];
      for (let i = 0; i < placedWord.word.length; i++) {
        const key = `${placedWord.start[0] + dr * i},${placedWord.start[1] + dc * i}`;
        const owners = foundCellOwners.get(key) ?? [];
        if (!owners.includes(ownerId)) {
          owners.push(ownerId);
          foundCellOwners.set(key, owners);
        }

        const wordKeys = foundCellWordKeys.get(key) ?? [];
        if (!wordKeys.includes(wordKey)) {
          wordKeys.push(wordKey);
          foundCellWordKeys.set(key, wordKeys);
        }
      }
    }
  }

  const boardWords = grid
    ? [...grid.placed_words].sort((a, b) => {
        const aFound = foundWords.has(a.word) ? 1 : 0;
        const bFound = foundWords.has(b.word) ? 1 : 0;
        if (aFound !== bFound) return bFound - aFound;
        return a.word.localeCompare(b.word);
      })
    : [];
  const rankedPlayers = [...players].sort(
    (a, b) => Number(b.connected) - Number(a.connected) || b.score - a.score
  );
  const resolvePlayerColor = (playerId: string) =>
    getPlayerColor(playerId, playersById, currentUserId, highlightColor);
  const interactiveBoard = game.status === "in_progress";
  const boardPointerHandlers = interactiveBoard
    ? {
        onPointerDown: handlePointerDown,
        onPointerMove: handlePointerMove,
        onPointerUp: handlePointerUp,
        onPointerCancel: handlePointerCancel,
        onLostPointerCapture: handlePointerCancel,
        onContextMenu: (event: React.MouseEvent) => event.preventDefault(),
        onDragStart: (event: React.DragEvent) => event.preventDefault(),
      }
    : {};
  const boardView = grid ? (
    <GameBoard
      grid={grid}
      selectedCells={selectedCells}
      foundWords={foundWords}
      foundCellOwners={foundCellOwners}
      foundCellWordKeys={foundCellWordKeys}
      playersById={playersById}
      currentUserId={currentUserId}
      highlightColor={highlightColor}
      interactive={interactiveBoard}
      gridRef={gridRef}
      boardPointerHandlers={boardPointerHandlers}
      getPlayerColor={getPlayerColor}
      buildCellGradient={buildCellGradient}
      buildStackedMonoHighlight={buildStackedMonoHighlight}
      getBoundaryBreaks={getBoundaryBreaks}
      buildBoundaryOverlay={buildBoundaryOverlay}
    />
  ) : null;

  return (
    <PageContainer
      className={`game-shell flex flex-col gap-3 py-3 ${interactiveBoard ? "game-shell--interactive" : ""}`}
    >
      <div className="flex items-center justify-between">
        <button
          onClick={() => router.push("/lobby")}
          className="flex min-h-11 min-w-11 items-center justify-center rounded-lg p-2 text-text-secondary hover:bg-bg-secondary"
        >
          <ArrowLeft size={20} />
        </button>
        <div className="flex items-center gap-2">
          <Badge variant="primary">{getThemeLabel(game.theme)}</Badge>
          <Badge className="uppercase">{game.difficulty}</Badge>
        </div>
        {game.time_limit_secs && game.status === "in_progress" && (
          <Timer seconds={game.time_limit_secs} className="w-32" />
        )}
        {!game.time_limit_secs && <div className="w-32" />}
      </div>

      {error && (
        <div className="rounded-lg bg-error/10 px-3 py-2 text-sm text-error">{error}</div>
      )}

      {(syncMessage || (!socketConnected && game.status !== "finished")) && (
        <div className="rounded-lg border border-border bg-bg-secondary/60 px-3 py-2 text-sm text-text-secondary">
          {syncMessage || "Waiting for live updates... checking server state."}
        </div>
      )}

      {game.status === "lobby" && (
        <LobbyRoster
          players={players}
          currentPlayer={me}
          onReady={handleReady}
          onStart={handleStart}
        />
      )}

      {game.status === "in_progress" && grid && (
        <div className="flex flex-col gap-2 lg:flex-row lg:items-start">
          <div className="flex flex-1 flex-col gap-2">
            {boardView}

            <div className="lg:hidden -mt-0.5">
              <GameInfoTabs active={activeMobilePanel} onChange={setActiveMobilePanel} />
            </div>
          </div>

          <div className="w-full lg:w-80">
            <div className="hidden flex-col gap-3 lg:flex">
              <ScoreboardPanel players={rankedPlayers} />
              <WordBankPanel
                words={grid.placed_words}
                foundWords={foundWords}
                myFoundWords={myFoundWords}
                foundWordOwners={foundWordOwners}
                playersById={playersById}
                currentUserId={currentUserId}
                difficulty={game.difficulty}
                resolvePlayerColor={resolvePlayerColor}
              />
              <ChatPanel
                messages={chatMessages}
                value={chatInput}
                onChange={setChatInput}
                onSend={sendChat}
              />
            </div>

            <div className="lg:hidden">
              {activeMobilePanel === "scores" && <ScoreboardPanel players={rankedPlayers} />}
              {activeMobilePanel === "words" && (
                <WordBankPanel
                  words={grid.placed_words}
                  foundWords={foundWords}
                  myFoundWords={myFoundWords}
                  foundWordOwners={foundWordOwners}
                  playersById={playersById}
                  currentUserId={currentUserId}
                  difficulty={game.difficulty}
                  resolvePlayerColor={resolvePlayerColor}
                />
              )}
              {activeMobilePanel === "chat" && (
                <ChatPanel
                  messages={chatMessages}
                  value={chatInput}
                  onChange={setChatInput}
                  onSend={sendChat}
                />
              )}
            </div>
          </div>
        </div>
      )}

      {game.status === "finished" && (
        <div className="flex flex-col gap-6 py-4">
        <div className="flex flex-col items-center gap-2 text-center">
          <h2 className="text-2xl font-bold text-text">Game Over!</h2>
          <p className="text-sm text-text-secondary">{finishSummary}</p>
        </div>

          <div className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_360px]">
            <div className="flex flex-col gap-4">
              <div className="grid gap-3 sm:grid-cols-3">
                <Card className="px-4 py-4">
                  <div className="text-xs uppercase tracking-[0.18em] text-text-secondary">Completion</div>
                  <div className="mt-1 text-2xl font-bold text-text">{completion}%</div>
                  <div className="text-sm text-text-secondary">{foundWordCount}/{totalWords} words found</div>
                </Card>
                <Card className="px-4 py-4">
                  <div className="text-xs uppercase tracking-[0.18em] text-text-secondary">You Found</div>
                  <div className="mt-1 text-2xl font-bold text-text">{myFoundCount}</div>
                  <div className="text-sm text-text-secondary">Words with your highlight</div>
                </Card>
                <Card className="px-4 py-4">
                  <div className="text-xs uppercase tracking-[0.18em] text-text-secondary">Board Value</div>
                  <div className="mt-1 text-2xl font-bold text-text">{boardPointTotal}</div>
                  <div className="text-sm text-text-secondary">Total points on the board</div>
                </Card>
              </div>

              {grid && (
                <Card className="px-4 py-4 sm:px-5">
                  <div className="mb-4 flex items-end justify-between gap-3">
                    <div>
                      <h3 className="text-base font-semibold text-text">Board Review</h3>
                      <p className="text-sm text-text-secondary">Found words stay highlighted so overlaps are easy to inspect.</p>
                    </div>
                    <span className="text-[11px] uppercase tracking-[0.18em] text-text-secondary">
                      {getThemeLabel(game.theme)}
                    </span>
                  </div>
                  {boardView}
                </Card>
              )}
            </div>

            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                {rankedPlayers.map((player, i) => (
                    <Card
                      key={player.id}
                      className={`flex items-center justify-between ${i === 0 ? "border-primary" : ""} ${
                        player.connected ? "" : "opacity-70"
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-lg font-bold text-primary">#{i + 1}</span>
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate font-medium text-text">{player.username}</span>
                          {!player.connected && (
                            <span className="text-[10px] uppercase tracking-[0.16em] text-text-secondary">
                              Offline
                            </span>
                          )}
                        </div>
                      </div>
                      <span className="font-bold text-text">{player.score} pts</span>
                    </Card>
                  ))}
              </div>

              {grid && (
                <Card className="px-4 py-4">
                  <div className="mb-3">
                    <h3 className="text-base font-semibold text-text">Word Review</h3>
                    <p className="text-sm text-text-secondary">Missed words stay visible so the whole board makes sense.</p>
                  </div>
                  <div className="flex max-h-[26rem] flex-col gap-2 overflow-y-auto pr-1">
                    {boardWords.map((placedWord) => {
                      const found = foundWords.has(placedWord.word);
                      const ownerId = foundWordOwners.get(placedWord.word);
                      const ownerName = ownerId ? playersById.get(ownerId)?.username ?? "Player" : "Missed";
                      const ownerColor = ownerId
                        ? getPlayerColor(ownerId, playersById, currentUserId, highlightColor)
                        : null;
                      const wordScore = scoreWord(placedWord.word, game.difficulty);

                      return (
                        <div
                          key={`${placedWord.word}:${placedWord.start.join(",")}:${placedWord.direction}`}
                          className="flex items-center justify-between gap-3 rounded-xl border border-border bg-bg-secondary/70 px-3 py-2"
                        >
                          <div className="min-w-0">
                            <div className="font-medium text-text">{placedWord.word}</div>
                            <div
                              className="text-xs"
                              style={ownerColor ? { color: ownerColor } : { color: "var(--text-secondary)" }}
                            >
                              {found ? `Found by ${ownerName}` : "Missed this round"}
                            </div>
                          </div>
                          <div className="shrink-0 text-sm font-semibold text-text-secondary">+{wordScore}</div>
                        </div>
                      );
                    })}
                  </div>
                </Card>
              )}

              <Button onClick={() => router.push("/lobby")}>Back to Lobby</Button>
            </div>
          </div>
        </div>
      )}
    </PageContainer>
  );
}
