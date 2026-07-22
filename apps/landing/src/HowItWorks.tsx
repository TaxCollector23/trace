import { ArrowRight, Eye, GitCommit, MagnifyingGlass, Play, ArrowCounterClockwise } from "@phosphor-icons/react";
import { Reveal } from "./components";

const STEPS = [
  {
    icon: Play,
    title: "Run",
    desc: "Wrap any agent: trace run \"claude fix the bug\". A checkpoint is created before anything touches your files.",
  },
  {
    icon: Eye,
    title: "Watch",
    desc: "File writes, shell commands, and Git state are observed live through the adapter for whatever agent is running.",
  },
  {
    icon: GitCommit,
    title: "Record",
    desc: "Every event lands in a local SQLite timeline under ~/.trace — nothing leaves your machine.",
  },
  {
    icon: MagnifyingGlass,
    title: "Review",
    desc: "Inspect the real Git diff, flagged commands, and estimated cost — from the CLI or the local dashboard.",
  },
  {
    icon: ArrowCounterClockwise,
    title: "Roll back",
    desc: "Not happy with the result? Restore the pre-run checkpoint with one command. Nothing is destructive by default.",
  },
];

export default function HowItWorks() {
  return (
    <div className="relative">
      <div className="grid grid-cols-1 gap-8 md:grid-cols-5 md:gap-4">
        {STEPS.map((s, i) => (
          <Reveal key={s.title} delay={i * 0.08} className="relative">
            <div className="flex items-center gap-3 md:block md:gap-0">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border bg-surface text-brand">
                <s.icon size={18} weight="bold" />
              </div>
              <h3 className="text-sm font-semibold md:mt-3">{s.title}</h3>
            </div>
            <p className="mt-1.5 text-sm text-text-dim">{s.desc}</p>
            {i < STEPS.length - 1 && (
              <ArrowRight
                size={16}
                weight="bold"
                className="pointer-events-none absolute right-[-22px] top-3 hidden text-text-dimmer md:block"
              />
            )}
          </Reveal>
        ))}
      </div>
    </div>
  );
}
