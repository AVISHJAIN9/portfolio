#!/usr/bin/env python3
"""
tools/run_timed.py
Usage: python3 tools/run_timed.py <seconds> <logfile> -- <command...>
Executes a command under a strict process-group timeout, streaming output to <logfile>.
Prints exactly one line at the end: EXIT=<code> TIMEOUT=<yes|no> ELAPSED=<s> LOG=<logfile>
Exits with code 124 on timeout.
"""

import sys
import os
import time
import signal
import subprocess
from pathlib import Path

def main():
    if len(sys.argv) < 4 or "--" not in sys.argv:
        print("Usage: python3 tools/run_timed.py <seconds> <logfile> -- <command...>", file=sys.stderr)
        sys.exit(2)

    sep_idx = sys.argv.index("--")
    timeout_sec = float(sys.argv[1])
    logfile_path = Path(sys.argv[2]).resolve()
    cmd = sys.argv[sep_idx + 1:]

    logfile_path.parent.mkdir(parents=True, exist_ok=True)

    t0 = time.time()
    timed_out = False
    return_code = 0

    with open(logfile_path, "w", encoding="utf-8", buffering=1) as log_f:
        try:
            proc = subprocess.Popen(
                cmd,
                stdout=log_f,
                stderr=subprocess.STDOUT,
                start_new_session=True
            )
        except Exception as e:
            log_f.write(f"Failed to start command {cmd}: {e}\n")
            print(f"EXIT=1 TIMEOUT=no ELAPSED=0.00 LOG={logfile_path}")
            sys.exit(1)

        pgid = proc.pid
        try:
            proc.wait(timeout=timeout_sec)
            return_code = proc.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            return_code = 124
            log_f.write(f"\n[run_timed] ERROR: Command timed out after {timeout_sec}s. Terminating process group {pgid}...\n")
            try:
                os.killpg(pgid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            
            # Wait up to 5s for graceful exit
            t_wait_start = time.time()
            while time.time() - t_wait_start < 5.0:
                if proc.poll() is not None:
                    break
                time.sleep(0.2)

            if proc.poll() is None:
                log_f.write(f"[run_timed] Force killing process group {pgid} with SIGKILL...\n")
                try:
                    os.killpg(pgid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                proc.wait()

    elapsed = time.time() - t0
    timeout_str = "yes" if timed_out else "no"
    print(f"EXIT={return_code} TIMEOUT={timeout_str} ELAPSED={elapsed:.2f} LOG={logfile_path}")
    sys.exit(return_code)

if __name__ == "__main__":
    main()
