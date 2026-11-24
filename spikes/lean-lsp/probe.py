#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 The lean4losophy Authors
"""Minimal LSP probe for the M0 Lean spike.

Starts `lake serve`, opens Lean4losophy/Spike.lean, waits for diagnostics, and prints
them. Exits nonzero when initialization or diagnostics fail.

This is spike code, not product code: it speaks just enough JSON-RPC to prove
the lifecycle. The real client lives in crates/lean4losophy-lean at M4.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
TARGET = HERE / "Lean4losophy" / "Spike.lean"
TIMEOUT = float(os.environ.get("LEAN_TIMEOUT", "120"))


class Client:
    def __init__(self) -> None:
        self.proc = subprocess.Popen(
            ["lake", "serve"],
            cwd=HERE,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self.next_id = 1
        self.lock = threading.Lock()
        self.responses: dict[int, dict] = {}
        self.diagnostics: dict[str, list[dict]] = {}
        self.diagnostics_event = threading.Event()

    def send(self, payload: dict) -> None:
        body = json.dumps(payload)
        header = f"Content-Length: {len(body.encode('utf-8'))}\r\n\r\n"
        assert self.proc.stdin is not None
        with self.lock:
            self.proc.stdin.write(header + body)
            self.proc.stdin.flush()

    def request(self, method: str, params: dict) -> int:
        request_id = self.next_id
        self.next_id += 1
        self.send(
            {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        )
        return request_id

    def notify(self, method: str, params: dict) -> None:
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def read_loop(self) -> None:
        assert self.proc.stdout is not None
        while True:
            length = None
            while True:
                line = self.proc.stdout.readline()
                if not line:
                    return
                line = line.strip()
                if not line:
                    break
                if line.lower().startswith("content-length:"):
                    length = int(line.split(":", 1)[1].strip())
            if length is None:
                continue
            chunks = []
            remaining = length
            while remaining > 0:
                chunk = self.proc.stdout.read(remaining)
                if not chunk:
                    return
                chunks.append(chunk)
                remaining -= len(chunk)
            try:
                message = json.loads("".join(chunks))
            except json.JSONDecodeError:
                continue
            if "id" in message and "method" not in message:
                self.responses[message["id"]] = message
            elif message.get("method") == "textDocument/publishDiagnostics":
                uri = message["params"]["uri"]
                self.diagnostics[uri] = message["params"]["diagnostics"]
                if uri.endswith("Spike.lean"):
                    self.diagnostics_event.set()

    def wait_response(self, request_id: int, deadline: float) -> dict | None:
        while time.monotonic() < deadline:
            if request_id in self.responses:
                return self.responses[request_id]
            time.sleep(0.05)
        return None

    def close(self) -> None:
        try:
            request_id = self.request("shutdown", {})
            self.wait_response(request_id, time.monotonic() + 5)
            self.notify("exit", {})
        except (BrokenPipeError, ValueError):
            pass
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def main() -> int:
    if not TARGET.exists():
        print(f"error: fixture missing: {TARGET}", file=sys.stderr)
        return 1

    client = Client()
    reader = threading.Thread(target=client.read_loop, daemon=True)
    reader.start()
    deadline = time.monotonic() + TIMEOUT

    init_id = client.request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": HERE.as_uri(),
            "capabilities": {
                "textDocument": {"publishDiagnostics": {"versionSupport": True}}
            },
        },
    )
    if client.wait_response(init_id, deadline) is None:
        print("error: no initialize response", file=sys.stderr)
        client.close()
        return 1
    print("initialize: ok")
    client.notify("initialized", {})

    client.notify(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": TARGET.as_uri(),
                "languageId": "lean",
                "version": 1,
                "text": TARGET.read_text(encoding="utf-8"),
            }
        },
    )

    if not client.diagnostics_event.wait(TIMEOUT):
        print("error: no diagnostics within the timeout", file=sys.stderr)
        client.close()
        return 1

    found = client.diagnostics.get(TARGET.as_uri(), [])
    print(f"Lean4losophy/Spike.lean: {len(found)} diagnostics")
    for diagnostic in found:
        start = diagnostic.get("range", {}).get("start", {})
        severity = {1: "error", 2: "warning", 3: "info", 4: "hint"}.get(
            diagnostic.get("severity"), "unknown"
        )
        message = diagnostic.get("message", "").splitlines()[0][:120]
        print(f"  ({start.get('line')}, {start.get('character')}) {severity}: {message}")

    client.close()
    if not found:
        print("error: diagnostics arrived but the list was empty", file=sys.stderr)
        return 1
    print("spike: diagnostics received")
    return 0


if __name__ == "__main__":
    sys.exit(main())
