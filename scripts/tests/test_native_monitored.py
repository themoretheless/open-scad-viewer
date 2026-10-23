import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

RUNNER = Path(__file__).resolve().parents[1] / 'run-native-monitored.py'


class NativeMonitorTests(unittest.TestCase):
    def test_exit_status_is_not_replaced_by_observer_success(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'run'
            result = subprocess.run([sys.executable, str(RUNNER), '--output', str(output), '--', sys.executable, '-c', 'raise SystemExit(7)'], timeout=10)
            self.assertEqual(result.returncode, 7)
            events = [json.loads(line) for line in (output / 'events.jsonl').read_text().splitlines()]
            self.assertEqual(events[-1]['exitCode'], 7)

    def test_stop_is_recorded_and_same_child_is_awaited_after_continue(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'run'
            command = 'import os,signal;os.kill(os.getpid(),signal.SIGSTOP);raise SystemExit(3)'
            observer = subprocess.Popen([sys.executable, str(RUNNER), '--output', str(output), '--', sys.executable, '-c', command])
            child = None
            try:
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    path = output / 'events.jsonl'
                    try:
                        events = [json.loads(line) for line in path.read_text().splitlines()]
                    except (FileNotFoundError, ValueError):
                        events = []
                    if events:
                        child = events[0]['pid']
                    if any(e['kind'] == 'stopped' for e in events):
                        break
                    time.sleep(.02)
                else:
                    self.fail('No stopped status observed')
                self.assertIsNone(observer.poll())
                self.assertEqual(events[-1]['signal'], 'SIGSTOP')
                os.kill(child, signal.SIGCONT)
                self.assertEqual(observer.wait(timeout=10), 3)
                events = [json.loads(line) for line in path.read_text().splitlines()]
                self.assertEqual({e['pid'] for e in events}, {child})
                # The OS may coalesce CONTINUED with an immediate exit.
                # The terminal status of the same PID is the decisive check.
                self.assertEqual(events[-1]['exitCode'], 3)
            finally:
                if observer.poll() is None:
                    if child is not None:
                        os.killpg(child, signal.SIGKILL)
                    observer.wait(timeout=10)


if __name__ == '__main__':
    unittest.main()
