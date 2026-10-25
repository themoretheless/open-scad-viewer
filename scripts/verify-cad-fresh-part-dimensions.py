#!/usr/bin/env python3
"""Independent exact STEP face/line intersections at declared part gauges."""
import hashlib
import json
import math
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.IntCurvesFace import IntCurvesFace_ShapeIntersector
from OCP.gp import gp_Pnt, gp_Dir, gp_Lin


def gauges(name, rounded):
    corner = 1-math.sqrt(0.75) if rounded else 0
    if name == 'bracket':
        return [('horizontal web', [20,-10,10], [0,1,0], [10,15]),
                ('upright web', [-10,20,10], [1,0,0], [10,15]),
                ('corner radius section', [-10,0.5,10], [1,0,0], [10+corner,50])]
    if name == 'flange':
        rim = 19+math.sqrt(0.75) if rounded else 20
        return [('bore and radial walls', [-30,0,3], [1,0,0], [10,25,35,50]),
                ('bore off-axis section', [-30,3,3], [1,0,0], [30-math.sqrt(391),26,34,30+math.sqrt(391)]),
                ('axial thickness', [10,0,-10], [0,0,1], [10,16]),
                ('outer rim radius section', [-30,0,5.5], [1,0,0], [30-rim,25,35,30+rim])]
    assert name == 'enclosure'
    return [('x walls and cavity', [-10,15,10], [1,0,0], [10,12,48,50]),
            ('y walls and cavity', [20,-10,10], [0,1,0], [10,12,38,40]),
            ('bottom thickness and open top', [20,15,-10], [0,0,1], [10,12]),
            ('wall height', [1,15,-10], [0,0,1], [10,30]),
            ('outer corner radius section', [-10,0.5,10], [1,0,0], [10+corner,50])]


def main(directory):
    manifest = json.loads((directory/'manifest.json').read_text())
    parts = []
    for part in manifest['parts']:
        path = directory/part['file']
        assert hashlib.sha256(path.read_bytes()).hexdigest() == part['sha256']
        reader = STEPControl_Reader()
        assert reader.ReadFile(str(path)) == IFSelect_RetDone and reader.TransferRoots()>0
        shape = reader.OneShape()
        specification = part.get('gaugeSpecification')
        if specification is None:
            rounded = part['name'].endswith('edited-rounded')
            shift = [3,-2,1] if rounded else [0,0,0]
            part_name = part['name'].split('-')[0]
        else:
            assert set(specification) == {'part','rounded','translationMm'}
            part_name, rounded, shift = specification['part'], specification['rounded'], specification['translationMm']
            assert part_name in ('bracket','flange','enclosure') and type(rounded) is bool
            assert len(shift)==3 and all(type(v) in (int,float) and math.isfinite(v) for v in shift)
        rows = []
        for name, origin, direction, expected in gauges(part_name, rounded):
            origin = [v+d for v,d in zip(origin,shift)]
            query = IntCurvesFace_ShapeIntersector()
            query.Load(shape,1e-9)
            query.Perform(gp_Lin(gp_Pnt(*origin),gp_Dir(*direction)),-100,100)
            assert query.IsDone()
            actual = []
            for value in sorted(query.WParameter(i) for i in range(1,query.NbPnt()+1)):
                assert math.isfinite(value)
                if not actual or abs(value-actual[-1])>1e-7:
                    actual.append(value)
            error = max((abs(a-b) for a,b in zip(actual,expected)),default=math.inf)
            passed = len(actual)==len(expected) and error<=1e-6
            rows.append(dict(gauge=name,origin=origin,direction=direction,
                             expectedParametersMm=expected,actualParametersMm=actual,
                             maxErrorMm=error if math.isfinite(error) else None,passed=passed))
        parts.append(dict(name=part['name'],stepSha256=part['sha256'],gauges=rows,passed=all(r['passed'] for r in rows)))
    report = dict(schema='cad-fresh-independent-dimensions/1',oracle='OpenCascade STEP face/line intersection',
                  passed=all(p['passed'] for p in parts),parts=parts,
                  scope='Declared cross sections only: web/wall/bottom thickness, cavity, bore, axial size and radius-1 sections. Not whole-wall or full seam certification.')
    with (directory/'dimension-report.json').open('x') as output:
        output.write(json.dumps(report,indent=2,allow_nan=False)+'\n')
    print(json.dumps(dict(passed=report['passed'],parts=len(parts),gauges=sum(len(p['gauges']) for p in parts))))
    return 0 if report['passed'] else 1


if __name__=='__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
