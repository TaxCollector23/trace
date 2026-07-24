import { Link } from "react-router-dom";
import Download from "../Download";

export default function DesktopDownload() {
  return (
    <div className="py-10">
      <Link to="/" className="text-sm text-text-dim hover:text-text">
        ← Back
      </Link>
      <div className="mt-6">
        <Download />
      </div>
    </div>
  );
}
