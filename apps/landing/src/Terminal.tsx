import { useEffect, useRef, useState } from "react";
import { Reveal } from "./components";

// The exact startup banner the CLI prints (figlet "ANSI Shadow").
const STARTUP_ART = `████████╗██████╗  █████╗  ██████╗███████╗
╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝
   ██║   ██████╔╝███████║██║     █████╗
   ██║   ██╔══██╗██╔══██║██║     ██╔══╝
   ██║   ██║  ██║██║  ██║╚██████╗███████╗
   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝`;
const ART_LINES = STARTUP_ART.split("\n");

export interface TermLine {
  text: string;
  cls?: string;
}

// A premium, self-contained terminal that plays a boot sequence when it
// scrolls into view: the banner prints line by line, the command types itself
// character by character, then the output lines fade in one at a time. Reused
// for the install command and the "wire up every agent" demo.
export default function Terminal({
  label = "trace",
  command,
  output = [],
  copyText,
}: {
  label?: string;
  command: string;
  output?: TermLine[];
  copyText?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [artLines, setArtLines] = useState(0);
  const [typed, setTyped] = useState("");
  const [typedDone, setTypedDone] = useState(false);
  const [outCount, setOutCount] = useState(0);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const reduce = !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      setArtLines(ART_LINES.length);
      setTyped(command);
      setTypedDone(true);
      setOutCount(output.length);
      return;
    }
    const el = ref.current;
    if (!el) return;
    const intervals: ReturnType<typeof setInterval>[] = [];
    let started = false;
    const start = () => {
      if (started) return;
      started = true;
      let a = 0;
      const artTimer = setInterval(() => {
        a += 1;
        setArtLines(a);
        if (a >= ART_LINES.length) {
          clearInterval(artTimer);
          let i = 0;
          const typeTimer = setInterval(() => {
            i += 1;
            setTyped(command.slice(0, i));
            if (i >= command.length) {
              clearInterval(typeTimer);
              setTypedDone(true);
              let o = 0;
              if (output.length > 0) {
                const outTimer = setInterval(() => {
                  o += 1;
                  setOutCount(o);
                  if (o >= output.length) clearInterval(outTimer);
                }, 110);
                intervals.push(outTimer);
              }
            }
          }, 42);
          intervals.push(typeTimer);
        }
      }, 70);
      intervals.push(artTimer);
    };
    const io = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          io.disconnect();
          start();
        }
      },
      { threshold: 0.3 }
    );
    io.observe(el);
    return () => {
      io.disconnect();
      intervals.forEach(clearInterval);
    };
  }, [command, output.length]);

  const copy = () => {
    if (!copyText) return;
    navigator.clipboard.writeText(copyText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1400);
  };

  const caretIdle = typedDone && output.length === 0;

  return (
    <Reveal>
      <div
        ref={ref}
        className="mx-auto max-w-[760px] overflow-hidden rounded-3xl border border-white/10 bg-[#0b0b0f] shadow-2xl"
      >
        {/* Title bar */}
        <div className="flex items-center gap-2 border-b border-white/10 bg-white/[0.03] px-5 py-3.5">
          <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <span className="h-3 w-3 rounded-full bg-[#28c840]" />
          <span className="ml-2.5 font-mono text-xs text-white/45">{label}</span>
          {copyText && (
            <button
              onClick={copy}
              aria-label="Copy command"
              className="ml-auto flex h-7 w-7 items-center justify-center rounded-md border border-white/15 bg-white/5 text-white/80 transition-colors hover:bg-white/15"
            >
              {copied ? (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M20 6 9 17l-5-5" />
                </svg>
              ) : (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
              )}
            </button>
          )}
        </div>

        {/* Body */}
        <div className="overflow-x-auto px-7 py-7">
          <pre className="font-mono text-[10px] leading-[1.25] text-brand sm:text-[14px]">
            {ART_LINES.map((line, i) => (
              <div
                key={i}
                className="transition-all duration-300 ease-out"
                style={{
                  opacity: i < artLines ? 1 : 0,
                  transform: i < artLines ? "none" : "translateY(4px)",
                }}
              >
                {line || " "}
              </div>
            ))}
          </pre>

          <pre className="mt-6 whitespace-pre-wrap break-words font-mono text-sm text-white sm:text-[15px]">
            <span className="select-none text-white/40">$ </span>
            {typed}
            <span
              className={`ml-0.5 inline-block h-4 w-[7px] translate-y-[2px] bg-white/80 align-middle ${
                caretIdle ? "term-caret" : ""
              }`}
              style={{ opacity: typedDone && output.length > 0 ? 0 : 1 }}
            />
          </pre>

          {output.length > 0 && (
            <pre className="mt-1.5 whitespace-pre-wrap break-words font-mono text-[13px] leading-relaxed text-white/85 sm:text-sm">
              {output.map((l, i) => (
                <div
                  key={i}
                  className={`transition-opacity duration-200 ${l.cls ?? ""}`}
                  style={{ opacity: i < outCount ? 1 : 0 }}
                >
                  {l.text || " "}
                </div>
              ))}
            </pre>
          )}
        </div>
      </div>
    </Reveal>
  );
}
