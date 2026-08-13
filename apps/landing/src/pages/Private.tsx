import { useState } from "react";
import { Link } from "react-router-dom";
import { motion } from "framer-motion";

// NOTE: this is a client-side gate only — the password ships in the built
// JavaScript, so it keeps casual visitors out but is NOT real security. Don't
// put anything behind it that would be harmful to leak.
const ACCESS_CODE = "password123";
const GUIDE_URL = "/trace-testing-guide.md";

export default function Private() {
  const [entry, setEntry] = useState("");
  const [unlocked, setUnlocked] = useState(false);
  const [error, setError] = useState(false);

  function submit(e: React.FormEvent) {
    e.preventDefault();
    if (entry === ACCESS_CODE) {
      setUnlocked(true);
      setError(false);
    } else {
      setError(true);
    }
  }

  return (
    <div className="max-w-[560px] py-16">
      <Link to="/" className="text-sm text-text-dim hover:text-text">
        ← Back
      </Link>

      <motion.h1
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        className="mt-5 font-serif text-4xl text-text md:text-5xl"
      >
        Private
      </motion.h1>

      {!unlocked ? (
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.08, ease: [0.16, 1, 0.3, 1] }}
        >
          <p className="mt-4 text-lg leading-relaxed text-text-dim">
            Enter the access code to unlock the download.
          </p>

          <form onSubmit={submit} className="mt-7 flex flex-col gap-3 sm:flex-row">
            <input
              type="password"
              autoFocus
              value={entry}
              onChange={(e) => {
                setEntry(e.target.value);
                setError(false);
              }}
              placeholder="Access code"
              aria-label="Access code"
              className="h-11 flex-1 rounded-full border border-border bg-white px-5 text-sm text-text outline-none focus:border-brand"
            />
            <button
              type="submit"
              className="h-11 rounded-full bg-brand px-6 text-sm font-medium text-white shadow-sm transition-transform hover:-translate-y-[1px]"
            >
              Unlock
            </button>
          </form>

          {error && (
            <p className="mt-3 text-sm text-[#dc2626]">
              Incorrect code. Try again.
            </p>
          )}
        </motion.div>
      ) : (
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        >
          <p className="mt-4 text-lg leading-relaxed text-text-dim">
            Unlocked. Here's your complete step-by-step guide to testing Trace
            and Ratify end to end — no AI key required.
          </p>

          <div className="mt-7 rounded-2xl border border-border bg-white p-6 shadow-sm">
            <div className="font-serif text-lg text-text">
              Testing Trace + Ratify — Complete Guide
            </div>
            <p className="mt-2 text-sm leading-relaxed text-text-dim">
              Build the CLI, run the detection benchmarks, scan a dangerous
              file, open the dashboard, and ratify a real GitHub pull request —
              every step, in one document.
            </p>
            <a
              href={GUIDE_URL}
              download="trace-testing-guide.md"
              className="mt-5 inline-flex h-11 items-center rounded-full bg-brand px-6 text-sm font-medium text-white shadow-sm transition-transform hover:-translate-y-[1px]"
            >
              Download the guide ↓
            </a>
            <a
              href={GUIDE_URL}
              target="_blank"
              rel="noreferrer"
              className="ml-3 text-sm text-text-dim hover:text-text"
            >
              or view it in the browser →
            </a>
          </div>
        </motion.div>
      )}
    </div>
  );
}
