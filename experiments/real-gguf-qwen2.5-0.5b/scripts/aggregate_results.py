#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import statistics
import sys

ROOT = Path(__file__).resolve().parents[1]
INPUTS = [ROOT / "results" / f"benchmark-{index:02d}.json" for index in range(1, 4)]


def number_word(value):
    words = {0: "zero", 1: "one", 2: "two", 3: "three", 4: "four", 5: "five"}
    return words.get(value, str(value))


def validate(reports):
    schema = "caleta.dev/benchmark/v1"
    keys = [
        "model_id", "model_sha256", "backend_version", "os", "arch",
        "context_size", "gpu_layers", "threads", "max_tokens", "seed",
        "temperature", "warmup_runs", "prompt_sha256", "prompt_bytes",
    ]
    first = reports[0]
    if first["schema"] != schema:
        raise ValueError("unexpected benchmark schema")
    for report in reports:
        if report["schema"] != schema:
            raise ValueError("mixed benchmark schemas")
        if len(report["runs"]) != 5:
            raise ValueError("each report must contain five measured runs")
        for key in keys:
            if report[key] != first[key]:
                raise ValueError(f"experimental configuration differs in {key}")
        if any(run["status"] != "ok" for run in report["runs"]):
            raise ValueError("cannot certify zero failed requests when failures are present")


def aggregate():
    reports = [json.loads(path.read_text(encoding="utf-8")) for path in INPUTS]
    validate(reports)
    warm = [
        run["metrics"]
        for report in reports
        for run in report["runs"]
        if run["status"] == "ok"
    ]
    decode = [metrics["decode_tps"] for metrics in warm if metrics["decode_tps"] is not None]
    ttft = [metrics["ttft_ms"] for metrics in warm]
    cold = [report["cold_start_ms"] for report in reports]
    failed = sum(
        run["status"] != "ok"
        for report in reports
        for run in report["runs"]
    )
    return {
        "schema": "caleta.dev/validation-summary/v1",
        "model_id": reports[0]["model_id"],
        "independent_cold_starts": len(reports),
        "measured_warm_requests": len(warm),
        "failed_measured_requests": failed,
        "warmup_requests_per_cold_start": reports[0]["warmup_runs"],
        "cold_start_ms": {
            "median": statistics.median(cold),
            "min": min(cold),
            "max": max(cold),
        },
        "ttft_ms": {
            "median": statistics.median(ttft),
            "min": min(ttft),
            "max": max(ttft),
        },
        "decode_tps": {
            "median": statistics.median(decode),
            "min": min(decode),
            "max": max(decode),
            "sample_stdev": statistics.stdev(decode),
        },
        "notes": [
            f"{number_word(len(reports)).capitalize()} independent invocations; {number_word(reports[0]['warmup_runs'])} warmups and five measured requests per invocation.",
            "Repeated prompts within each backend process may benefit from caching.",
            "TTFT includes HTTP and queueing; observed prefill is not a cold kernel measurement.",
            f"CPU-only {reports[0]['os'].capitalize()} {reports[0]['arch']}, {number_word(reports[0]['threads'])} threads, context {reports[0]['context_size']}, {reports[0]['model_id'].split(':')[-1].upper()}, seed {reports[0]['seed']}, temperature {reports[0]['temperature']}.",
            "No cross-engine comparison, energy, memory, quality, or safety evaluation.",
        ],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    summary = aggregate()
    if args.check:
        expected = json.loads((ROOT / "results" / "summary.json").read_text(encoding="utf-8"))
        expected.pop("provenance", None)
        if summary != expected:
            print("summary.json does not match the aggregate of the raw benchmark reports", file=sys.stderr)
            return 1
        print("summary.json matches all aggregate statistics from the raw benchmark reports")
        return 0
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
