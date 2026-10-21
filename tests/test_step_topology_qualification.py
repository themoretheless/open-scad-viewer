"""Real STEP-oracle regressions; run with the qualified OCP Python environment.

Generate the owned_edge_partition_audit fixtures first. No binary fixture is
committed. Missing generated evidence/OCP skips these optional external checks.
"""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'output/qualification/trimmed-face-partition-2026-10-15'


@unittest.skipUnless(importlib.util.find_spec('OCP') and (FIXTURE / 'manifest.json').exists(),
                     'requires OCP and generated owned-edge STEP evidence')
class StepTopologyQualification(unittest.TestCase):
    def run_variant(self, alter):
        manifest = json.loads((FIXTURE / 'manifest.json').read_text())
        part = copy.deepcopy(next(p for p in manifest['parts'] if p['name'] == 'root-cut-trimmed-face'))
        manifest['parts'] = [part]
        alter(part['expected'])
        with tempfile.TemporaryDirectory(prefix='osv-step-topology-') as tmp:
            directory = Path(tmp)
            shutil.copyfile(FIXTURE / part['file'], directory / part['file'])
            (directory / 'manifest.json').write_text(json.dumps(manifest))
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/verify-cad-step-occt.py'),
                                     str(directory), '--no-preview'], capture_output=True, text=True)
            self.assertTrue((directory / 'occt-report.json').exists(), result.stdout + result.stderr)
            report = json.loads((directory / 'occt-report.json').read_text())
            return result.returncode, report['parts'][0]

    def test_wrong_vertex_or_edge_expectations_reject_an_otherwise_valid_solid(self):
        for key, wrong in [('vertices', 8), ('edges', 12)]:
            with self.subTest(key=key):
                code, part = self.run_variant(lambda expected: expected.update({key: wrong}))
                self.assertEqual(code, 1)
                self.assertTrue(part['valid'])
                self.assertEqual(part['solids'], 1)
                self.assertEqual(part['uniqueVertices'], 9)
                self.assertEqual(part['uniqueEdges'], 13)
                self.assertEqual(part['volumeErrorMm3'], 0)
                self.assertFalse(part['topologyPassed'])
                self.assertFalse(part['passed'])

    def test_legacy_manifest_does_not_claim_topology_expectations_were_checked(self):
        def remove(expected):
            expected.pop('vertices')
            expected.pop('edges')
        code, part = self.run_variant(remove)
        self.assertEqual(code, 0)
        self.assertTrue(part['passed'])
        self.assertIsNone(part['topologyPassed'])
        self.assertEqual(part['topologyExpectations'], {})


if __name__ == '__main__':
    unittest.main(verbosity=2)
