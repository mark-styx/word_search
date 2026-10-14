import { Badge, Button } from "@/components/ui";
import type { WordBank } from "@/types";

type WordBankLibraryCardProps = {
  bank: WordBank;
  selected: boolean;
  onUse: (bank: WordBank) => void;
  renderStatus: (bank: WordBank) => string;
  formatError: (message: string) => string;
};

export default function WordBankLibraryCard({
  bank,
  selected,
  onUse,
  renderStatus,
  formatError,
}: WordBankLibraryCardProps) {
  const ownershipLabel =
    bank.source === "builtin"
      ? "Built-in bank"
      : bank.is_owner
        ? bank.is_shared
          ? "Shared with other players"
          : "Private to you"
        : `Shared by ${bank.owner_username ?? "another player"}`;

  return (
    <div
      className={`rounded-xl border p-4 ${
        selected ? "border-primary bg-primary/5" : "border-border bg-bg-secondary/50"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-semibold text-text">{bank.label}</h3>
            <Badge variant={bank.source === "builtin" ? "default" : "primary"}>
              {bank.source === "builtin" ? "Built-In" : "Custom"}
            </Badge>
          </div>
          <p className="mt-1 text-xs text-text-secondary">
            {ownershipLabel}
            {" · "}
            {bank.used_words_count}/{bank.total_words || 0} unique words used
            {" · "}
            {bank.total_usage_count} total placements
          </p>
        </div>
        <Badge
          variant={
            bank.status === "ready"
              ? "primary"
              : bank.status === "failed"
                ? "error"
                : "default"
          }
        >
          {renderStatus(bank)}
        </Badge>
      </div>

      <div className="mt-3 h-2 overflow-hidden rounded-full bg-border">
        <div
          className={`h-full rounded-full transition-all ${
            bank.status === "failed" ? "bg-error" : "bg-primary"
          }`}
          style={{ width: `${Math.max(bank.coverage_percent, bank.status === "ready" ? 4 : 0)}%` }}
        />
      </div>

      <p className="mt-2 text-xs text-text-secondary">
        Coverage {bank.coverage_percent}%.
        {bank.extension_state === "extending" ? " Fresh words are being fetched now." : ""}
        {bank.extension_state === "disabled" && bank.can_extend === false
          ? " This theme is being kept intentionally tight."
          : ""}
      </p>

      {bank.error_message && bank.status === "failed" && (
        <p className="mt-2 text-sm text-error">{formatError(bank.error_message)}</p>
      )}

      {bank.last_extension_error && bank.extension_state === "failed" && (
        <p className="mt-2 text-sm text-error">{formatError(bank.last_extension_error)}</p>
      )}

      {bank.status === "ready" && (
        <div className="mt-3">
          <Button
            size="sm"
            variant={selected ? "primary" : "secondary"}
            onClick={() => onUse(bank)}
          >
            {selected ? "Selected" : "Use This Bank"}
          </Button>
        </div>
      )}
    </div>
  );
}
