#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import platform
import subprocess
import sys


def command(arguments):
    result = subprocess.run(arguments, capture_output=True, text=True)
    return {
        "command": arguments,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--backend", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    records = {
        "platform": command([sys.executable, "-c", "import platform; print(platform.platform())"]),
        "rustc": command(["rustc", "-vV"]),
        "caleta_revision": command(["git", "rev-parse", "HEAD"]),
    }
    if platform.system() == "Windows":
        records["host"] = command([
            "powershell.exe", "-NoProfile", "-Command",
            "$cs=Get-CimInstance Win32_ComputerSystem; $cpu=Get-CimInstance Win32_Processor; $os=Get-CimInstance Win32_OperatingSystem; [pscustomobject]@{Model=$cs.Model;CPU=$cpu.Name;Cores=$cpu.NumberOfCores;LogicalProcessors=$cpu.NumberOfLogicalProcessors;RAM_GiB=[math]::Round($cs.TotalPhysicalMemory/1GB,2);OS=$os.Caption;OSVersion=$os.Version;Architecture=$os.OSArchitecture} | ConvertTo-Json",
        ])
    else:
        records["host"] = command(["uname", "-a"])
    if args.backend:
        records["backend_version"] = command([str(args.backend), "--version"])
    evidence = {
        "schema": "caleta.dev/system-metadata/v1",
        "collection_method": "actual command execution",
        "records": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"System metadata written to {args.output}")


if __name__ == "__main__":
    main()
