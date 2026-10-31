#!/usr/bin/env python3
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def capture(path, pattern):
    text = (ROOT / path).read_text(encoding="utf-8")
    match = re.search(pattern, text)
    if not match:
        raise RuntimeError(f"parameter not found in {path}: {pattern}")
    return match.group(1)


parameters = {
    "schema": "caleta.dev/implementation-parameters/v1",
    "internal_api_key_bytes": int(capture("src/backend.rs", r"0_u8; (\d+)\]")),
    "max_request_mib": int(capture("src/server.rs", r"MAX_REQUEST_BYTES: usize = (\d+)")),
    "minimum_public_key_characters": int(capture("src/server.rs", r"key\.len\(\) < (\d+)")),
    "request_timeout_seconds": int(capture("src/server.rs", r"\.timeout\(Duration::from_secs\((\d+)\)\)")),
    "max_concurrent_requests": int(capture("src/server.rs", r"ConcurrencyLimitLayer::new\((\d+)\)")),
    "backend_version_timeout_seconds": int(capture("src/backend.rs", r"tokio::time::timeout\(\s*Duration::from_secs\((\d+)\)")),
    "rss_sample_ms": int(capture("src/bench.rs", r"from_millis\((\d+)\)")),
    "verification_buffer_mib": int(capture("src/verify.rs", r"vec!\[0_u8; (\d+)")) // int(capture("src/verify.rs", r"1024 \* (\d+)")),
    "minimum_rust_version": float(capture("Cargo.toml", r"rust-version = \"([^\"]+)\"")),
}
print(json.dumps(parameters, indent=2))
