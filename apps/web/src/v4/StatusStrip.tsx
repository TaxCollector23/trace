import type { Action, StateDescriptor } from "../state";
import { LiveRegion, ToneIcon } from "./ui";

// ---------------------------------------------------------------------------
// The always-present status strip. It makes the current dashboard state, its
// explanation, the primary action, and the keyboard model impossible to miss —
// so the user never hunts for "what is happening right now" (§2).
// ---------------------------------------------------------------------------

export function StatusStrip({
  descriptor,
  onAction,
  right,
}: {
  descriptor: StateDescriptor;
  onAction: (a: Action) => void;
  right?: React.ReactNode;
}) {
  const { tone, label, explanation, primaryAction, secondaryActions, keyboard, announce } =
    descriptor;
  return (
    <div className={`v4-strip tone-${tone}`} role="region" aria-label="Current status">
      <LiveRegion text={announce.text} assertive={announce.assertive} />
      <div className="v4-strip-main">
        <span className="v4-strip-state">
          <ToneIcon tone={tone} size={16} />
          {label}
        </span>
        <span className="v4-strip-explain">{explanation}</span>
      </div>
      <div className="v4-strip-actions">
        {right}
        {secondaryActions.map((a) => (
          <button key={a.label} className="btn-ghost" onClick={() => onAction(a)}>
            {a.label}
          </button>
        ))}
        {primaryAction && (
          <button
            className={`btn ${primaryAction.tone === "danger" ? "danger" : ""}`}
            onClick={() => onAction(primaryAction)}
          >
            {primaryAction.label}
          </button>
        )}
      </div>
      <div className="v4-strip-kbd" aria-hidden="true">
        {keyboard}
      </div>
    </div>
  );
}
