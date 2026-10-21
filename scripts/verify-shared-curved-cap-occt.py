#!/usr/bin/env python3
"""Independent STEP check of z=-.2+x²/8+y²/50 over the unit square."""
import importlib.util
import json
import sys
from pathlib import Path

if __name__ == '__main__':
    directory=Path(sys.argv[1])
    manifest=json.loads((directory/'manifest.json').read_text())
    # Integral from z=-1 to the authored cap over 0<=x,y<=1.
    independent_volume=1.-.2+1./24.+1./150.
    assert abs(manifest['expected']['volumeMm3']-independent_volume)<1e-15
    assert {k:manifest['expected'][k] for k in ['solids','faces','edges','vertices']} == {
        'solids':1,'faces':6,'edges':14,'vertices':10}
    spec=importlib.util.spec_from_file_location('curved_cap_check',Path(__file__).with_name('verify-curved-cap-occt.py'))
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    sys.exit(module.main(directory))
