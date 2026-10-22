#!/usr/bin/env python3
"""Measure real stdio didChange latency for 40 packages with 50 functions each.

Usage: python tools/measure-lsp-semantic-cache.py target/debug/ond-lsp.exe
Run against both the previous and updated binary on the same machine.
Only the Python standard library is required; fixtures live in a temporary folder.
"""

import argparse
import json
import os
from pathlib import Path
import queue
import statistics
import subprocess
import tempfile
import threading
import time


def measure(binary, root, tests):
    root.mkdir()
    (root / "ond.toml").write_text('[project]\nname="cache_benchmark"\n', encoding="utf-8")
    normal = "package main\nfunc main() {}\n"
    (root / "main.ond").write_text(normal, encoding="utf-8")
    for index in range(40):
        package = root / f"p{index:02}"
        package.mkdir()
        source = f"package p{index:02}\n" + "".join(
            f"func F{number}() -> u32 {{ return {number} }}\n" for number in range(50)
        )
        (package / "api.ond").write_text(source, encoding="utf-8")
        if tests:
            (package / "test.ond").write_text(
                f"package p{index:02}\nfunc TestOK() {{}}\n", encoding="utf-8"
            )
    environment = dict(os.environ)
    process = subprocess.Popen(
        [str(binary), "stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, env=environment,
    )
    messages = queue.Queue()
    errors = []

    def read_messages():
        try:
            while True:
                length = None
                while True:
                    line = process.stdout.readline()
                    if not line:
                        return
                    if line == b"\r\n":
                        break
                    if line.lower().startswith(b"content-length:"):
                        length = int(line.split(b":", 1)[1])
                if length is None:
                    raise RuntimeError("Missing Content-Length")
                body = bytearray()
                while len(body) < length:
                    part = process.stdout.read(length - len(body))
                    if not part:
                        raise RuntimeError("Incomplete LSP response")
                    body.extend(part)
                messages.put(json.loads(body))
        except Exception as error:
            messages.put(error)

    def read_errors():
        for line in iter(process.stderr.readline, b""):
            errors.append(line.decode(errors="replace"))

    readers = [threading.Thread(target=read_messages), threading.Thread(target=read_errors)]
    for reader in readers:
        reader.start()

    def send(message):
        body = json.dumps({"jsonrpc": "2.0", **message}).encode()
        process.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        process.stdin.flush()

    def response(identifier):
        while True:
            message = messages.get(timeout=60)
            if isinstance(message, Exception):
                raise message
            if message.get("method") == "textDocument/publishDiagnostics":
                if any(diagnostic.get("severity") == 1 for diagnostic in message["params"]["diagnostics"]):
                    raise RuntimeError(f"Unexpected diagnostics: {message}")
            if message.get("id") == identifier:
                if "error" in message:
                    raise RuntimeError(str(message))
                return

    uri = (root / "main.ond").as_uri()

    def barrier(identifier):
        send({"id": identifier, "method": "textDocument/documentSymbol", "params": {"textDocument": {"uri": uri}}})
        response(identifier)

    try:
        send({"id": 1, "method": "initialize", "params": {}})
        response(1)
        send({"method": "initialized", "params": {}})
        start = time.perf_counter()
        send({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "ond", "version": 1, "text": normal,
        }}})
        barrier(2)
        opened = (time.perf_counter() - start) * 1000
        edits = []
        for index in range(3):
            start = time.perf_counter()
            send({"method": "textDocument/didChange", "params": {
                "textDocument": {"uri": uri, "version": index + 2},
                "contentChanges": [{"text": normal + "\n" * (index + 1)}],
            }})
            barrier(index + 3)
            edits.append(round((time.perf_counter() - start) * 1000, 2))
        send({"id": 6, "method": "shutdown", "params": None})
        response(6)
        return {"open_ms": round(opened, 2), "edit_ms": edits, "median_ms": round(statistics.median(edits), 2)}
    except Exception as error:
        raise RuntimeError(f"{error}\n{''.join(errors[-10:])}") from error
    finally:
        try:
            if process.poll() is None:
                send({"method": "exit"})
                process.wait(timeout=10)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            for reader in readers:
                reader.join(timeout=10)
            process.stdin.close()
            process.stdout.close()
            process.stderr.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    arguments = parser.parse_args()
    binary = arguments.binary.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="ond-lsp-semantic-cache-") as temporary:
        root = Path(temporary)
        results = {"with_tests" if tests else "without_tests": measure(binary, root / str(tests), tests) for tests in (False, True)}
    print(json.dumps({"binary": str(binary), "results": results}, indent=2))


if __name__ == "__main__":
    main()
