#!/usr/bin/env python3
"""Independent gauges for the declared 20-edit bracket, flange and enclosure.
Expected dimensions follow the fixture specification and six edits, not our B-rep.
"""
import hashlib
import json
import math
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.IntCurvesFace import IntCurvesFace_ShapeIntersector
from OCP.gp import gp_Pnt, gp_Dir, gp_Lin


def gauges(name):
    corner = 8 - math.sqrt(.75)
    if name == 'bracket':
        return [
            ('horizontal web 5 mm', [27, -10, 13], [0, 1, 0], [10, 15]),
            ('upright web 5 mm', [-10, 20, 13], [1, 0, 0], [17, 22]),
            ('corner radius 1 mm', [-10, .5, 13], [1, 0, 0], [10 + corner, 63]),
        ]
    if name == 'flange':
        return [
            ('bore radius 6.5 mm and outside radius 20 mm', [7, 0, 6], [1, 0, 0], [6.5, 20]),
            ('outer fillet radius 1 mm', [7, 0, 11.5], [1, 0, 0], [6.5, 19 + math.sqrt(.75)]),
        ]
    if name == 'enclosure':
        return [
            ('opposing walls 1.4 mm', [-10, 15, 13], [1, 0, 0], [17, 18.4, 55.6, 57]),
            ('bottom thickness 2 mm', [27, 15, -10], [0, 0, 1], [10, 12]),
            ('corner radius 1 mm', [-10, .5, 13], [1, 0, 0], [10 + corner, 57]),
        ]
    raise ValueError('Unsupported acceptance specimen: ' + name)


def main(directory, service=False):
    manifest = json.loads((directory / 'manifest.json').read_text())
    marker = '-mixed20-' if service else '-ui20-'
    expected_names = {name + marker + str(cycle) for name in ['bracket', 'flange', 'enclosure'] for cycle in range(2)}
    if len(manifest['parts']) != 6 or {part['name'] for part in manifest['parts']} != expected_names:
        raise ValueError('Expected all three declared specimens and both STEP cycles')
    results = []
    for part in manifest['parts']:
        if marker not in part['name']:
            raise ValueError('Expected the actual UI 20-edit exports')
        file = directory / part['file']
        digest = hashlib.sha256(file.read_bytes()).hexdigest()
        if digest != part['sha256']:
            raise ValueError('STEP hash differs from the accepted browser export')
        reader = STEPControl_Reader()
        if reader.ReadFile(str(file)) != IFSelect_RetDone or reader.TransferRoots() == 0:
            raise ValueError('Independent STEP import failed')
        shape = reader.OneShape()
        rows = []
        for label, origin, direction, expected in gauges(part['name'].split(marker)[0]):
            query = IntCurvesFace_ShapeIntersector()
            query.Load(shape, 1e-8)
            query.Perform(gp_Lin(gp_Pnt(*origin), gp_Dir(*direction)), 0, 100)
            if not query.IsDone():
                raise ValueError('Independent ray intersection did not finish')
            raw = sorted(query.WParameter(i) for i in range(1, query.NbPnt() + 1))
            actual = []
            for value in raw:
                if not actual or abs(value - actual[-1]) > 1e-7:
                    actual.append(value)
            error = max((abs(a-b) for a, b in zip(actual, expected)), default=float('inf'))
            passed = len(actual) == len(expected) and error <= 1e-6
            rows.append(dict(gauge=label, origin=origin, direction=direction,
                             expectedParametersMm=expected, actualParametersMm=actual,
                             maxErrorMm=error, passed=passed))
        results.append(dict(name=part['name'], stepSha256=digest, gauges=rows,
                            passed=all(row['passed'] for row in rows)))
    report = dict(schema='cad-mixed-service-independent-dimensions/1' if service else 'cad-mixed-ui-independent-dimensions/1',
                  source='20-edit service exports' if service else '20-edit browser exports',
                  oracle='OpenCascade original STEP faces and exact line intersections',
                  scope='Specified walls, bore and radius-1 corner/fillet sections of the declared 20-edit specimens; not whole-body thickness coverage',
                  passed=all(row['passed'] for row in results), parts=results)
    (directory / 'dimension-report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(passed=report['passed'], parts=len(results),
                         gauges=sum(len(row['gauges']) for row in results))))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve(), '--service' in sys.argv[2:]))
