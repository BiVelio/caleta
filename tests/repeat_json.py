#!/usr/bin/env python3
import json
from pathlib import Path
import sys

source = Path(sys.argv[1])
print(json.dumps(json.loads(source.read_text(encoding="utf-8")), indent=2))
