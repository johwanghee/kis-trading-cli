#!/usr/bin/env python3
from __future__ import annotations

import json
import pathlib
import sys
from collections import defaultdict


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: render_cli_reference.py <manifest-json> <output-markdown>",
            file=sys.stderr,
        )
        return 1

    manifest_path = pathlib.Path(sys.argv[1]).expanduser().resolve()
    output_path = pathlib.Path(sys.argv[2]).expanduser().resolve()

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    markdown = render_reference(manifest)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(markdown, encoding="utf-8")
    print(f"wrote {output_path}")
    return 0


def render_reference(manifest: dict) -> str:
    grouped = defaultdict(list)
    for entry in manifest["apis"]:
        grouped[entry["category_id"]].append(entry)

    lines: list[str] = []
    lines.append("# CLI Reference")
    lines.append("")
    lines.append(
        "> Generated from `data/kis_api_manifest.json`. Edit the manifest or generator, not this file."
    )
    lines.append("")
    lines.append(f"- Source repo: `{manifest.get('source_repo', '')}`")
    lines.append(f"- Source commit: `{manifest['source_commit']}`")
    lines.append(f"- Categories: `{manifest['category_count']}`")
    lines.append(f"- APIs: `{manifest['api_count']}`")
    lines.append("")
    lines.append("## Top-level commands")
    lines.append("")
    lines.append("- `config`: local config file path/template management")
    lines.append("- `catalog`: embedded manifest summary/export")
    for category in manifest["categories"]:
        lines.append(f"- `{category['id']}`: {normalize_text(category['introduce'])} ({category['api_count']} APIs)")
    lines.append("")
    lines.append("## Global options")
    lines.append("")
    lines.append("- `--env <demo|real>`")
    lines.append("- `--config <PATH>`")
    lines.append("- `--compact`")
    lines.append("")

    for category in manifest["categories"]:
        category_id = category["id"]
        entries = sorted(grouped[category_id], key=lambda item: display_command_name(item))

        lines.append(f"## `{category_id}`")
        lines.append("")
        lines.append(f"- Description: {normalize_text(category['introduce'])}")
        lines.append(f"- Config source file: `{category['config_file']}`")
        lines.append(f"- API count: `{category['api_count']}`")
        lines.append("")
        lines.append("| Command | 설명 | Method | Path | Required flags |")
        lines.append("| --- | --- | --- | --- | ---: |")

        for entry in entries:
            command = f"`{display_command_name(entry)}`"
            description = escape_table(normalize_text(entry["display_name"]))
            method = f"`{entry['http_method']}`"
            path = f"`{entry['api_path']}`"
            required = str(required_visible_param_count(entry))
            lines.append(f"| {command} | {description} | {method} | {path} | {required} |")

        lines.append("")

    return "\n".join(lines) + "\n"


def display_command_name(entry: dict) -> str:
    command_name = entry["command_name"]
    if entry["category_id"] == "auth" and command_name.startswith("auth-"):
        return command_name.removeprefix("auth-")
    return command_name


def required_visible_param_count(entry: dict) -> int:
    return sum(1 for param in entry["params"] if not param["hidden"] and param["required"])


def normalize_text(value: str) -> str:
    return " ".join((value or "").split())


def escape_table(value: str) -> str:
    return value.replace("|", "\\|")


if __name__ == "__main__":
    raise SystemExit(main())
