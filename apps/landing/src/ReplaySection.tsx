import { useState } from "react";
import { Section } from "./components";

type Line = { tone: "in" | "out" | "add" | "del" | "dim"; text: string };

interface Frame {
  label: string;
  time: string;
  lines: Line[];
}

const FRAMES: Frame[] = [
  {
    label: "Run created",
    time: "0:00",
    lines: [{ tone: "in", text: 'trace run "claude fix the login bug"' }],
  },
  {
    label: "Checkpoint",
    time: "0:01",
    lines: [
      { tone: "in", text: 'trace run "claude fix the login bug"' },
      { tone: "out", text: "Checkpoint created at a3f9c21" },
    ],
  },
  {
    label: "Watching files",
    time: "0:02",
    lines: [
      { tone: "in", text: 'trace run "claude fix the login bug"' },
      { tone: "out", text: "Checkpoint created at a3f9c21" },
      { tone: "out", text: "Watching file changes…" },
    ],
  },
  {
    label: "Patch captured",
    time: "0:14",
    lines: [
      { tone: "in", text: 'trace run "claude fix the login bug"' },
      { tone: "out", text: "Checkpoint created at a3f9c21" },
      { tone: "dim", text: "src/auth/login.ts" },
      { tone: "del", text: "- if (user.token = null) {" },
      { tone: "add", text: "+ if (user.token === null) {" },
    ],
  },
  {
    label: "Checks run",
    time: "0:22",
    lines: [
      { tone: "dim", text: "src/auth/login.ts" },
      { tone: "del", text: "- if (user.token = null) {" },
      { tone: "add", text: "+ if (user.token === null) {" },
      { tone: "in", text: "npm test" },
      { tone: "out", text: "✓ 42 passed, 0 failed" },
    ],
  },
  {
    label: "Complete",
    time: "0:23",
    lines: [
      { tone: "in", text: "npm test" },
      { tone: "out", text: "✓ 42 passed, 0 failed" },
      { tone: "dim", text: "── Trace run summary ──" },
      { tone: "out", text: "  status:  completed (exit 0)" },
      { tone: "out", text: "  cost:    $0.02" },
    ],
  },
];

const toneClass: Record<Line["tone"], string> = {
  in: "text-text",
  out: "text-text-dim",
  add: "text-good",
  del: "text-bad",
  dim: "text-text-dim",
};

export default function ReplaySection() {
  const [frame, setFrame] = useState(0);
  const current = FRAMES[frame];

  return (
    <Section
      title="Replay"
      lede="Drag the playhead. This is real recorded run data — the same commands, the same diff, the same summary trace run prints — not a video."
    >
      <div className="overflow-hidden rounded-md border border-border bg-surface">
        <div className="flex items-center justify-between border-b border-border px-4 py-2.5 font-mono text-xs text-text-dim">
          <span>127.0.0.1:8757 — Trace</span>
          <span>{current.time}</span>
        </div>
        <div className="min-h-[180px] p-4 font-mono text-[13px] leading-[1.7]">
          {current.lines.map((l, i) => (
            <div key={i} className={`whitespace-pre ${toneClass[l.tone]}`}>
              {l.tone === "in" && <span className="mr-2 select-none text-brand">$</span>}
              {l.text}
            </div>
          ))}
        </div>

        <div className="border-t border-border px-4 py-4">
          <input
            type="range"
            min={0}
            max={FRAMES.length - 1}
            step={1}
            value={frame}
            onChange={(e) => setFrame(Number(e.target.value))}
            className="trace-scrubber w-full"
            aria-label="Replay timeline"
          />
          <div className="mt-2 grid text-[11px] text-text-dim" style={{ gridTemplateColumns: `repeat(${FRAMES.length}, 1fr)` }}>
            {FRAMES.map((f, i) => (
              <button
                key={f.label}
                onClick={() => setFrame(i)}
                className={`text-left ${i === frame ? "font-medium text-brand" : "hover:text-text"}`}
              >
                {f.label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </Section>
  );
}
