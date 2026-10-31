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
    args = parser.parse_args()
    if len(args.directories) != args.expected_os:
        raise SystemExit("the number of runner directories does not match --expected-os")

    validations = []
    for directory in args.directories:
        unit_path = directory / "unit_tests.json"
        mock_path = directory / "mock-e2e.json"
        unit = json.loads(unit_path.read_text(encoding="utf-8"))
        mock = json.loads(mock_path.read_text(encoding="utf-8"))
        validations.append((unit, mock))
    passed = sum(unit["passed"] for unit, _ in validations)
    if passed != args.expected_os:
        raise SystemExit(f"only {passed}/{args.expected_os} runner validations passed")

    local_unit = json.loads((args.local_dir / "unit_tests.json").read_text(encoding="utf-8"))
    local_mock = json.loads((args.local_dir / "mock-e2e.json").read_text(encoding="utf-8"))
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
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
