import { ChartColumn, ChevronDown, ChevronRight } from "lucide-react";

import WordBankLibraryCard from "@/components/lobby/WordBankLibraryCard";
import { Card } from "@/components/ui";
import type { WordBank } from "@/types";

type WordBankLibrarySectionProps = {
  wordBanks: WordBank[];
  showWordBanks: boolean;
  selectedBankKey: string;
  onToggle: () => void;
  onUseBank: (bank: WordBank) => void;
  renderStatus: (bank: WordBank) => string;
  formatError: (message: string) => string;
};

export default function WordBankLibrarySection({
  wordBanks,
  showWordBanks,
  selectedBankKey,
  onToggle,
  onUseBank,
  renderStatus,
  formatError,
}: WordBankLibrarySectionProps) {
  if (wordBanks.length === 0) {
    return null;
  }

  return (
    <Card>
      <button
        type="button"
        className="flex w-full items-start justify-between gap-3 text-left"
        onClick={onToggle}
      >
        <div className="flex items-center gap-2">
          <ChartColumn size={18} className="text-primary" />
          <div>
            <h2 className="font-semibold text-text">Word Bank Library</h2>
            <p className="text-sm text-text-secondary">
              Built-in and custom banks live together here. Shared custom banks behave like any other bank.
            </p>
          </div>
        </div>
        <span className="mt-1 inline-flex items-center gap-1 text-sm text-text-secondary">
          {showWordBanks ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          {showWordBanks ? "Hide" : "Show"}
        </span>
      </button>

      {showWordBanks && (
        <div className="mt-4 grid gap-3 lg:grid-cols-2">
          {wordBanks.map((bank) => (
            <WordBankLibraryCard
              key={bank.id}
              bank={bank}
              selected={selectedBankKey === bank.key}
              onUse={onUseBank}
              renderStatus={renderStatus}
              formatError={formatError}
            />
          ))}
        </div>
      )}
    </Card>
  );
}
