import type { PlayerState } from "@/types";

type ScoreboardPanelProps = {
  players: PlayerState[];
};

export default function ScoreboardPanel({ players }: ScoreboardPanelProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="px-0.5 text-[11px] font-semibold uppercase tracking-[0.16em] text-text-secondary">Scores</h3>
      {players.map((player) => (
        <div
          key={player.id}
          className={`flex min-h-9 items-center justify-between rounded-md bg-bg-secondary/80 px-2.5 py-1.5 ${
            player.connected ? "" : "opacity-70"
          }`}
        >
          <div className="flex min-w-0 items-center gap-2">
            <span className="truncate text-sm font-medium text-text">{player.username}</span>
            {!player.connected && (
              <span className="text-[10px] uppercase tracking-[0.16em] text-text-secondary">
                Offline
              </span>
            )}
          </div>
          <span className="text-sm font-bold text-primary">{player.score} pts</span>
        </div>
      ))}
    </div>
  );
}
