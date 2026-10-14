"use client";

import { Card } from "@/components/ui";
import {
  formatLeaderboardMetricValue,
  getLeaderboardMetricValue,
  leaderboardMetricLabels,
  sortLeaderboardEntries,
  type LeaderboardMetricKey,
} from "@/lib/leaderboard";
import type { LeaderboardEntry } from "@/types";

type LeaderboardChartsProps = {
  entries: LeaderboardEntry[];
  metrics: LeaderboardMetricKey[];
  title?: string;
  description?: string;
  compact?: boolean;
};

export default function LeaderboardCharts({
  entries,
  metrics,
  title,
  description,
  compact = false,
}: LeaderboardChartsProps) {
  if (entries.length === 0) {
    return (
      <div className="flex flex-col gap-4">
        {(title || description) && (
          <div>
            {title && <h2 className="text-xl font-semibold text-text">{title}</h2>}
            {description && <p className="text-sm text-text-secondary">{description}</p>}
          </div>
        )}
        <Card className="px-5 py-8 text-center text-text-secondary">
          No leaderboard data yet.
        </Card>
      </div>
    );
  }

  const rowLimit = compact ? 5 : 8;

  return (
    <div className="flex flex-col gap-4">
      {(title || description) && (
        <div>
          {title && <h2 className="text-xl font-semibold text-text">{title}</h2>}
          {description && <p className="text-sm text-text-secondary">{description}</p>}
        </div>
      )}

      <div className={`grid gap-4 ${compact ? "lg:grid-cols-2" : "xl:grid-cols-2"}`}>
        {metrics.map((metric) => {
          const ranked = sortLeaderboardEntries(entries, metric).slice(0, rowLimit);
          const maxValue = Math.max(...ranked.map((entry) => getLeaderboardMetricValue(entry, metric)), 1);

          return (
            <Card key={metric} className="px-4 py-4">
              <div className="mb-4 flex items-end justify-between gap-3">
                <div>
                  <h3 className="text-base font-semibold text-text">{leaderboardMetricLabels[metric]}</h3>
                  <p className="text-xs uppercase tracking-[0.18em] text-text-secondary">
                    Top {ranked.length}
                  </p>
                </div>
              </div>

              <div className="flex flex-col gap-3">
                {ranked.map((entry, index) => {
                  const value = getLeaderboardMetricValue(entry, metric);
                  const width = Math.max((value / maxValue) * 100, value > 0 ? 8 : 0);

                  return (
                    <div key={`${metric}:${entry.username}`} className="flex flex-col gap-1">
                      <div className="flex items-center justify-between gap-3 text-sm">
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="w-6 shrink-0 text-xs font-semibold uppercase tracking-[0.18em] text-text-secondary">
                            {index + 1}
                          </span>
                          <span className="truncate font-medium text-text">{entry.username}</span>
                        </div>
                        <span className="shrink-0 font-semibold text-text">
                          {formatLeaderboardMetricValue(value, metric)}
                        </span>
                      </div>
                      <div className="h-2 overflow-hidden rounded-full bg-border/80">
                        <div
                          className="h-full rounded-full"
                          style={{
                            width: `${width}%`,
                            background:
                              "linear-gradient(90deg, var(--primary) 0%, color-mix(in srgb, var(--primary) 65%, white) 100%)",
                          }}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
