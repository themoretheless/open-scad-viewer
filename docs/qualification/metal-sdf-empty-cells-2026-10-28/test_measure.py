import ast
import importlib.util
from pathlib import Path
import unittest

SOURCE = Path(__file__).with_name('measure.py')
spec = importlib.util.spec_from_file_location('sdf_measure', SOURCE)
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


class QualificationSafety(unittest.TestCase):
    def test_no_process_discovery_can_send_stop_or_resume(self):
        tree = ast.parse(SOURCE.read_text())
        self.assertFalse(any(isinstance(n, ast.Attribute) and n.attr in {'kill', 'killpg', 'SIGSTOP', 'SIGCONT'}
                             for n in ast.walk(tree)))

    def test_own_descendants_are_excluded_but_sibling_jobs_remain_visible(self):
        def p(pid, parent, command):
            return dict(pid=pid, ppid=parent, comm=command)
        snap = {10: p(10, 1, '/app/codex'), 20: p(20, 10, '/bin/python'),
                21: p(21, 20, '/repo/target/release/build/sdf-core/hash/out/sdf_core-1234567890abcdef'),
                30: p(30, 10, '/tool/cargo'), 31: p(31, 30, '/tool/rustc'),
                32: p(32, 31, '/tool/worker')}
        self.assertEqual(measure.descendants(snap, 20), {20, 21})
        self.assertEqual([p['pid'] for p in measure.competing(snap, {20, 21})], [30, 31, 32])

    def test_native_benchmark_outside_target_is_observed(self):
        native = dict(pid=4, ppid=1, comm='/repo/artifacts/frozen/street_walk', explicit_work=True)
        self.assertEqual(measure.competing({4: native}, set()), [native])

    def test_control_gate_rejects_bias_wide_interval_or_missing_control(self):
        self.assertTrue(measure.stable([dict(kind='aa', ratio=1., lo=.99, hi=1.01)]))
        self.assertFalse(measure.stable([dict(kind='aa', ratio=.96, lo=.95, hi=.97)]))
        self.assertFalse(measure.stable([dict(kind='aa', ratio=1., lo=.94, hi=1.06)]))
        self.assertFalse(measure.stable([dict(kind='batch', ratio=1.1, lo=1.08, hi=1.12)]))


if __name__ == '__main__':
    unittest.main()

class MissingEvidenceSafety(unittest.TestCase):
    def test_missing_binary_fails_closed(self):
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(measure.matches_digest(Path(d)/'missing', 'irrelevant'))
