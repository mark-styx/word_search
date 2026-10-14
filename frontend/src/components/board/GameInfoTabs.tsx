type PanelKey = "scores" | "words" | "chat";

type GameInfoTabsProps = {
  active: PanelKey;
  onChange: (panel: PanelKey) => void;
};

const PANELS: { key: PanelKey; label: string }[] = [
  { key: "words", label: "Words" },
  { key: "scores", label: "Scores" },
  { key: "chat", label: "Chat" },
];

export default function GameInfoTabs({ active, onChange }: GameInfoTabsProps) {
  return (
    <div className="grid grid-cols-3 gap-1 rounded-full border border-border/70 bg-bg-secondary/45 p-0.5">
      {PANELS.map((panel) => (
        <button
          key={panel.key}
          type="button"
          onClick={() => onChange(panel.key)}
          className={`rounded-full px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.12em] transition ${
            active === panel.key
              ? "bg-primary text-white shadow-sm"
              : "text-text-secondary hover:bg-bg-secondary/80 hover:text-text"
          }`}
          aria-pressed={active === panel.key}
        >
          {panel.label}
        </button>
      ))}
    </div>
  );
}
