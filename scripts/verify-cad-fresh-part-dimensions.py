#!/usr/bin/env python3
"""Independent exact STEP face/line intersections at declared part gauges."""
import hashlib
import json
import math
import re
import importlib.metadata
from pathlib import Path
import sys


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
    if name != 'enclosure':
        raise ValueError('Unsupported acceptance part: '+str(name))
    return [('x walls and cavity', [-10,15,10], [1,0,0], [10,12,48,50]),
            ('y walls and cavity', [20,-10,10], [0,1,0], [10,12,38,40]),
            ('bottom thickness and open top', [20,15,-10], [0,0,1], [10,12]),
            ('wall height', [1,15,-10], [0,0,1], [10,30]),
            ('outer corner radius section', [-10,0.5,10], [1,0,0], [10+corner,50])]


PARTS = ('bracket', 'flange', 'enclosure')
STAGES = {'fresh-cad-parts-step/1': ('source', 'edited-rounded'),
          'fresh-cad-ui-step/1': ('ui-edited', 'ui-reloaded')}


def validate_manifest(manifest):
    """Require the full declared six-file matrix before loading an oracle."""
    if not isinstance(manifest, dict) or manifest.get('schema') not in STAGES:
        raise ValueError('Unsupported fresh-part STEP manifest schema')
    schema = manifest['schema']
    expected = {name+'-'+stage for name in PARTS for stage in STAGES[schema]}
    rows = manifest.get('parts')
    if not isinstance(rows, list) or len(rows) != len(expected):
        raise ValueError('Expected all three parts and both declared STEP states')
    if any(not isinstance(row, dict) or not isinstance(row.get('name'), str) for row in rows):
        raise ValueError('Every STEP row needs a declared name')
    if {row['name'] for row in rows} != expected:
        raise ValueError('Missing, duplicate or unexpected STEP state')
    files = set()
    specifications = {}
    for row in rows:
        file, digest = row.get('file'), row.get('sha256')
        if (not isinstance(file, str) or not file.endswith('.step') or
                Path(file).name != file or file in files):
            raise ValueError('Expected distinct local STEP filenames')
        files.add(file)
        if not isinstance(digest, str) or not re.fullmatch('[0-9a-f]{64}', digest):
            raise ValueError('Every STEP state needs its SHA-256')
        part = next(name for name in PARTS if row['name'].startswith(name+'-'))
        rounded = row['name'].endswith('edited-rounded') if schema == 'fresh-cad-parts-step/1' else True
        shift = [3,-2,1] if rounded else [0,0,0]
        spec = row.get('gaugeSpecification')
        if spec is None:
            if schema == 'fresh-cad-ui-step/1':
                raise ValueError('UI states need declared independent gauge placement')
        else:
            if (not isinstance(spec, dict) or set(spec) != {'part','rounded','translationMm'} or
                    spec['part'] != part or type(spec['rounded']) is not bool or spec['rounded'] != rounded):
                raise ValueError('Gauge specification disagrees with the declared STEP state')
            declared = spec['translationMm']
            if (not isinstance(declared, list) or len(declared) != 3 or
                    not all(type(x) in (int,float) and math.isfinite(x) for x in declared)):
                raise ValueError('Gauge placement must contain three finite numbers')
            if schema == 'fresh-cad-parts-step/1' and declared != shift:
                raise ValueError('Native state has an unexpected declared placement')
            shift = declared
        specifications[row['name']] = (part, rounded, shift)
    if schema == 'fresh-cad-ui-step/1':
        for part in PARTS:
            if specifications[part+'-ui-edited'] != specifications[part+'-ui-reloaded']:
                raise ValueError('Reload must preserve the declared measurement frame')
    return specifications


def main(directory):
    manifest_bytes = (directory/'manifest.json').read_bytes()
    manifest = json.loads(manifest_bytes)
    specifications = validate_manifest(manifest)
    from OCP.STEPControl import STEPControl_Reader
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.IntCurvesFace import IntCurvesFace_ShapeIntersector
    from OCP.gp import gp_Pnt, gp_Dir, gp_Lin
    parts = []
    for part in manifest['parts']:
        path = directory/part['file']
        if hashlib.sha256(path.read_bytes()).hexdigest() != part['sha256']:
            raise ValueError('STEP hash mismatch: '+part['name'])
        reader = STEPControl_Reader()
        if reader.ReadFile(str(path)) != IFSelect_RetDone or reader.TransferRoots() <= 0:
            raise RuntimeError('OpenCascade could not load '+part['name'])
        shape = reader.OneShape()
        part_name, rounded, shift = specifications[part['name']]
        rows = []
        for name, origin, direction, expected in gauges(part_name, rounded):
            origin = [v+d for v,d in zip(origin,shift)]
            query = IntCurvesFace_ShapeIntersector()
            query.Load(shape,1e-9)
            query.Perform(gp_Lin(gp_Pnt(*origin),gp_Dir(*direction)),-100,100)
            if not query.IsDone():
                raise RuntimeError('Independent gauge did not complete: '+name)
            actual = []
            for value in sorted(query.WParameter(i) for i in range(1,query.NbPnt()+1)):
                if not math.isfinite(value):
                    raise RuntimeError('Non-finite independent gauge result: '+name)
                if not actual or abs(value-actual[-1])>1e-7:
                    actual.append(value)
            error = max((abs(a-b) for a,b in zip(actual,expected)),default=math.inf)
            passed = len(actual)==len(expected) and error<=1e-6
            rows.append(dict(gauge=name,origin=origin,direction=direction,
                             expectedParametersMm=expected,actualParametersMm=actual,
                             maxErrorMm=error if math.isfinite(error) else None,passed=passed))
        if hashlib.sha256(path.read_bytes()).hexdigest() != part['sha256']:
            raise ValueError('STEP changed during measurement: '+part['name'])
        parts.append(dict(name=part['name'],stepSha256=part['sha256'],gauges=rows,passed=all(r['passed'] for r in rows)))
    if (directory/'manifest.json').read_bytes() != manifest_bytes:
        raise ValueError('Manifest changed during measurement')
    report = dict(schema='cad-fresh-independent-dimensions/2',oracle='OpenCascade STEP face/line intersection',
                  oracleVersion=importlib.metadata.version('cadquery-ocp'),
                  manifestSha256=hashlib.sha256(manifest_bytes).hexdigest(),
                  toolSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  passed=all(p['passed'] for p in parts),parts=parts,
                  scope='Declared cross sections only: web/wall/bottom thickness, cavity, bore, axial size and radius-1 sections. Not whole-wall or full seam certification.')
    with (directory/'dimension-report.json').open('x') as output:
        output.write(json.dumps(report,indent=2,allow_nan=False)+'\n')
    print(json.dumps(dict(passed=report['passed'],parts=len(parts),gauges=sum(len(p['gauges']) for p in parts))))
    return 0 if report['passed'] else 1


if __name__=='__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
