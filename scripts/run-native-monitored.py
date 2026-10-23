#!/usr/bin/env python3
"""Run one native command in its own session and record real wait statuses.

Never restarts or resumes a stopped child. A stop remains observable while the
same process is awaited. Exit status reflects the child, not the observer.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def run(command, directory):
    directory.mkdir(parents=True, exist_ok=False)
    with (directory / 'output.log').open('w') as output, (directory / 'events.jsonl').open('w', buffering=1) as events:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output, stderr=output, start_new_session=True)
        def record(kind, **fields):
            events.write(json.dumps(dict(time=time.time(), pid=child.pid, kind=kind, **fields)) + '\n')
        def forward(signum, frame):
            try:
                os.killpg(child.pid, signum)
            except ProcessLookupError:
                pass
        previous = {sig: signal.signal(sig, forward) for sig in (signal.SIGINT, signal.SIGTERM)}
        record('started', observer=os.getpid(), processGroup=child.pid, command=command)
        try:
            while True:
                try:
                    _, status = os.waitpid(child.pid, os.WUNTRACED | os.WCONTINUED)
                except InterruptedError:
                    continue
                if os.WIFSTOPPED(status):
                    record('stopped', signal=signal.Signals(os.WSTOPSIG(status)).name)
                elif os.WIFCONTINUED(status):
                    record('continued')
                elif os.WIFEXITED(status):
                    code = os.WEXITSTATUS(status)
                    child.returncode = code
                    record('exited', exitCode=code)
                    return code
                elif os.WIFSIGNALED(status):
                    sig = os.WTERMSIG(status)
                    child.returncode = -sig
                    record('signalled', signal=signal.Signals(sig).name)
                    return 128 + sig
        finally:
            for sig, handler in previous.items():
                signal.signal(sig, handler)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='Fresh evidence directory')
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command:
        parser.error('A native command is required')
    raise SystemExit(run(command, args.output))
