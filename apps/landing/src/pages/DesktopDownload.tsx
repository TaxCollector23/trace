import { Link } from "react-router-dom";
import Tracey from "../Tracey";
import { Mark } from "../Mark";
import { GITHUB_REPO } from "../config";

const RELEASE_URL = `${GITHUB_REPO}/releases/latest`;
const DMG_URL = `${GITHUB_REPO}/releases/latest/download/trace-desktop-macos-arm64.dmg`;

export default function DesktopDownload() {
  return (
    <div className="mx-auto max-w-[560px] py-20 text-center">
      <div className="flex items-center justify-center gap-3">
        <Mark size={36} />
        <span className="text-2xl font-semibold">Trace Desktop</span>
      </div>

      <div className="mt-8 flex justify-center">
        <Tracey expression="excited" size={96} />
      </div>

      <p className="mt-6 text-text-dim">
        The same local dashboard, in its own window. Starts the daemon automatically — no terminal needed.
      </p>

      <div className="mt-8 flex flex-col items-center gap-4">
        <a
          href={DMG_URL}
          className="rounded bg-brand px-8 py-3 text-base font-medium text-white hover:bg-brand-dim"
        >
          Download for macOS (Apple Silicon)
        </a>
        <span className="text-xs text-text-dimmer">.dmg · arm64 · macOS 12+</span>
      </div>

      <p className="mt-10 text-sm text-text-dim">
        Windows and Linux builds are coming.{" "}
        <a
          href={RELEASE_URL}
          target="_blank"
          rel="noreferrer"
          className="font-medium text-brand hover:text-brand-dim"
        >
          See all releases on GitHub
        </a>
      </p>

      <div className="mt-10">
        <Link to="/" className="text-sm text-text-dim hover:text-text">
          ← Back to home
        </Link>
      </div>
    </div>
  );
}
