import { ToneIcon } from "./ui";

// ---------------------------------------------------------------------------
// Honest whole-screen states (§79, §121, §125, §126). None of these fabricate
// history or show placeholder charts. A new install is told exactly what to do;
// a broken daemon is told exactly what is wrong and how to recover.
// ---------------------------------------------------------------------------

export function DisconnectedScreen({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="v4-screen">
      <div className="v4-screen-icon tone-danger">
        <ToneIcon tone="danger" size={30} />
      </div>
      <h2>The Trace daemon is unreachable</h2>
      <p className="muted">
        The dashboard could not reach the local daemon on 127.0.0.1. Nothing below is live. This is
        an honest blank — Trace will not show stale or invented data while disconnected.
      </p>
      <p className="muted">Start Trace with <b>trc daemon start</b>, then refresh this page.</p>
      <p className="muted">If it still does not connect, run <b>trc doctor</b>.</p>
      <button className="btn" style={{ marginTop: 8 }} onClick={onRetry}>
        Retry connection
      </button>
    </div>
  );
}

export function DbUnavailableScreen({ reason, onRetry }: { reason: string; onRetry: () => void }) {
  return (
    <div className="v4-screen">
      <div className="v4-screen-icon tone-danger">
        <ToneIcon tone="danger" size={30} />
      </div>
      <h2>The run database is unavailable</h2>
      <p className="muted">
        The daemon is running but returned a server error while reading its store. {reason}
      </p>
      <p className="muted">Run <b>trc doctor</b> in a terminal, then refresh this page.</p>
      <button className="btn" style={{ marginTop: 8 }} onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}

export function CorruptScreen({ reason, onRetry }: { reason: string; onRetry: () => void }) {
  return (
    <div className="v4-screen">
      <div className="v4-screen-icon tone-danger">
        <ToneIcon tone="danger" size={30} />
      </div>
      <h2>This run's data could not be read</h2>
      <p className="muted">
        {reason} Rather than guess at the missing structure, Trace is refusing to render it. Pick
        another run, or retry once the daemon is on a matching version.
      </p>
      <button className="btn" style={{ marginTop: 8 }} onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}

export function OnboardingScreen() {
  return (
    <div className="v4-onboard">
      <h2>Trace is connected. No runs recorded yet.</h2>
      <p className="muted">
        There is no history to show, so there are no charts here — that would be fake. Do these
        three things and this screen fills with real execution.
      </p>
      <ol className="v4-steps">
        <li>
          <div className="v4-step-n">1</div>
          <div>
            <b>Connect an agent</b>
            <p className="muted">Install the shims so Trace can observe your coding agent.</p>
            <p className="muted">Run <b>trc integrations install all</b> once.</p>
          </div>
        </li>
        <li>
          <div className="v4-step-n">2</div>
          <div>
            <b>Run a task through Trace</b>
            <p className="muted">Wrap an agent session so every command and edit is recorded.</p>
            <p className="muted">Start a task with <b>trc run "claude"</b>.</p>
          </div>
        </li>
        <li>
          <div className="v4-step-n">3</div>
          <div>
            <b>Watch execution here</b>
            <p className="muted">
              The Run Page opens automatically for the active run — its timeline, risk signals,
              diff, and controls update live.
            </p>
          </div>
        </li>
      </ol>
    </div>
  );
}
