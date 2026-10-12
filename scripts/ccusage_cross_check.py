#!/usr/bin/env python3
"""Opt-in Trace versus ccusage comparison for the same Claude JSONL tree.

This script never installs or invokes ccusage implicitly in CI. Provide JSON
files captured from both tools, or explicitly provide a command template such
as `ccusage daily --json --since {since}`. It prints only aggregate day/model
rows, never transcript text or paths.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
from decimal import Decimal
from pathlib import Path
from typing import Any


TOKENS = ("input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens", "reasoning_tokens")
TOKEN_ALIASES = {
    "input_tokens": ("input_tokens", "inputTokens"),
    "output_tokens": ("output_tokens", "outputTokens"),
    "cache_read_tokens": (
        "cache_read_tokens",
        "cache_read_input_tokens",
        "cacheReadInputTokens",
    ),
    "cache_write_tokens": (
        "cache_write_tokens",
        "cache_creation_input_tokens",
        "cacheCreationInputTokens",
    ),
    "reasoning_tokens": ("reasoning_tokens", "reasoningTokens"),
}


def as_int(value: Any) -> int:
    try:
        return int(value or 0)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"invalid token count: {value!r}") from exc


def as_decimal(value: Any) -> Decimal | None:
    if value is None or value == "":
        return None
    return Decimal(str(value))


def normalized_row(row: dict[str, Any], default_model: str = "<all>") -> dict[str, Any] | None:
    day = row.get("day") or row.get("date") or row.get("dt")
    if day is None and isinstance(row.get("key"), str) and len(row["key"]) == 10:
        candidate = row["key"]
        if candidate[4] == "-" and candidate[7] == "-":
            day = candidate
    if day is None and row.get("timestamp"):
        day = str(row["timestamp"])[:10]
    if day is None:
        return None
    model = row.get("model") or row.get("model_id") or row.get("model_name") or default_model
    result = {
        field: as_int(next((row[name] for name in TOKEN_ALIASES[field] if name in row), 0))
        for field in TOKENS
    }
    cost = row.get(
        "cost_usd",
        row.get("cost", row.get("total_cost_usd", row.get("totalCost"))),
    )
    result["cost_usd"] = as_decimal(cost)
    return {"day": str(day)[:10], "model": str(model), **result}


def rows_from(value: Any, default_model: str = "<all>") -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if isinstance(value, list):
        for item in value:
            if isinstance(item, dict):
                row = normalized_row(item, default_model)
                if row:
                    rows.append(row)
                else:
                    rows.extend(rows_from(item, default_model))
    elif isinstance(value, dict):
        direct = normalized_row(value, default_model)
        if direct:
            rows.append(direct)
        else:
            for key, child in value.items():
                child_model = str(key) if "model" in key.lower() else default_model
                rows.extend(rows_from(child, child_model))
    return rows


def load_rows(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    return rows_from_value(value, path)


def rows_from_value(value: Any, source: Path | None = None) -> list[dict[str, Any]]:
    rows = rows_from(value)
    if not rows:
        label = source if source is not None else "captured output"
        raise ValueError(f"{label}: no day/model aggregate rows found")
    return rows


def run_json(command: list[str], env: dict[str, str], label: str) -> Any:
    result = subprocess.run(command, env=env, text=True, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(f"{label} failed with {result.returncode}: {result.stderr.strip()}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"{label} did not return JSON") from exc


def aggregate(rows: list[dict[str, Any]]) -> dict[tuple[str, str], dict[str, Any]]:
    result: dict[tuple[str, str], dict[str, Any]] = {}
    for row in rows:
        key = (row["day"], row["model"])
        current = result.setdefault(key, {field: 0 for field in TOKENS} | {"cost_usd": Decimal("0")})
        for field in TOKENS:
            current[field] += row[field]
        if row["cost_usd"] is not None:
            current["cost_usd"] += row["cost_usd"]
    return result


def fmt(value: Any) -> str:
    if value is None:
        return "unavailable"
    if isinstance(value, Decimal):
        return format(value, "f")
    return str(value)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trc", type=Path, default=Path("target/release/trc"))
    parser.add_argument("--since", required=True, help="YYYY-MM-DD lower bound")
    parser.add_argument("--trace-json", type=Path, help="pre-captured trc usage --by day --json output")
    parser.add_argument("--ccusage-json", type=Path, help="pre-captured ccusage JSON output")
    parser.add_argument("--ccusage-command", help="explicit command template; {since} is replaced")
    parser.add_argument("--claude-root", type=Path, default=Path.home() / ".claude" / "projects")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        env = os.environ.copy()
        env["CLAUDE_CONFIG_DIR"] = str(args.claude_root.parent)
        if args.trace_json:
            trace_rows = load_rows(args.trace_json)
        else:
            if not args.trc.exists():
                raise ValueError(f"Trace binary not found: {args.trc}")
            trace = run_json([str(args.trc), "usage", "--by", "day", "--since", args.since, "--json"], env, "trc usage")
            trace_rows = rows_from_value(trace)

        if args.ccusage_json:
            cc_rows = load_rows(args.ccusage_json)
        else:
            if not args.ccusage_command:
                if shutil.which("ccusage") is None:
                    raise ValueError("ccusage is not installed; pass --ccusage-json or explicit --ccusage-command (opt-in)")
                command = ["ccusage", "daily", "--json", "--since", args.since]
            else:
                command = shlex.split(args.ccusage_command.format(since=args.since))
            cc = run_json(command, env, "ccusage")
            cc_rows = rows_from_value(cc)

        trace = aggregate([row for row in trace_rows if row["day"] >= args.since])
        ccusage = aggregate([row for row in cc_rows if row["day"] >= args.since])
        print("day\tmodel\tTrace tokens\tccusage tokens\tTrace cost\tccusage cost\tdelta")
        for key in sorted(set(trace) | set(ccusage)):
            left = trace.get(key, {field: 0 for field in TOKENS} | {"cost_usd": Decimal("0")})
            right = ccusage.get(key, {field: 0 for field in TOKENS} | {"cost_usd": Decimal("0")})
            left_tokens = sum(left[field] for field in TOKENS)
            right_tokens = sum(right[field] for field in TOKENS)
            print(
                f"{key[0]}\t{key[1]}\t{left_tokens}\t{right_tokens}\t"
                f"{fmt(left['cost_usd'])}\t{fmt(right['cost_usd'])}\t{left_tokens - right_tokens} tokens"
            )
        print("\nExpected drift to investigate:")
        print("- Claude Code transcript retention and ccusage file selection can differ.")
        print("- Repeated streaming records must be deduped by agent/session/message identity.")
        print("- Cache read/write TTLs, long-context tiers, provider routing, and rounding can change cost totals.")
        print("- A plan/console number is not necessarily the same as public API-rate computed cost.")
        print("- Trace's one-dimension usage endpoint labels a day-only capture <all>; pass day/model captures to compare per model.")
        return 0
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as exc:
        print(f"ccusage cross-check: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
