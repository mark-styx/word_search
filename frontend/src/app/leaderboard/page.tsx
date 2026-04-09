"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Trophy, ArrowLeft, Medal } from "lucide-react";
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
        <div className="flex flex-col gap-2">
          {entries.map((e, i) => {
            const rank = (page - 1) * 25 + i + 1;
            return (
              <Card key={e.username} className="flex items-center justify-between">
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
                  <span className="font-medium text-text">{e.username}</span>
                </div>
                <div className="flex items-center gap-4 text-sm">
                  <div className="text-right">
                    <span className="font-bold text-text">{e.total_score.toLocaleString()}</span>
                    <span className="text-text-secondary ml-1">pts</span>
                  </div>
                  <div className="text-right hidden sm:block">
                    <span className="text-text-secondary">{e.games_won}/{e.total_games}</span>
                    <span className="text-text-secondary ml-1">wins</span>
                  </div>
                </div>
              </Card>
            );
          })}
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
