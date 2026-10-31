#!/usr/bin/env python3
import argparse
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SUMMARY = re.compile(r"(\d+) passed; (\d+) failed")


def run(command, log_path, env=None):
    result = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    log_path.write_text(result.stdout, encoding="utf-8")
    if result.returncode != 0:
        raise RuntimeError(f"command failed with exit {result.returncode}: {' '.join(command)}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--runner-os", default=platform.system().lower())
    args = parser.parse_args()
    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)

    test_output = run(["cargo", "test", "--locked"], output / "cargo-test.log")
    counts = [(int(passed), int(failed)) for passed, failed in SUMMARY.findall(test_output)]
    passed = sum(item[0] for item in counts)
    failed = sum(item[1] for item in counts)
    if not counts:
        raise RuntimeError("no Rust test summaries found")

    e2e_report = output / "mock-e2e.json"
    env = dict(
        os.environ,
        CALETA_BIN=os.environ.get("CALETA_BIN", "target/debug/caleta"),
        CALETA_E2E_REPORT=str(e2e_report),
    )
    run(
        [sys.executable, "tests/e2e_mock.py"],
        output / "e2e.log",
        env=env,
    )
    mock = json.loads(e2e_report.read_text(encoding="utf-8"))
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    evidence = {
        "schema": "caleta.dev/software-validation/v1",
        "os": args.runner_os,
        "arch": platform.machine().lower(),
        "source_revision": revision,
        "unit_tests": {"passed": passed, "failed": failed},
        "mock_e2e": {"passed": mock["passed"]},
        "passed": failed == 0 and mock["passed"],
        "commands": ["cargo test --locked", "python3 tests/e2e_mock.py"],
    }
    (output / "unit_tests.json").write_text(
        json.dumps(evidence, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Validation evidence: {passed} unit tests passed, {failed} failed; mock E2E passed")


if __name__ == "__main__":
    main()
