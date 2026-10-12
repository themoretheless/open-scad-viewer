"""Lossless JSON value partition for Node's single-string size limit.

Qualification input only: preserve every root property and ordered case. The
consumer binds these temporary records to the original byte SHA, not their
reserialized representation. No native geometric claims are inferred here.
"""
import hashlib
import json
import sys
from pathlib import Path


def main():
    source, destination = map(Path, sys.argv[1:])
    data = source.read_bytes()
    # JSON.parse preserves the sign of the integer spelling -0 as well.
    manifest = json.loads(data, parse_int=lambda text: -0.0 if text == "-0" else int(text))
    cases = manifest.pop("cases")
    if not isinstance(cases, list):
        raise ValueError("Sweep manifest cases must be an ordered array")
    records = []
    for index, case in enumerate(cases):
        name = f"case-{index:06d}.json"
        encoded = json.dumps(case, separators=(",", ":"), allow_nan=False).encode()
        (destination / name).write_bytes(encoded)
        records.append({"file": name, "sha256": hashlib.sha256(encoded).hexdigest()})
    metadata = {
        "schema": "sweep-manifest-partition/1",
        "originalSha256": hashlib.sha256(data).hexdigest(),
        "root": manifest,
        "caseCount": len(cases),
        "records": records,
    }
    (destination / "index.json").write_text(
        json.dumps(metadata, separators=(",", ":"), allow_nan=False)
    )


if __name__ == "__main__":
    main()
