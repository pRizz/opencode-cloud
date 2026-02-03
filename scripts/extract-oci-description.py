#!/usr/bin/env python3
import pathlib
import re
import sys


def main() -> int:
    dockerfile_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("Dockerfile")
    try:
        dockerfile = dockerfile_path.read_text()
    except OSError as exc:
        print(f"Failed to read Dockerfile at {dockerfile_path}: {exc}", file=sys.stderr)
        return 2

    match = re.search(
        r'org\.opencontainers\.image\.description\s*=\s*"([^"]+)"',
        dockerfile,
    )
    if not match:
        print(
            "Missing org.opencontainers.image.description label in Dockerfile",
            file=sys.stderr,
        )
        return 3

    print(match.group(1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
