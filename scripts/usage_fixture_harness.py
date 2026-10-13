#!/usr/bin/env python3
"""Golden/property harness for importer-backed usage fixtures.

This is intentionally independent of Trace's Rust implementation. It creates
an isolated Claude home, invokes the shipped binary, normalizes only the
documented JSON summary contract, and compares it with hand-verified expected
files. That makes the same harness reusable when another agent importer lands.

Exit codes:
  0  validation or importer-backed checks passed
  2  the requested importer/CLI contract is unavailable or a check failed
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import random
import re
import subprocess
import sys
import tempfile
import time
from contextlib import contextmanager
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any, Iterator


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures" / "claude"
TOKEN_ALIASES = {
    "input_tokens": ("input_tokens",),
    "output_tokens": ("output_tokens",),
    "cache_read_tokens": ("cache_read_tokens", "cache_read_input_tokens"),
    "cache_write_tokens": (
        "cache_write_tokens",
        "cache_creation_input_tokens",
    ),
    "reasoning_tokens": ("reasoning_tokens", "thinking_tokens"),
}
SECRET_RE = re.compile(
    r"(?:AKIA[0-9A-Z]{16}|(?:sk|ghp|xox[baprs])[-_][A-Za-z0-9_-]{12,}|"
    r"Bearer\s+[A-Za-z0-9._~-]{12,})"
)
PRIVATE_PATH_RE = re.compile(
    r"(?:/Users/[^\s\"']+|/home/[^\s\"']+|/private/tmp/[^\s\"']+|/tmp/[^\s\"']+|"
    r"/var/folders/[^\s\"']+|/Volumes/[^\s\"']+|"
    r"[A-Za-z]:\\[^\s\"']+)"
)
REDACTED_PATHS = {"<path>", "<cwd>", "<redacted>"}


class HarnessError(RuntimeError):
    """A useful, user-facing harness failure."""


def fixture_cases() -> list[Path]:
    return sorted(p for p in FIXTURES.iterdir() if p.is_dir())


def jsonl_files(case: Path) -> list[Path]:
    return sorted(case.rglob("*.jsonl"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise HarnessError(f"{path}:{line_no}: fixture is not valid JSON: {exc}") from exc
        if not isinstance(value, dict):
            raise HarnessError(f"{path}:{line_no}: fixture record must be an object")
        records.append(value)
    return records


def walk_strings(value: Any, key: str = "") -> Iterator[tuple[str, str]]:
    if isinstance(value, dict):
        for child_key, child_value in value.items():
            yield from walk_strings(child_value, str(child_key))
    elif isinstance(value, list):
        for child in value:
            yield from walk_strings(child, key)
    elif isinstance(value, str):
        yield key, value


def validate_case(case: Path) -> dict[str, Any]:
    expected_path = case / "expected.json"
    if not expected_path.exists():
        raise HarnessError(f"{case}: expected.json is missing")
    try:
        expected = json.loads(expected_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise HarnessError(f"{expected_path}: invalid JSON: {exc}") from exc
    if expected.get("fixture") != case.name:
        raise HarnessError(f"{expected_path}: fixture name does not match directory")
    if expected.get("agent") != "claude":
        raise HarnessError(f"{expected_path}: agent must be claude")

    for path in jsonl_files(case):
        for record in read_jsonl(path):
            for key, value in walk_strings(record):
                if SECRET_RE.search(value):
                    raise HarnessError(f"{path}: secret-shaped value found in {key}")
                if PRIVATE_PATH_RE.search(value) and value not in REDACTED_PATHS:
                    raise HarnessError(f"{path}: private path found in {key}")
                if key in {"prompt", "command", "text", "rendered", "toolUseResult"}:
                    raise HarnessError(f"{path}: raw transcript field {key} is not allowed")
            message = record.get("message")
            if isinstance(message, dict) and "content" in message and message["content"] != []:
                raise HarnessError(f"{path}: assistant/user content must be empty")
    totals = expected.get("totals", {})
    if totals.get("cost_source") == "unknown" and totals.get("cost_usd") in (0, "0", "0.0", "0.00000000"):
        raise HarnessError(f"{expected_path}: unknown cost must be null, never zero")
    return expected


def validate_fixtures() -> int:
    cases = fixture_cases()
    if not cases:
        raise HarnessError(f"no fixture directories found under {FIXTURES}")
    for case in cases:
        validate_case(case)
        print(f"PASS fixture shape: {case.name}")
    return len(cases)


def decimal_string(value: Any) -> str | None:
    if value is None:
        return None
    try:
        decimal = Decimal(str(value))
    except (InvalidOperation, ValueError) as exc:
        raise HarnessError(f"cost is not numeric: {value!r}") from exc
    if decimal == 0:
        return "0"
    # Compare numbers, not JSON's choice of trailing zero scale. The expected
    # files keep fixed-width arithmetic for review, while serde may emit
    # 0.00103 instead of 0.00103000.
    return format(decimal.normalize(), "f")


def int_value(value: Any) -> int:
    if value is None:
        return 0
    if isinstance(value, bool):
        raise HarnessError(f"boolean token count is invalid: {value!r}")
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise HarnessError(f"token count is not an integer: {value!r}") from exc


def first_value(mapping: dict[str, Any], names: tuple[str, ...]) -> Any:
    for name in names:
        if name in mapping:
            return mapping[name]
    return None


def normalize_bucket(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise HarnessError(f"usage bucket is not an object: {value!r}")
    result: dict[str, Any] = {}
    for canonical, aliases in TOKEN_ALIASES.items():
        result[canonical] = int_value(first_value(value, aliases))
    raw_cost = first_value(value, ("cost_usd", "estimated_cost", "total_cost_usd", "cost"))
    result["cost_usd"] = decimal_string(raw_cost)
    result["cost_source"] = value.get("cost_source", value.get("source", "unknown"))
    result["pricing_version"] = value.get("pricing_version", value.get("price_version"))
    if result["cost_source"] == "unknown" and result["cost_usd"] in {"0", "0.0", "0.00000000"}:
        raise HarnessError("unknown pricing was rendered as zero")
    return result


def unwrap_payload(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise HarnessError("trc usage --json did not return an object")
    current = payload
    for _ in range(3):
        for key in ("data", "usage", "result"):
            child = current.get(key)
            if isinstance(child, dict) and (
                "totals" in child or "by_model" in child or "rows" in child
            ):
                current = child
                break
        else:
            break
    return current


def normalize_models(value: Any) -> dict[str, dict[str, Any]]:
    if value is None:
        return {}
    if isinstance(value, dict):
        return {str(name): normalize_bucket(bucket) for name, bucket in value.items()}
    if isinstance(value, list):
        result: dict[str, dict[str, Any]] = {}
        for row in value:
            if not isinstance(row, dict):
                continue
            name = row.get("model") or row.get("model_id") or row.get("name") or row.get("key")
            if name is not None:
                result[str(name)] = normalize_bucket(row)
        return result
    raise HarnessError("by_model is neither an object nor a list")


def normalize_subagents(value: Any) -> list[dict[str, Any]]:
    if value is None:
        return []
    if not isinstance(value, list):
        raise HarnessError("subagents is not a list")
    result = []
    identity_keys = ("subagent_id", "agent_id", "id")
    for row in value:
        if not isinstance(row, dict):
            raise HarnessError("subagent row is not an object")
        normalized = normalize_bucket(row)
        key = row.get("key")
        key_session = None
        key_subagent = None
        if isinstance(key, str) and " / subagent:" in key:
            key_session, key_subagent = key.split(" / subagent:", 1)
        normalized.update(
            {
                "subagent_id": next((str(row[k]) for k in identity_keys if k in row), key_subagent),
                "session_id": row.get("session_id", row.get("sessionId", key_session)),
                "parent_message_id": row.get("parent_message_id", row.get("parentMessageId")),
            }
        )
        if normalized["subagent_id"] is not None or key_subagent is not None:
            result.append(normalized)
    return sorted(result, key=lambda row: (row["subagent_id"] or "", row["session_id"] or ""))


def normalize_output(payload: Any) -> dict[str, Any]:
    root = unwrap_payload(payload)
    totals_value = root.get("totals", root.get("summary"))
    if totals_value is None:
        raise HarnessError("trc usage --json has no totals object")
    import_value = root.get("import")
    if not isinstance(import_value, dict):
        import_value = {}
    rows = root.get("rows")
    by = root.get("by")
    if by == "model" and rows is not None:
        by_model: Any = rows
    else:
        by_model = root.get("by_model", root.get("models"))
    if "subagents" in root or "children" in root:
        subagents: Any = root.get("subagents", root.get("children"))
    elif by == "session" and isinstance(rows, list):
        subagents = [
            row
            for row in rows
            if isinstance(row, dict)
            and isinstance(row.get("key"), str)
            and " / subagent:" in row["key"]
        ]
    else:
        subagents = []
    return {
        "parse_errors": int_value(
            root.get("parse_errors", root.get("errors_count", import_value.get("malformed_lines", 0)))
        ),
        "events": int_value(root.get("events", root.get("event_count", totals_value.get("event_count", 0)))),
        "subagent_events": int_value(
            root.get("subagent_events", root.get("subagent_event_count", totals_value.get("subagent_events", 0)))
        ),
        "totals": normalize_bucket(totals_value),
        "by_model": normalize_models(by_model),
        "subagents": normalize_subagents(subagents),
    }


def expected_projection(expected: dict[str, Any]) -> dict[str, Any]:
    return {
        "parse_errors": expected["parse_errors"],
        "events": expected["events"],
        "subagent_events": expected["subagent_events"],
        "totals": normalize_bucket(expected["totals"]),
        "by_model": normalize_models(expected.get("by_model")),
        "subagents": normalize_subagents(expected.get("subagents", [])),
    }


def stable_import_projection(actual: dict[str, Any]) -> dict[str, Any]:
    """Fields that should not change when a source is imported a second time."""
    return {key: value for key, value in actual.items() if key != "parse_errors"}


def materialize_case(case: Path, root: Path, mutation: str | None = None, seed: int = 0) -> None:
    destination = root / "home" / ".claude" / "projects" / "-fixture-project"
    destination.mkdir(parents=True, exist_ok=True)
    rng = random.Random(seed)
    for source in jsonl_files(case):
        relative = source.relative_to(case)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        raw_lines = source.read_text(encoding="utf-8").splitlines(keepends=True)
        if mutation == "shuffle":
            rng.shuffle(raw_lines)
        if mutation == "duplicate":
            raw_lines = raw_lines + copy.copy(raw_lines)
        if mutation == "large":
            raw_lines = raw_lines * 1000
        if mutation == "truncate" and raw_lines:
            raw_lines[-1] = raw_lines[-1][: max(1, len(raw_lines[-1]) // 2)]
        if mutation == "unknown-fields":
            transformed = []
            for line in raw_lines:
                record = json.loads(line)
                record["harness_unknown_field"] = {"nested": True, "seed": seed}
                if isinstance(record.get("message"), dict):
                    record["message"]["harness_message_unknown"] = "ignored"
                transformed.append(json.dumps(record, separators=(",", ":")) + "\n")
            raw_lines = transformed
        if mutation == "fuzz":
            transformed = []
            for line in raw_lines:
                record = json.loads(line)
                record["fuzz_unknown_field"] = {"seed": seed, "nested": {"ok": True}}
                if isinstance(record.get("message"), dict):
                    record["message"]["fuzz_message_field"] = [seed, None, False]
                transformed.append(json.dumps(record, separators=(",", ":")) + "\n")
            raw_lines = transformed
            if raw_lines and rng.random() < 0.75:
                raw_lines.insert(rng.randrange(len(raw_lines)), rng.choice(raw_lines))
            # A complete malformed record must be counted and skipped; this
            # exercises the parser's error path without embedding any content.
            raw_lines.append('{"fuzz_malformed":\n')
        target.write_text("".join(raw_lines), encoding="utf-8")


@contextmanager
def isolated_case(
    case: Path,
    binary: Path,
    mutation: str | None = None,
    seed: int = 0,
) -> Iterator[tuple[Path, dict[str, str]]]:
    with tempfile.TemporaryDirectory(prefix="trace-usage-fixture-") as temporary:
        root = Path(temporary)
        materialize_case(case, root, mutation=mutation, seed=seed)
        home = root / "home"
        trace_home = root / "trace-home"
        trace_home.mkdir()
        env = os.environ.copy()
        env.update(
            {
                "HOME": str(home),
                "USERPROFILE": str(home),
                "CLAUDE_CONFIG_DIR": str(home / ".claude"),
                "TRACE_HOME": str(trace_home),
                "TRACE_DB": str(trace_home / "trace.db"),
                "XDG_CONFIG_HOME": str(home / ".config"),
                "NO_COLOR": "1",
            }
        )
        try:
            yield root, env
        finally:
            state_path = trace_home / "daemon.json"
            daemon_pid: int | None = None
            try:
                daemon_pid = int(json.loads(state_path.read_text(encoding="utf-8"))["pid"])
            except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
                pass
            subprocess.run(
                [str(binary), "daemon", "stop"],
                cwd=root,
                env=env,
                capture_output=True,
                text=True,
                timeout=5,
            )
            if daemon_pid is not None:
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    try:
                        os.kill(daemon_pid, 0)
                    except ProcessLookupError:
                        break
                    except PermissionError:
                        pass
                    time.sleep(0.05)


def invoke(binary: Path, root: Path, env: dict[str, str], extra: list[str] | None = None) -> dict[str, Any]:
    command = [str(binary), "usage", "--json"] + (extra or [])
    result = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        if "unrecognized subcommand" in detail or "unknown subcommand" in detail:
            detail += " (Workstreams 2–3 must land trc usage before importer-backed checks can run)"
        raise HarnessError(f"{' '.join(command)} failed with {result.returncode}: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        # A future CLI may print a non-JSON diagnostic before the JSON object;
        # accept only the final complete JSON line, never scrape token values.
        for line in reversed(result.stdout.splitlines()):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
        raise HarnessError(f"{' '.join(command)} did not return JSON")


def normalized_case_output(binary: Path, root: Path, env: dict[str, str]) -> dict[str, Any]:
    """Collect the public model and session projections into one strict fixture view."""
    model = normalize_output(invoke(binary, root, env, ["--by", "model"]))
    session = normalize_output(invoke(binary, root, env, ["--by", "session"]))
    if model["subagent_events"] != session["subagent_events"]:
        raise HarnessError("model/session projections disagree on subagent event count")
    model["subagents"] = session["subagents"]
    return model


def run_golden(binary: Path, case: Path) -> dict[str, Any]:
    expected = expected_projection(validate_case(case))
    with isolated_case(case, binary) as (root, env):
        first = normalized_case_output(binary, root, env)
        second = normalized_case_output(binary, root, env)
        if stable_import_projection(first) != stable_import_projection(second):
            raise HarnessError(f"{case.name}: re-import changed the result\nfirst={first}\nsecond={second}")
        if first != expected:
            raise HarnessError(
                f"{case.name}: importer result differs from expected.json\n"
                f"expected={expected}\nactual={first}"
            )
        return first


def run_properties(binary: Path, case: Path, seed: int, fuzz_rounds: int) -> None:
    expected = expected_projection(validate_case(case))
    for mutation in ("duplicate", "unknown-fields", "shuffle", "large"):
        with isolated_case(case, binary, mutation=mutation, seed=seed) as (root, env):
            started = time.monotonic()
            actual = normalized_case_output(binary, root, env)
            elapsed = time.monotonic() - started
            if mutation != "large" and actual != expected:
                raise HarnessError(f"{case.name}/{mutation}: result changed\nexpected={expected}\nactual={actual}")
            if mutation == "large" and actual != expected:
                raise HarnessError(f"{case.name}/large: repeated message ids were double-counted")
            if mutation == "large" and elapsed > 30:
                raise HarnessError(f"{case.name}/large: importer took {elapsed:.1f}s (>30s)")

    with isolated_case(case, binary, mutation="truncate", seed=seed) as (root, env):
        # A torn final JSON line is recoverable input. The importer must not
        # crash; it may report a parse error and a partial total.
        normalize_output(invoke(binary, root, env))

    for fuzz_seed in range(seed, seed + fuzz_rounds):
        with isolated_case(case, binary, mutation="fuzz", seed=fuzz_seed) as (root, env):
            first = normalized_case_output(binary, root, env)
            second = normalized_case_output(binary, root, env)
            if stable_import_projection(first) != stable_import_projection(second):
                raise HarnessError(
                    f"{case.name}/fuzz seed {fuzz_seed}: re-import changed the result\n"
                    f"first={first}\nsecond={second}"
                )
            if first["events"] != expected["events"]:
                raise HarnessError(
                    f"{case.name}/fuzz seed {fuzz_seed}: duplicate/malformed input changed event count"
                )

    with isolated_case(case, binary, seed=seed) as (root, env):
        daemon = subprocess.run([str(binary), "daemon", "start"], cwd=root, env=env, text=True, capture_output=True)
        if daemon.returncode != 0:
            raise HarnessError(f"{case.name}/concurrency: daemon start failed: {daemon.stderr.strip()}")
        try:
            processes = [
                subprocess.Popen(
                    [str(binary), "usage", "--json", "--by", "model"],
                    cwd=root,
                    env=env,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                for _ in range(4)
            ]
            outputs = []
            for process in processes:
                stdout, stderr = process.communicate(timeout=30)
                if process.returncode != 0:
                    raise HarnessError(f"{case.name}/concurrency: importer failed: {stderr.strip()}")
                outputs.append(normalize_output(json.loads(stdout)))
            if any(output != outputs[0] for output in outputs[1:]):
                raise HarnessError(f"{case.name}/concurrency: concurrent imports disagree")
        finally:
            subprocess.run([str(binary), "daemon", "stop"], cwd=root, env=env, capture_output=True, text=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/trc"))
    parser.add_argument("--fixture", action="append", help="fixture directory name; repeatable")
    parser.add_argument("--all", action="store_true", dest="all_cases", help="run every fixture")
    parser.add_argument("--properties", action="store_true", help="run mutation and concurrency cases")
    parser.add_argument("--fuzz-rounds", type=int, default=8, help="deterministic fuzz rounds per fixture")
    parser.add_argument("--validate-only", action="store_true", help="validate sanitized fixtures without trc")
    parser.add_argument("--seed", type=int, default=42)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        validate_fixtures()
        if args.validate_only:
            return 0
        if not args.all_cases and not args.fixture:
            raise HarnessError("choose --all, --fixture NAME, or --validate-only")
        binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
        if not binary.exists():
            raise HarnessError(f"binary not found: {binary}; build it first")
        selected = fixture_cases() if args.all_cases else [FIXTURES / name for name in args.fixture]
        for case in selected:
            if not case.is_dir():
                raise HarnessError(f"unknown fixture: {case.name}")
            run_golden(binary, case)
            print(f"PASS golden importer: {case.name}")
            if args.properties:
                if args.fuzz_rounds < 0:
                    raise HarnessError("--fuzz-rounds must be non-negative")
                run_properties(binary, case, args.seed, args.fuzz_rounds)
                print(f"PASS importer properties: {case.name}")
        return 0
    except (HarnessError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        print(f"usage fixture harness: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
