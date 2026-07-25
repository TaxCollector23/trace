import { Link } from "react-router-dom";
import { Button } from "../components";
import { GITHUB_REPO } from "../config";

const DMG_URL = `${GITHUB_REPO}/releases/latest/download/trace-desktop-macos-arm64.dmg`;

export default function DesktopDownload() {
  return (
    <div className="flex flex-col items-center py-24 text-center">
      <Link to="/" className="self-start text-sm text-text-dim hover:text-text">
        ← Back
      </Link>

      <h1 className="mt-10 font-serif text-3xl text-text">Download Trace for macOS</h1>
      <p className="mt-3 max-w-[440px] text-text-dim">
        Apple Silicon · macOS 12+. No terminal required — it starts its own local
        daemon and connects to your agents automatically.
      </p>

      <div className="mt-8">
        <Button href={DMG_URL}>Download for macOS</Button>
      </div>
    </div>
  );
}
