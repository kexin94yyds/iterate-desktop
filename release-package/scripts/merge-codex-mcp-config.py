#!/usr/bin/env python3
"""Merge the Iterate MCP entry into Codex config without dropping user settings."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

TIMEOUT_SECONDS = 315_360_000


def find_section(text: str) -> tuple[int, int] | None:
    pattern = r"(?m)^\[(?:mcp_servers\.\"iterate-zhi\"|mcp_servers\.iterate-zhi|mcp_servers\.cunzhi)\]\s*$"
    match = re.search(pattern, text)
    if not match:
        return None
    next_header = re.search(r"(?m)^\[", text[match.end() :])
    end = match.end() + next_header.start() if next_header else len(text)
    return match.start(), end


def replace_setting(block: str, name: str, value: str) -> str:
    pattern = rf"(?m)^\s*{re.escape(name)}\s*=.*$"
    if re.search(pattern, block):
        return re.sub(pattern, f"{name} = {value}", block, count=1)
    return block


def merge_config(text: str, command: str) -> str:
    section = find_section(text)
    command_value = json.dumps(command, ensure_ascii=False)
    if section is None:
        suffix = "" if not text or text.endswith("\n") else "\n"
        if text.strip():
            suffix += "\n"
        return text + suffix + (
            '[mcp_servers."iterate-zhi"]\n'
            f"command = {command_value}\nargs = []\n"
            f"tool_timeout_sec = {TIMEOUT_SECONDS}\n"
        )

    start, end = section
    block = replace_setting(text[start:end], "command", command_value)
    if not re.search(r"(?m)^\s*args\s*=", block):
        command_end = re.search(r"(?m)^\s*command\s*=.*$", block)
        insertion = command_end.end() if command_end else 0
        block = block[:insertion] + "\nargs = []" + block[insertion:]
    block = replace_setting(block, "tool_timeout_sec", str(TIMEOUT_SECONDS))
    if not re.search(r"(?m)^\s*tool_timeout_sec\s*=", block):
        block = block.rstrip("\n") + f"\ntool_timeout_sec = {TIMEOUT_SECONDS}\n"
    return text[:start] + block + text[end:]


if len(sys.argv) != 3:
    raise SystemExit(f"usage: {Path(sys.argv[0]).name} CONFIG_PATH MCP_SERVER_PATH")
path = Path(sys.argv[1]).expanduser()
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(merge_config(path.read_text() if path.exists() else "", sys.argv[2]))
print(path)
