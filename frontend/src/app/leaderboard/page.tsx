"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Trophy, ArrowLeft, Medal } from "lucide-react";
import LeaderboardCharts from "@/components/stats/LeaderboardCharts";
import { PageContainer } from "@/components/layout";
import { Button, Card } from "@/components/ui";
import { api } from "@/lib/api";
import type { LeaderboardEntry } from "@/types";

export default function LeaderboardPage() {
  const router = useRouter();
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);

  useEffect(() => {
    api
      .getLeaderboard(page, 25)
      .then((res) => {
        setEntries((res as { entries: LeaderboardEntry[] }).entries ?? []);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [page]);

  return (
    <PageContainer className="flex flex-col gap-6 py-8">
      <div className="flex items-center gap-3">
        <button
          onClick={() => router.back()}
          className="rounded-lg p-2 text-text-secondary hover:bg-bg-secondary transition-colors"
        >
          <ArrowLeft size={20} />
        </button>
        <Trophy size={24} className="text-primary" />
        <h1 className="text-2xl font-bold text-text">Leaderboard</h1>
      </div>

      {loading ? (
        <p className="text-text-secondary text-center py-12">Loading...</p>
      ) : entries.length === 0 ? (
        <Card className="py-12 text-center text-text-secondary">
          No games played yet. Be the first!
        </Card>
      ) : (
        <div className="flex flex-col gap-6">
          <LeaderboardCharts
            entries={entries}
            metrics={["total_score", "games_won", "points_per_game", "win_rate"]}
            title="Leaderboard Views"
            description="Separate charts for total points, wins, scoring efficiency, and win percentage."
          />

          <div className="flex flex-col gap-2">
            {entries.map((e, i) => {
              const rank = (page - 1) * 25 + i + 1;
              const winRate = e.total_games > 0 ? Math.round((e.games_won / e.total_games) * 100) : 0;
              const pointsPerGame = e.total_games > 0 ? e.total_score / e.total_games : 0;

              return (
                <Card key={e.username} className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <div
                      className={`flex h-8 w-8 items-center justify-center rounded-full text-sm font-bold ${
                        rank === 1
                          ? "bg-yellow-500/20 text-yellow-600"
                          : rank === 2
                            ? "bg-gray-300/20 text-gray-500"
                            : rank === 3
                              ? "bg-orange-400/20 text-orange-600"
                              : "bg-bg-secondary text-text-secondary"
                      }`}
                    >
                      {rank <= 3 ? <Medal size={16} /> : rank}
                    </div>
                    <div className="min-w-0">
                      <div className="truncate font-medium text-text">{e.username}</div>
                      <div className="text-xs text-text-secondary">
                        {e.games_won} wins in {e.total_games} games
                      </div>
                    </div>
                  </div>
                  <div className="grid shrink-0 grid-cols-2 gap-x-4 gap-y-1 text-right text-sm">
                    <span className="text-text-secondary">Points</span>
                    <span className="font-semibold text-text">{e.total_score.toLocaleString()}</span>
                    <span className="text-text-secondary">Pts/Game</span>
                    <span className="font-semibold text-text">{pointsPerGame.toFixed(pointsPerGame >= 100 ? 0 : 1)}</span>
                    <span className="text-text-secondary">Win %</span>
                    <span className="font-semibold text-text">{winRate}%</span>
                  </div>
                </Card>
              );
            })}
          </div>
        </div>
      )}

      <div className="flex justify-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          disabled={page === 1}
          onClick={() => setPage((p) => Math.max(1, p - 1))}
        >
          Previous
        </Button>
        <span className="flex items-center text-sm text-text-secondary">Page {page}</span>
        <Button
          variant="ghost"
          size="sm"
          disabled={entries.length < 25}
          onClick={() => setPage((p) => p + 1)}
        >
          Next
        </Button>
      </div>
    </PageContainer>
  );
}
