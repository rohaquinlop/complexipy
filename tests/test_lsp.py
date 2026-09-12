"""Tests for the language server entry point.

The server talks LSP on stdio, so these tests act as a minimal client.
The handshake test reads stdout through a raw pipe and fails if anything
that is not a protocol frame appears there, which is the only way to
catch a stray Python write on the protocol channel.
"""

from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import threading
from pathlib import Path

import pytest

import complexipy
from complexipy._complexipy import run_lsp

TIMEOUT = 15.0
READ_CHUNK = 65536

STRICT_CONFIG = "max-complexity-allowed = 2\n"

HEAVY = """def light():
    return 1


def heavy(a, b):
    if a:
        if b:
            return 1
    return 0
"""

SUPPRESSED = """def heavy(a, b):  # noqa: complexipy
    if a:
        if b:
            return 1
    return 0
"""


class LspSession:
    """Minimal LSP client over the stdio transport.

    A reader thread parses stdout into frames and hands them to the main
    thread through a queue. That avoids polling the pipe, because `select`
    only accepts sockets on Windows, and it reads through a raw file
    descriptor so every byte the server writes is inspected as it arrives.
    """

    def __init__(self, root: Path, extra_args: list[str] | None = None) -> None:
        self.process = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "complexipy.cli",
                "lsp",
                *(extra_args or []),
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(root),
        )
        self.stdout_fd = self.process.stdout.fileno()
        self.inbox: queue.Queue = queue.Queue()
        self.buffer = b""
        self.request_id = 0
        self.closed = False
        self.reader = threading.Thread(target=self.read_frames, daemon=True)
        self.reader.start()

    def __enter__(self) -> LspSession:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def close(self) -> None:
        if self.closed:
            return
        self.closed = True
        if self.process.poll() is None:
            self.process.kill()
        self.process.communicate()

    def fail(self, message: str) -> None:
        if self.process.poll() is None:
            self.process.kill()
        stderr = b""
        if self.process.stderr is not None:
            stderr = self.process.stderr.read()
        raise AssertionError(
            f"{message}; exit={self.process.poll()}; "
            f"stderr={stderr.decode(errors='replace')}"
        )

    def send(self, payload: dict) -> None:
        body = json.dumps(payload).encode()
        frame = b"Content-Length: %d\r\n\r\n" % len(body) + body
        self.process.stdin.write(frame)
        self.process.stdin.flush()

    def request(self, method: str, params: dict) -> dict:
        self.request_id += 1
        self.send(
            {
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": method,
                "params": params,
            }
        )
        return self.await_response(self.request_id)

    def notify(self, method: str, params: dict) -> None:
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def await_response(self, request_id: int) -> dict:
        while True:
            message = self.read_message()
            if message.get("id") == request_id:
                assert "error" not in message, message
                return message["result"]

    def await_notification(self, method: str) -> dict | None:
        while True:
            message = self.read_message()
            if message.get("method") == method:
                return message.get("params")

    def read_message(self) -> dict:
        try:
            kind, payload = self.inbox.get(timeout=TIMEOUT)
        except queue.Empty:
            self.fail(f"timed out after {TIMEOUT}s waiting for the server")
        if kind == "error":
            raise payload
        return payload

    def read_frames(self) -> None:
        try:
            while True:
                self.inbox.put(("message", self.read_frame()))
        except BaseException as error:  # noqa: BLE001 - forwarded to the test thread
            self.inbox.put(("error", error))

    def read_frame(self) -> dict:
        headers: dict[bytes, bytes] = {}
        while True:
            line = self.read_line()
            if line == b"":
                break
            name, _, value = line.partition(b":")
            headers[name.strip().lower()] = value.strip()
        length = headers.get(b"content-length")
        if length is None:
            raise AssertionError(
                f"stdout carried a non-protocol line: {headers!r}"
            )
        return json.loads(self.read_exactly(int(length)))

    def read_line(self) -> bytes:
        line = bytearray()
        while True:
            byte = self.read_exactly(1)
            if byte == b"\n":
                return bytes(line)
            if byte != b"\r":
                line += byte

    def read_exactly(self, count: int) -> bytes:
        while len(self.buffer) < count:
            chunk = os.read(self.stdout_fd, READ_CHUNK)
            if not chunk:
                raise AssertionError(
                    "the server closed stdout before answering"
                )
            self.buffer += chunk
        chunk, self.buffer = self.buffer[:count], self.buffer[count:]
        return chunk

    def handshake(self, root: Path, capabilities: dict | None = None) -> dict:
        result = self.request(
            "initialize",
            {
                "processId": None,
                "rootUri": root.as_uri(),
                "workspaceFolders": [{"uri": root.as_uri(), "name": "root"}],
                "capabilities": capabilities or {},
            },
        )
        self.notify("initialized", {})
        return result

    def shutdown(self) -> int:
        self.request("shutdown", {})
        self.notify("exit", {})
        return self.process.wait(timeout=TIMEOUT)


def document(uri: str, text: str) -> dict:
    return {
        "textDocument": {
            "uri": uri,
            "languageId": "python",
            "version": 1,
            "text": text,
        }
    }


def hints_request(uri: str) -> dict:
    return {
        "textDocument": {"uri": uri},
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 1000, "character": 0},
        },
    }


def test_run_lsp_is_exposed_but_not_public() -> None:
    assert callable(run_lsp)
    assert "run_lsp" not in complexipy.__all__
    assert not hasattr(complexipy, "run_lsp")


def test_full_session_keeps_stdout_protocol_only(tmp_path: Path) -> None:
    (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
    heavy = (tmp_path / "heavy.py").as_uri()
    suppressed = (tmp_path / "suppressed.py").as_uri()

    with LspSession(tmp_path) as session:
        result = session.handshake(tmp_path)

        assert result["capabilities"]["textDocumentSync"] == 1
        assert result["capabilities"]["inlayHintProvider"] is True
        assert result["capabilities"]["hoverProvider"] is True
        assert "codeActionProvider" not in result["capabilities"]
        assert result["serverInfo"]["name"] == "complexipy-lsp"

        session.notify("textDocument/didOpen", document(heavy, HEAVY))
        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert diagnostics["uri"] == heavy
        assert len(diagnostics["diagnostics"]) == 1
        assert (
            diagnostics["diagnostics"][0]["message"]
            == "cognitive complexity 3 exceeds the allowed 2"
        )
        assert diagnostics["diagnostics"][0]["severity"] == 2
        assert diagnostics["diagnostics"][0]["source"] == "complexipy"
        assert diagnostics["diagnostics"][0]["code"] == "cognitive-complexity"

        hints = session.request("textDocument/inlayHint", hints_request(heavy))

        assert len(hints) == 1
        assert hints[0]["label"] == "cognitive: 3"
        assert hints[0]["position"] == {"line": 4, "character": 16}

        session.notify("textDocument/didOpen", document(suppressed, SUPPRESSED))
        suppressed_diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert suppressed_diagnostics["uri"] == suppressed
        assert suppressed_diagnostics["diagnostics"] == []

        assert session.shutdown() == 0


def test_no_ignore_reports_suppressed_functions(tmp_path: Path) -> None:
    (tmp_path / "complexipy.toml").write_text(
        STRICT_CONFIG + "no-ignore = true\n"
    )
    uri = (tmp_path / "suppressed.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, SUPPRESSED))
        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert len(diagnostics["diagnostics"]) == 1
        assert (
            diagnostics["diagnostics"][0]["message"]
            == "cognitive complexity 3 exceeds the allowed 2"
        )
        assert session.shutdown() == 0


def test_invalid_exclude_pattern_keeps_the_server_running(
    tmp_path: Path,
) -> None:
    (tmp_path / "complexipy.toml").write_text(
        'max-complexity-allowed = 2\nexclude = ["[unclosed"]\n'
    )
    uri = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, HEAVY))
        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert len(diagnostics["diagnostics"]) == 1
        assert session.shutdown() == 0


def test_requests_narrowed_to_a_range_drop_other_hints(tmp_path: Path) -> None:
    (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
    uri = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, HEAVY))
        session.await_notification("textDocument/publishDiagnostics")

        first_line = {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 1, "character": 0},
            },
        }
        heavy_line = {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 4, "character": 0},
                "end": {"line": 4, "character": 100},
            },
        }

        assert session.request("textDocument/inlayHint", first_line) == []
        assert len(session.request("textDocument/inlayHint", heavy_line)) == 1
        assert session.shutdown() == 0


def test_hint_refresh_is_requested_when_the_client_supports_it(
    tmp_path: Path,
) -> None:
    (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
    uri = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(
            tmp_path, {"workspace": {"inlayHint": {"refreshSupport": True}}}
        )
        session.notify("textDocument/didOpen", document(uri, HEAVY))
        session.await_notification("textDocument/publishDiagnostics")

        assert session.await_notification("workspace/inlayHint/refresh") is None
        assert session.shutdown() == 0


def test_syntax_errors_clear_output_without_failing(tmp_path: Path) -> None:
    (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
    uri = (tmp_path / "broken.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, "def broken(:\n"))
        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert diagnostics["diagnostics"] == []
        assert (
            session.request("textDocument/inlayHint", hints_request(uri)) == []
        )
        assert session.shutdown() == 0


def test_lsp_ignores_trailing_arguments(tmp_path: Path) -> None:
    (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
    heavy = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path, extra_args=["--stdio"]) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(heavy, HEAVY))
        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert len(diagnostics["diagnostics"]) == 1
        assert session.shutdown() == 0


def test_closed_stdin_does_not_fall_back_to_the_cli(tmp_path: Path) -> None:
    process = subprocess.run(
        [sys.executable, "-m", "complexipy.cli", "lsp"],
        input=b"",
        capture_output=True,
        cwd=str(tmp_path),
        timeout=TIMEOUT,
    )

    assert process.returncode == 1
    assert process.stdout == b""


def test_other_arguments_still_reach_the_cli(tmp_path: Path) -> None:
    process = subprocess.run(
        [sys.executable, "-m", "complexipy.cli", "--version"],
        capture_output=True,
        cwd=str(tmp_path),
        timeout=TIMEOUT,
    )

    assert process.returncode == 0
    assert b"complexipy" in process.stdout


def test_configuration_reload_is_honoured(tmp_path: Path) -> None:
    uri = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, HEAVY))

        assert (
            session.await_notification("textDocument/publishDiagnostics")[
                "diagnostics"
            ]
            == []
        )

        (tmp_path / "complexipy.toml").write_text(STRICT_CONFIG)
        session.notify("workspace/didChangeConfiguration", {"settings": {}})

        diagnostics = session.await_notification(
            "textDocument/publishDiagnostics"
        )

        assert len(diagnostics["diagnostics"]) == 1
        assert (
            session.request("textDocument/inlayHint", hints_request(uri)) != []
        )
        assert session.shutdown() == 0


def test_missing_config_is_not_an_error(tmp_path: Path) -> None:
    uri = (tmp_path / "heavy.py").as_uri()

    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.notify("textDocument/didOpen", document(uri, HEAVY))

        assert (
            session.await_notification("textDocument/publishDiagnostics")[
                "diagnostics"
            ]
            == []
        )
        assert (
            session.request("textDocument/inlayHint", hints_request(uri)) == []
        )
        assert session.shutdown() == 0


def test_shutdown_without_exit_keeps_the_server_alive(tmp_path: Path) -> None:
    with LspSession(tmp_path) as session:
        session.handshake(tmp_path)
        session.request("shutdown", {})

        with pytest.raises(subprocess.TimeoutExpired):
            session.process.wait(timeout=1.0)

        session.notify("exit", {})
        assert session.process.wait(timeout=TIMEOUT) == 0
