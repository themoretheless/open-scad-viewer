"""Coverage gates must refuse before importing OpenCascade or measuring files."""
import importlib.util
from pathlib import Path
import unittest
import subprocess
import sys
import tempfile
import json

spec = importlib.util.spec_from_file_location('fresh_dimensions', Path(__file__).resolve().parents[1]/'scripts/verify-cad-fresh-part-dimensions.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def manifest(ui=False):
    schema = 'fresh-cad-ui-step/1' if ui else 'fresh-cad-parts-step/1'
    stages = ('ui-edited', 'ui-reloaded') if ui else ('source', 'edited-rounded')
    parts = []
    for part in ('bracket', 'flange', 'enclosure'):
        for stage in stages:
            name = part+'-'+stage
            row = dict(name=name, file=name+'.step', sha256='a'*64)
            if ui:
                row['gaugeSpecification'] = dict(part=part, rounded=True, translationMm=[1,0,0])
            parts.append(row)
    return dict(schema=schema, parts=parts)


class Coverage(unittest.TestCase):
    def test_empty_cli_matrix_cannot_emit_a_success_report_with_or_without_optimization(self):
        for optimize in (False, True):
            with tempfile.TemporaryDirectory() as root:
                folder = Path(root)
                (folder/'manifest.json').write_text(json.dumps(dict(schema='fresh-cad-parts-step/1',parts=[])))
                command = [sys.executable]+(['-O'] if optimize else [])+[str(module.__file__),str(folder)]
                result = subprocess.run(command,text=True,capture_output=True,check=False)
                self.assertNotEqual(result.returncode,0)
                self.assertIn('Expected all three parts and both declared STEP states',result.stderr)
                self.assertFalse((folder/'dimension-report.json').exists())

    def test_complete_native_and_ui_matrices(self):
        for ui in (False, True):
            self.assertEqual(len(module.validate_manifest(manifest(ui))), 6)

    def test_empty_missing_and_duplicate_states_refuse(self):
        for rows in ([], manifest()['parts'][:-1], manifest()['parts'][:-1]+[manifest()['parts'][0]]):
            bad = manifest()
            bad['parts'] = rows
            with self.assertRaises(ValueError):
                module.validate_manifest(bad)

    def test_state_cannot_override_rounding_part_or_native_placement(self):
        for change in (dict(part='flange', rounded=True, translationMm=[3,-2,1]),
                       dict(part='bracket', rounded=False, translationMm=[3,-2,1]),
                       dict(part='bracket', rounded=True, translationMm=[0,0,0])):
            bad = manifest()
            bad['parts'][1]['gaugeSpecification'] = change
            with self.assertRaises(ValueError):
                module.validate_manifest(bad)

    def test_ui_reload_requires_the_same_declared_frame(self):
        for replacement in (None, dict(part='bracket', rounded=True, translationMm=[2,0,0]),
                            dict(part='bracket', rounded=True, translationMm=[float('nan'),0,0])):
            bad = manifest(True)
            bad['parts'][1]['gaugeSpecification'] = replacement
            with self.assertRaises(ValueError):
                module.validate_manifest(bad)

    def test_distinct_local_files_valid_hashes_and_known_schema_are_required(self):
        for key, value in (('file','../external.step'), ('file','flange-source.step'),
                           ('sha256',''), ('name','bracket-unknown')):
            bad = manifest()
            bad['parts'][0][key] = value
            with self.assertRaises(ValueError):
                module.validate_manifest(bad)
        bad = manifest()
        bad['schema'] = 'other/1'
        with self.assertRaises(ValueError):
            module.validate_manifest(bad)


if __name__ == '__main__':
    unittest.main()
