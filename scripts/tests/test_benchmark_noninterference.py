import ast
from pathlib import Path
import re
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
RUNNERS = [
    'metal-small-sdf-isolated-2026-10-20/qualify.py',
    'metal-small-sdf-clean-windows-2026-10-20/qualify.py',
    'metal-small-sdf-quiescent-2026-10-20/qualify.py',
    'metal-sdf-large-auto-2026-10-21/qualify-template.py',
    'metal-sdf-large-auto-drained-2026-10-21/qualify-template.py',
]


class BenchmarkNoninterference(unittest.TestCase):
    def test_competing_jobs_are_observed_without_signals_or_journal_changes(self):
        for name in RUNNERS:
            with self.subTest(runner=name):
                tree = ast.parse((ROOT / 'docs/qualification' / name).read_text())
                self.assertFalse(any(isinstance(n, ast.Attribute) and n.attr == 'SIGSTOP' for n in ast.walk(tree)))
                functions = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in ('competitor', 'observe_competitors')]
                self.assertEqual(len(functions), 2)
                processes = {
                    10: dict(pid=10, ppid=1, state='R', comm='/tool/cargo'),
                    11: dict(pid=11, ppid=10, state='R', comm='/tool/rustc'),
                    12: dict(pid=12, ppid=11, state='S', comm='/tool/worker'),
                    20: dict(pid=20, ppid=1, state='T', comm='/tool/rustc'),
                    30: dict(pid=30, ppid=1, state='R', comm='/tool/cargo'),
                    31: dict(pid=31, ppid=30, state='R', comm='/tool/rustc'),
                }
                class NoSignals:
                    @staticmethod
                    def getpid():
                        return 30
                    def kill(self, *args):
                        raise AssertionError('Benchmark must not signal competing work')
                journal = {'paused': []}
                namespace = dict(snapshot=lambda: processes, allowed={1,30}, deadline=time.monotonic()+100,
                                 time=time, pathlib=__import__('pathlib'), re=re, events=[], os=NoSignals(),
                                 data=journal, save=lambda *args: self.fail('Unexpected journal mutation'))
                exec(compile(ast.Module(body=functions, type_ignores=[]), name, 'exec'), namespace)
                self.assertEqual(namespace['observe_competitors'](), [10,11,12])
                self.assertEqual(journal, {'paused': []})
                self.assertEqual(namespace['events'][0]['observed_competitors'], [10,11,12])
                self.assertEqual(namespace['events'][0]['paused'], [])


if __name__ == '__main__':
    unittest.main()
