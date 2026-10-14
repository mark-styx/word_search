import { Loader2, Search } from "lucide-react";

import { Badge, Button, Card } from "@/components/ui";
import { difficultyLabels, getThemeLabel } from "@/lib/game-themes";
import type { GameLobby } from "@/types";

type ActiveGamesSectionProps = {
  loading: boolean;
  games: GameLobby[];
  onJoin: (id: string) => void;
  onCreate: () => void;
};

export default function ActiveGamesSection({
  loading,
  games,
  onJoin,
  onCreate,
}: ActiveGamesSectionProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-12 text-text-secondary">
        <Loader2 className="mr-2 animate-spin" size={20} /> Loading games...
      </div>
    );
  }

  if (games.length === 0) {
    return (
      <Card className="flex flex-col items-center gap-3 py-12 text-center">
        <Search size={40} className="text-text-secondary" />
        <p className="text-text-secondary">No active games right now</p>
        <p className="text-sm text-text-secondary">Create one and invite your friends.</p>
        <Button size="sm" onClick={onCreate}>Create Game</Button>
      </Card>
    );
  }

  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {games.map((game) => (
        <Card key={game.id} hoverable onClick={() => onJoin(game.id)}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Badge variant="primary">{getThemeLabel(game.theme) || "Unknown Theme"}</Badge>
              <Badge>{difficultyLabels[game.difficulty] ?? game.difficulty ?? "Unknown"}</Badge>
            </div>
            <span className="text-sm text-text-secondary">
              {game.player_count}/{game.max_players}
            </span>
          </div>
          <p className="mt-2 text-xs text-text-secondary">
            Host: {game.host?.trim() || "Unknown host"}
          </p>
        </Card>
      ))}
    </div>
  );
}
