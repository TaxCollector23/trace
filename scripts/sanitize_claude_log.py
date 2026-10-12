#!/usr/bin/env python3
"""Turn a Claude Code JSONL transcript into a safe, text-free fixture.

The sanitizer is intentionally allow-list based for strings. It preserves
usage/model/linkage metadata and replaces prompts, tool payloads, paths, ids,
and secret-shaped values. It never prints a source record or a source path.

Examples:

  python3 scripts/sanitize_claude_log.py --input session.jsonl --output fixture.jsonl
  python3 scripts/sanitize_claude_log.py --input ~/.claude/projects \
      --output tests/fixtures/claude/my-case --fail-on-malformed
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


SECRET_RE = re.compile(
    r"(?:AKIA[0-9A-Z]{16}|(?:sk|ghp|xox[baprs])[-_][A-Za-z0-9_-]{12,}|"
    r"Bearer\s+[A-Za-z0-9._~-]{12,}|-----BEGIN [^-]+ PRIVATE KEY-----)"
)
PATH_RE = re.compile(
    r"(?:/Users/|/home/|/private/tmp/|/tmp/|/var/folders/|/Volumes/|[A-Za-z]:\\)"
)
ID_KEYS = {
    "id": "message",
    "uuid": "uuid",
    "parentuuid": "message",
    "logicalparentuuid": "message",
    "sessionid": "session",
    "agentid": "agent",
    "messageid": "message",
    "sourcetooluseid": "tool",
    "sourcetoolassistantuuid": "message",
}
PATH_KEYS = {
    "cwd",
    "path",
    "filepath",
    "file_path",
    "filename",
    "gitbranch",
    "projectpath",
    "project_path",
}
TEXT_KEYS = {
    "command",
    "content",
    "description",
    "diff",
    "input",
    "lastprompt",
    "messagecontent",
    "prompt",
    "rendered",
    "stderr",
    "stdout",
    "text",
    "toolresult",
    "tooluseresult",
}
SAFE_STRING_KEYS = {
    "agentname",
    "attributionagent",
    "entrypoint",
    "inferencegeo",
    "model",
    "origin",
    "role",
    "service_tier",
    "source",
    "stop_reason",
    "stopreason",
    "subtype",
    "timestamp",
    "type",
    "version",
}


class Sanitizer:
    def __init__(self) -> None:
        self.ids: dict[tuple[str, str], str] = {}
        self.next_id: dict[str, int] = {}

    def stable_id(self, kind: str, value: str) -> str:
        key = (kind, value)
        if key not in self.ids:
            number = self.next_id.get(kind, 0) + 1
            self.next_id[kind] = number
            self.ids[key] = f"{kind}-{number:02d}"
        return self.ids[key]

    def string(self, key: str, value: str) -> str:
        lowered = key.lower().replace("-", "_")
        compact = lowered.replace("_", "")
        if compact in ID_KEYS:
            return self.stable_id(ID_KEYS[compact], value)
        if lowered in PATH_KEYS or compact in {k.replace("_", "") for k in PATH_KEYS}:
            return "<path>"
        if compact in TEXT_KEYS or lowered in TEXT_KEYS:
            return "<redacted>"
        if SECRET_RE.search(value):
            return SECRET_RE.sub("<secret>", value)
        if PATH_RE.search(value):
            return "<path>"
        if lowered in SAFE_STRING_KEYS or compact in {k.replace("_", "") for k in SAFE_STRING_KEYS}:
            return value
        # Unknown free-form strings can contain prompts, tool output, or
        # provider metadata that later turns out to be sensitive. Do not make
        # contributors audit an allow-list after every Claude Code release.
        return "<redacted>"

    def value(self, key: str, value: Any) -> Any:
        lowered = key.lower().replace("-", "_")
        compact = lowered.replace("_", "")
        if compact == "content":
            return []
        if isinstance(value, dict):
            return {child_key: self.value(str(child_key), child_value) for child_key, child_value in value.items()}
        if isinstance(value, list):
            return [self.value(key, child) for child in value]
        if isinstance(value, str):
            return self.string(key, value)
        # Numeric usage/cost/timing values are safe and are the only values
        # the importer needs. Booleans are metadata such as isSidechain.
        if isinstance(value, (int, float, bool)) or value is None:
            return value
        return "<redacted>"


def sanitize_record(record: dict[str, Any], sanitizer: Sanitizer) -> dict[str, Any]:
    sanitized = sanitizer.value("record", record)
    if not isinstance(sanitized, dict):
        raise ValueError("record did not remain an object")
    # A content list can occur under message.content and must be empty even if
    # a future field name is not recognized by the allow-list.
    message = sanitized.get("message")
    if isinstance(message, dict) and "content" in message:
        message["content"] = []
    return sanitized


def process_file(source: Path, destination: Path | None, fail_on_malformed: bool) -> tuple[int, int]:
    sanitizer = Sanitizer()
    output = sys.stdout if destination is None else destination.open("w", encoding="utf-8")
    written = 0
    malformed = 0
    try:
        with source.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle, 1):
                try:
                    value = json.loads(line)
                    if not isinstance(value, dict):
                        raise ValueError("record is not an object")
                    safe = sanitize_record(value, sanitizer)
                except (json.JSONDecodeError, ValueError) as exc:
                    malformed += 1
                    if fail_on_malformed:
                        raise ValueError(f"malformed record at line {line_number}: {exc}") from exc
                    continue
                output.write(json.dumps(safe, separators=(",", ":"), sort_keys=True) + "\n")
                written += 1
    finally:
        if destination is not None:
            output.close()
    return written, malformed


def check_safe(root: Path) -> None:
    for path in sorted(root.rglob("*.jsonl")) if root.is_dir() else [root]:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if SECRET_RE.search(line) or PATH_RE.search(line):
                raise ValueError(f"{path.name}:{line_number}: unsafe value remains")
            value = json.loads(line)
            message = value.get("message") if isinstance(value, dict) else None
            if isinstance(message, dict) and message.get("content") not in (None, []):
                raise ValueError(f"{path.name}:{line_number}: message content was not removed")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="JSONL file or Claude projects directory")
    parser.add_argument("--output", type=Path, help="output JSONL file or directory; stdout for a single file when omitted")
    parser.add_argument("--fail-on-malformed", action="store_true")
    parser.add_argument("--check", type=Path, help="check an already-sanitized file or directory")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.check is not None:
            check_safe(args.check)
            print(f"sanitized: {args.check}")
            return 0
        if args.input is None:
            raise ValueError("--input is required unless --check is used")
        source = args.input
        if source.is_dir():
            if args.output is None:
                raise ValueError("--output is required when --input is a directory")
            args.output.mkdir(parents=True, exist_ok=True)
            total_written = total_malformed = 0
            for file in sorted(source.rglob("*.jsonl")):
                target = args.output / file.relative_to(source)
                target.parent.mkdir(parents=True, exist_ok=True)
                written, malformed = process_file(file, target, args.fail_on_malformed)
                total_written += written
                total_malformed += malformed
            check_safe(args.output)
            print(f"sanitized {total_written} records; skipped {total_malformed} malformed records", file=sys.stderr)
            return 0
        if not source.is_file():
            raise ValueError(f"input does not exist: {source}")
        written, malformed = process_file(source, args.output, args.fail_on_malformed)
        if args.output is not None:
            check_safe(args.output)
        print(f"sanitized {written} records; skipped {malformed} malformed records", file=sys.stderr)
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"sanitize_claude_log: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
