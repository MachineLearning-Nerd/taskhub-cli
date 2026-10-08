#!/usr/bin/env python3
"""Measures the CLI for docs/benchmarks.md: startup time, latency percentiles and requests per command.

Runs the release binary against a TaskHub test server (never production). Each sample is a fresh process,
the way an agent calls the CLI. Creates and submits items in the given project.

Env: E2E_DEV_TOKEN, E2E_TESTER_TOKEN (token files), E2E_ORIGIN (default http://127.0.0.1:3100),
E2E_PROJECT (default E2E), TASKHUB_BIN (default target/release/taskhub), BENCH_SAMPLES (default 20).
"""

import json
import os
import pathlib
import statistics
import subprocess
import sys
import tempfile
import threading
import time

HERE = pathlib.Path(__file__).resolve().parent
BIN = os.path.abspath(os.environ.get("TASKHUB_BIN", HERE.parent / "target/release/taskhub"))
ORIGIN = os.environ.get("E2E_ORIGIN", "http://127.0.0.1:3100")
PROJECT = os.environ.get("E2E_PROJECT", "E2E")
SAMPLES = int(os.environ.get("BENCH_SAMPLES", "20"))
if "dineshjinjala.com" in ORIGIN:
    sys.exit("bench: refusing to run against production")
WORK = pathlib.Path(tempfile.mkdtemp(prefix="taskhub-bench-", dir=os.environ.get("TMPDIR")))


def env(user):
    home = WORK / user
    home.mkdir(mode=0o700, exist_ok=True)
    clean = {k: v for k, v in os.environ.items() if not k.startswith("TASKHUB_")}
    return clean | {"HOME": str(home), "XDG_CONFIG_HOME": str(home / ".config"), "XDG_STATE_HOME": str(home / ".local/state")}


def run(user, *args, stdin=None):
    started = time.perf_counter()
    done = subprocess.run([BIN, "--json", *args], env=env(user), input=stdin, capture_output=True, text=True)
    elapsed = (time.perf_counter() - started) * 1000
    if done.returncode != 0:
        sys.exit(f"bench: taskhub {' '.join(args)} failed ({done.returncode}): {done.stdout}{done.stderr}")
    return json.loads(done.stdout) if done.stdout.strip() else None, elapsed


def login(user, token_file, origin=ORIGIN):
    run(user, "auth", "login", "--with-token", "--origin", origin, stdin=pathlib.Path(token_file).read_text())


def summary(name, samples):
    samples = sorted(samples)
    p95 = samples[min(len(samples) - 1, round(0.95 * (len(samples) - 1)))]
    print(f"| {name} | {len(samples)} | {statistics.median(samples):.1f} | {p95:.1f} | {samples[0]:.1f} | {samples[-1]:.1f} |")


def help_ms():
    started = time.perf_counter()
    subprocess.run([BIN, "--help"], capture_output=True, check=True)
    return (time.perf_counter() - started) * 1000


def requests_per_command():
    """Counts HTTP requests per command through the logging proxy."""
    proxy = subprocess.Popen([sys.executable, HERE / "e2e-delay-proxy.py", ORIGIN, "NONE", "^$", "0"],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    port = proxy.stdout.readline().strip()
    login("counted", os.environ["E2E_DEV_TOKEN"], f"http://127.0.0.1:{port}")
    lines = []
    threading.Thread(target=lambda: lines.extend(line.strip() for line in proxy.stderr), daemon=True).start()

    def drain():
        time.sleep(0.5)  # the proxy logs before it replies; this only lets the reader thread catch up

    drain()
    run("tester", "items", "create", "--project", PROJECT, "--type", "task", "--priority", "low",
        "--title", "Benchmark request count")
    counts = {}

    def counted(label, *args):
        seen = len(lines)
        out = run("counted", *args)[0]
        drain()
        counts[label] = lines[seen:]
        return out

    key = counted("next --claim", "next", "--claim", "--project", PROJECT)["data"]["key"]
    counted("show", "show", key)
    counted("comment", "comment", key, "--body", "Counting requests.")
    counted("submit (no files)", "submit", key, "--summary", "Counted.", "--testing", "None: benchmark only.")
    proxy.kill()
    return counts


def main():
    login("dev", os.environ["E2E_DEV_TOKEN"])
    login("tester", os.environ["E2E_TESTER_TOKEN"])
    if os.environ.get("BENCH_ONLY") == "requests":
        for label, lines in requests_per_command().items():
            print(f"- `{label}`: {len(lines)} ({', '.join(lines)})")
        return
    print(f"taskhub {run('dev', 'version')[0]['data']}")
    print(f"{SAMPLES} samples per command, origin {ORIGIN}\n")
    print("| Command | Samples | p50 ms | p95 ms | min ms | max ms |")
    print("| --- | --- | --- | --- | --- | --- |")
    summary("`version` (startup, no network)", [run("dev", "version")[1] for _ in range(SAMPLES)])
    summary("`--help` (startup, no network)", [help_ms() for _ in range(SAMPLES)])

    keys = [run("tester", "items", "create", "--project", PROJECT, "--type", "task", "--priority", "low",
                "--title", f"Benchmark item {i + 1}")[0]["data"]["key"] for i in range(SAMPLES)]
    claims = []
    claimed = []
    for _ in keys:
        out, ms = run("dev", "next", "--claim", "--project", PROJECT)
        claims.append(ms)
        claimed.append(out["data"]["key"])
    shows = [run("dev", "show", key)[1] for key in claimed]
    submits = [run("dev", "submit", key, "--summary", "Benchmark submission.", "--testing", "None: benchmark only.")[1]
               for key in claimed]
    summary("`show`", shows)
    summary("`next --claim`", claims)
    summary("`submit` (no files)", submits)

    print("\nHTTP requests per command:")
    for label, lines in requests_per_command().items():
        print(f"- `{label}`: {len(lines)} ({', '.join(lines)})")
    print(f"\nItems used: {claimed[0]}..{claimed[-1]} (left in Dev Done).")


main()
