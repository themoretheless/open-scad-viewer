#!/usr/bin/env python3
"""Independent STEP transfer/basic topology check; never a solid/embedding proof."""
import argparse
import hashlib
import json
from pathlib import Path
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopExp import TopExp
from OCP.TopTools import TopTools_IndexedMapOfShape
from OCP.TopAbs import TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX, TopAbs_SOLID, TopAbs_SHELL
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps

def verify(path):
    reader = STEPControl_Reader()
    readable = reader.ReadFile(str(path)) == IFSelect_RetDone
    roots = reader.TransferRoots() if readable else 0
    if not readable or roots == 0:
        report = dict(occtVersion=OCP.__version__, inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            transferredRoots=roots, basicBrepValid=False,
            error='No transferred STEP roots' if readable else 'STEP read failed',
            globalEmbeddingQualified=False, selfIntersectionsQualified=False, materialQualified=False)
        path.with_suffix('.occt.json').write_text(json.dumps(report, indent=2)+'\n')
        print(json.dumps(report, indent=2))
        return 1
    shape = reader.OneShape()
    counts = {}
    for name, kind in [('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX),
                       ('solids', TopAbs_SOLID), ('shells', TopAbs_SHELL)]:
        entities = TopTools_IndexedMapOfShape()
        TopExp.MapShapes_s(shape, kind, entities)
        counts[name] = entities.Extent()
    properties = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, properties)
    valid = BRepCheck_Analyzer(shape, True).IsValid()
    report = dict(occtVersion=OCP.__version__, inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        transferredRoots=roots, counts=counts, basicBrepValid=valid,
        volumeEstimateMm3=properties.Mass(), globalEmbeddingQualified=False,
        selfIntersectionsQualified=False, materialQualified=False,
        scope='Independent STEP representation/basic topology and numerical volume only; not a global solid certificate')
    path.with_suffix('.occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if valid and counts == dict(faces=13, edges=33, vertices=22, solids=1, shells=1) and properties.Mass() > 0 else 1

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path)
    raise SystemExit(verify(parser.parse_args().path))
