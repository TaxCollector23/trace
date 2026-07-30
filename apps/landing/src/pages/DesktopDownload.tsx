import { Link } from "react-router-dom";
import { Button } from "../components";
import { DOWNLOADS } from "../config";

interface Option {
  os: "macOS" | "Windows" | "Linux";
  tagline: string;
  primary: { label: string; href: string; sub: string };
  secondary?: { label: string; href: string };
}

const OPTIONS: Option[] = [
  {
    os: "macOS",
    tagline: "Apple Silicon · macOS 12+",
    primary: {
      label: "Download for macOS",
      href: DOWNLOADS.macOS,
      sub: "trace-desktop-macos-arm64.dmg",
    },
  },
  {
    os: "Windows",
    tagline: "Windows 10/11 · x64",
    primary: {
      label: "Download for Windows",
      href: DOWNLOADS.windows,
      sub: "trace-desktop-windows-x64.exe · NSIS installer",
    },
  },
  {
    os: "Linux",
    tagline: "x64 · Ubuntu/Debian/Fedora/Arch",
    primary: {
      label: "Download .deb",
      href: DOWNLOADS.linuxDeb,
      sub: "trace-desktop-linux-x64.deb",
    },
    secondary: { label: "Or AppImage", href: DOWNLOADS.linuxAppImage },
  },
];

export default function DesktopDownload() {
  return (
    <div className="py-24">
      <Link to="/" className="text-sm text-text-dim hover:text-text">
        ← Back
      </Link>

      <div className="mt-10 text-center">
        <h1 className="font-serif text-4xl text-text">Download Trace</h1>
        <p className="mx-auto mt-4 max-w-[520px] text-text-dim">
          One binary. It starts its own local daemon, embeds the dashboard, and
          connects to your agents automatically — no terminal required.
        </p>
      </div>

      <div className="mx-auto mt-14 grid max-w-[960px] grid-cols-1 gap-6 md:grid-cols-3">
        {OPTIONS.map((opt) => (
          <div
            key={opt.os}
            className="flex flex-col items-center rounded-3xl border border-border bg-white p-8 text-center transition-shadow hover:shadow-lg"
          >
            <div className="text-xl font-semibold text-text">{opt.os}</div>
            <div className="mt-1 text-sm text-text-dim">{opt.tagline}</div>
            <div className="mt-8">
              <Button href={opt.primary.href}>{opt.primary.label}</Button>
            </div>
            <div className="mt-3 text-xs text-text-dim">{opt.primary.sub}</div>
            {opt.secondary && (
              <a
                href={opt.secondary.href}
                className="mt-4 text-sm text-brand hover:text-brand-dim"
              >
                {opt.secondary.label} →
              </a>
            )}
          </div>
        ))}
      </div>

      <div className="mx-auto mt-16 max-w-[720px] rounded-2xl border border-border bg-white/50 p-6 text-center">
        <div className="text-sm font-medium text-text">Prefer the CLI?</div>
        <div className="mt-1 text-sm text-text-dim">
          Homebrew, npm, and cargo install paths are all live. See the{" "}
          <a
            href={DOWNLOADS.releases}
            target="_blank"
            rel="noreferrer"
            className="text-brand hover:text-brand-dim"
          >
            release page
          </a>{" "}
          for the full list and checksums.
        </div>
      </div>
    </div>
  );
}
