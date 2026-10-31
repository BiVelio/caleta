#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directories", nargs="+", type=Path)
    parser.add_argument("--expected-os", type=int, required=True)
    parser.add_argument("--local-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--expected-unit-tests", type=int, default=15)
    args = parser.parse_args()
    if len(args.directories) != args.expected_os:
        raise SystemExit("the number of runner directories does not match --expected-os")

    validations = []
    for directory in args.directories:
        unit_path = directory / "unit_tests.json"
        mock_path = directory / "mock-e2e.json"
        unit = json.loads(unit_path.read_text(encoding="utf-8"))
        mock = json.loads(mock_path.read_text(encoding="utf-8"))
        if not unit["passed"] or not mock["passed"]:
            raise SystemExit(f"validation failed in {directory}")
        validations.append((unit, mock))
    runners = [unit["os"] for unit, _ in validations]
    if len(set(runners)) != len(runners):
        raise SystemExit("runner operating systems must be distinct")
    revisions = {unit["source_revision"] for unit, _ in validations}
    if len(revisions) != 1:
        raise SystemExit("runner source revisions differ")
    passed = sum(unit["passed"] for unit, _ in validations)
    if passed != args.expected_os:
        raise SystemExit(f"only {passed}/{args.expected_os} runner validations passed")

    local_unit = json.loads((args.local_dir / "unit_tests.json").read_text(encoding="utf-8"))
    local_mock = json.loads((args.local_dir / "mock-e2e.json").read_text(encoding="utf-8"))
    if local_unit["unit_tests"]["passed"] != args.expected_unit_tests or local_unit["unit_tests"]["failed"] != 0:
        raise SystemExit(f"local unit-test counts do not match {args.expected_unit_tests} passed and 0 failed")
    if not local_mock["passed"]:
        raise SystemExit("local mock E2E failed")
    if local_mock["benchmark"]["successful_runs"] != 2 or local_mock["benchmark"]["failed_runs"] != 0:
        raise SystemExit("local mock benchmark counts do not match two successes and zero failures")
    if local_mock["artifact"]["bytes"] != 45:
        raise SystemExit("local mock artifact size is not 45 bytes")
    evidence = {
        "schema": "caleta.dev/software-validation-summary/v1",
        "unit_tests": {
            "passed": local_unit["unit_tests"]["passed"],
            "failed": local_unit["unit_tests"]["failed"],
            "source": str(args.local_dir / "unit_tests.json"),
        },
        "mock_e2e": {
            "successful_runs": local_mock["benchmark"]["successful_runs"],
            "failed_runs": local_mock["benchmark"]["failed_runs"],
            "artifact_bytes": local_mock["artifact"]["bytes"],
        },
        "hosted_validation": {
            "operating_systems": len(validations),
            "passed": passed,
            "failed": len(validations) - passed,
            "runners": [unit["os"] for unit, _ in validations],
        },
        "passed": True,
    }
    if args.check:
        expected = json.loads(args.output.read_text(encoding="utf-8"))
        expected.pop("provenance", None)
        if evidence != expected:
            raise SystemExit("stale software-validation evidence")
        print("software-validation evidence matches all runner outputs")
        return
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
