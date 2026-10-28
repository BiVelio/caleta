#!/usr/bin/env python3
"""End-to-end smoke test; requires a built caleta binary and Python 3."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("CALETA_BIN", ROOT / "target" / "debug" / "caleta"))
PORT = int(os.environ.get("CALETA_TEST_PORT", "18089"))
BASE = f"http://127.0.0.1:{PORT}"
KEY = "e2e-public-key-with-enough-length"


def request(path, authorized=True, data=None):
    headers = {"Content-Type": "application/json"}
    if authorized:
        headers["Authorization"] = f"Bearer {KEY}"
    req = urllib.request.Request(BASE + path, headers=headers, data=data)
    return urllib.request.urlopen(req, timeout=10)


with tempfile.TemporaryDirectory() as temporary:
    temp = Path(temporary)
    backend = temp / "mock-llama-server"
    shutil.copy(ROOT / "tests" / "mock_llama_server.py", backend)
    if os.name == "nt":
        launcher = temp / "mock-llama-server.cmd"
        launcher.write_text(
            f'@"{sys.executable}" "{backend}" %*\n',
            encoding="utf-8",
        )
        backend = launcher
    else:
        backend.chmod(0o700)
    model = temp / "model.gguf"
    model.write_bytes(b"not-a-real-model; deterministic mock artifact")
    digest = hashlib.sha256(model.read_bytes()).hexdigest()
    manifest = temp / "model.toml"
    manifest.write_text(f'''
schema = "caleta.dev/model/v1"
id = "tests/mock"
license = "Apache-2.0"
source = "file://tests/mock"
[artifact]
path = "model.gguf"
sha256 = "{digest}"
bytes = {model.stat().st_size}
[backend]
kind = "llama.cpp"
executable = {json.dumps(str(backend))}
sha256 = "{hashlib.sha256(backend.read_bytes()).hexdigest()}"
[runtime]
context_size = 128
gpu_layers = 0
''')
    subprocess.run([BINARY, "verify", str(manifest), "--json"], check=True, capture_output=True)
    env = dict(os.environ, CALETA_API_KEY=KEY)
    record = temp / "execution.json"
    process = subprocess.Popen(
        [BINARY, "serve", str(manifest), "--listen", f"127.0.0.1:{PORT}", "--record-out", record],
        env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    try:
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError(process.stderr.read().decode())
            try:
                request("/health", authorized=False).close()
                break
            except Exception:
                time.sleep(0.1)
        else:
            raise RuntimeError("server did not become ready")

        try:
            request("/v1/models", authorized=False)
            raise AssertionError("missing API key was accepted")
        except urllib.error.HTTPError as error:
            assert error.code == 401
        models = json.load(request("/v1/models"))
        assert models["data"][0]["id"] == "tests/mock"
        stream = request("/v1/chat/completions", data=json.dumps({
            "model": "tests/mock", "messages": [], "stream": True
        }).encode()).read().decode()
        assert "private" in stream and "data: [DONE]" in stream
        conformance = temp / "conformance.json"
        subprocess.run([
            BINARY, "conformance", "--url", BASE, "--model", "tests/mock",
            "--output", conformance,
        ], env=env, check=True, capture_output=True)
        assert json.loads(conformance.read_text())["passed"]
    finally:
        if os.name == "nt":
            subprocess.run(
                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                check=True,
                capture_output=True,
            )
        else:
            process.send_signal(signal.SIGINT)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    if os.name != "nt":
        assert json.loads(record.read_text())["prompt_content_recorded"] is False

    results = temp / "benchmark.json"
    subprocess.run([
        BINARY, "bench", str(manifest), "--output", results,
        "--runs", "2", "--warmup", "1", "--max-tokens", "8",
    ], check=True, capture_output=os.name != "nt")
    report = json.loads(results.read_text())
    assert report["summary"]["successful_runs"] == 2
    assert "prompt" not in report
    assert report["model_sha256"] == digest
    assert all(run["status"] == "ok" for run in report["runs"])
    if os.name == "nt":
        cleanup_env = dict(os.environ, CALETA_TEST_TEMP=str(temp))
        subprocess.run([
            "powershell.exe", "-NoProfile", "-Command",
            "Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like ('*' + $env:CALETA_TEST_TEMP + '*') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }",
        ], env=cleanup_env, check=True, capture_output=True)

if os.name == "nt":
    print("E2E PASS: verify, serve, auth, models, SSE, conformance and bench (forced shutdown)")
else:
    print("E2E PASS: verify, serve, auth, models, SSE, conformance, audit and bench")
