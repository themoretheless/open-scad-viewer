"""Check the edited STEP round-trip corpus with OpenCascade.

Reads every <case>.step written by
`cargo test -p cad-step --test exchange_round_trip_corpus` with
STEP_CORPUS_OUTPUT=<dir> and compares OpenCascade's validity, solid count,
volume, area and centroid with the kernel's <case>.expect.json sidecar.
"""
import json
import sys
from pathlib import Path

from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

root = Path(sys.argv[1])
reports, failures = [], []
for expect_path in sorted(root.glob('*.expect.json')):
    name = expect_path.name.removesuffix('.expect.json')
    expect = json.loads(expect_path.read_text())
    reader = STEPControl_Reader()
    if reader.ReadFile(str(root / f'{name}.step')) != IFSelect_RetDone or reader.TransferRoots() < 1:
        failures.append(f'{name}: OpenCascade did not read the file')
        continue
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    solids = 0
    explorer = TopExp_Explorer(shape, TopAbs_SOLID)
    while explorer.More():
        solids += 1
        explorer.Next()
    volume_props, area_props = GProp_GProps(), GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, volume_props)
    BRepGProp.SurfaceProperties_s(shape, area_props)
    centre = volume_props.CentreOfMass().Coord()
    rel = lambda a, e: abs(a - e) / max(abs(e), 1.0)
    report = dict(
        case=name,
        valid=valid,
        solids=solids,
        volume_rel_error=rel(volume_props.Mass(), expect['volume']),
        area_rel_error=rel(area_props.Mass(), expect['area']),
        centroid_error=max(abs(a - e) for a, e in zip(centre, expect['centroid'])),
    )
    reports.append(report)
    if not valid or solids != expect['solids'] or report['volume_rel_error'] > 1e-4 \
            or report['area_rel_error'] > 1e-4 or report['centroid_error'] > 1e-4 * (1 + max(map(abs, expect['centroid']))):
        failures.append(f'{name}: {report}')

result = dict(parser='OpenCascade/cadquery-ocp', cases=reports, failures=failures,
              scope='edited STEP round-trip corpus; volume/area/centroid at 1e-4 relative')
(root / 'opencascade-roundtrip-corpus.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
assert len(reports) >= 40, f'corpus shrank to {len(reports)} cases'
assert not failures, '\n'.join(failures)
