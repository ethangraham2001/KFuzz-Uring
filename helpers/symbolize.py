#!/usr/bin/env python3
import sys
import subprocess
import os


def usage():
    print(f"Usage: {sys.argv[0]} <path_to_vmlinux> [coverage_file]")
    print("  coverage_file defaults to 'coverage.txt'")
    sys.exit(1)


def main():
    if len(sys.argv) < 2:
        usage()

    vmlinux = sys.argv[1]
    cov_file = sys.argv[2] if len(sys.argv) > 2 else "coverage.txt"

    if not os.path.exists(cov_file):
        print(f"Error: {cov_file} not found.")
        sys.exit(1)

    print(f"[+] Reading PCs from {cov_file}...")
    with open(cov_file, "r") as f:
        # Read unique PCs, ignore empty lines
        pcs = {line.strip() for line in f if line.strip()}

    print(f"[+] Resolving {len(pcs)} unique PCs against {vmlinux}...")

    # We batch the calls to addr2line for speed
    # Command: addr2line -e vmlinux -f <address>
    args = ["addr2line", "-e", vmlinux, "-f", "-i"]  # -f: func name, -i: inlines

    # Start the process
    proc = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    # Feed all PCs to stdin
    input_str = "\n".join(pcs)
    stdout, stderr = proc.communicate(input=input_str)

    if proc.returncode != 0:
        print(f"Error running addr2line: {stderr}")
        sys.exit(1)

    # Parse output. addr2line output is alternating lines:
    # Function Name
    # File:Line
    lines = stdout.strip().splitlines()

    hits = {}  # Map "Function" -> Count of PCs hit in that function

    for i in range(0, len(lines), 2):
        if i + 1 >= len(lines):
            break
        func_name = lines[i]
        file_line = lines[i + 1]

        # Cleanup '?' entries
        if func_name == "??":
            continue

        # Key can be just function name, or file for more granularity
        key = f"{func_name} ({file_line})"
        hits[key] = hits.get(key, 0) + 1

    print("\n[+] Coverage Report (Deepest functions first):")
    print("-" * 60)

    # Sort by function name to group them, or by "depth" logic if we knew call graph.
    # For now, let's sort alphabetically to group function PCs together.
    for loc in sorted(hits.keys()):
        print(f" {hits[loc]:4d} hits | {loc}")


if __name__ == "__main__":
    main()
