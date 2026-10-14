import { Crown } from "lucide-react";

import { Button, Card } from "@/components/ui";
import type { PlayerState } from "@/types";

type LobbyRosterProps = {
  players: PlayerState[];
  currentPlayer: PlayerState | null;
  onReady: () => void;
  onStart: () => void;
};

export default function LobbyRoster({
  players,
  currentPlayer,
  onReady,
  onStart,
}: LobbyRosterProps) {
  return (
    <div className="flex flex-col items-center gap-6 py-8">
      <h2 className="text-xl font-bold text-text">Waiting for players...</h2>
      <div className="flex flex-wrap gap-3">
        {players.map((player) => (
          <Card
            key={player.id}
            className={`flex items-center gap-2 px-4 ${player.connected ? "" : "opacity-70"}`}
          >
            {player.is_host && <Crown size={14} className="text-yellow-500" />}
            <span className="font-medium text-text">{player.username}</span>
            {player.ready && (
              <span className="text-xs uppercase tracking-[0.18em] text-primary">
                Ready
              </span>
            )}
            {!player.connected && (
              <span className="text-xs uppercase tracking-[0.18em] text-text-secondary">
                Disconnected
              </span>
            )}
          </Card>
        ))}
      </div>
      <div className="flex gap-3">
        <Button onClick={onReady} disabled={currentPlayer?.ready}>
          {currentPlayer?.ready ? "Ready" : "Ready Up"}
        </Button>
        {currentPlayer?.is_host && (
          <Button onClick={onStart}>Start Game</Button>
        )}
      </div>
    </div>
  );
}
