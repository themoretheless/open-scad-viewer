#!/usr/bin/env python3
"""Independent import of the 0.1 mm setback control; no radius/G2 claim."""
import hashlib
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool


def entities(shape, kind):
    found = []
    explorer = TopExp_Explorer(shape, kind)
    while explorer.More():
        entity = explorer.Current()
        if not any(entity.IsSame(old) for old in found):
            found.append(entity)
        explorer.Next()
    return found


def main(directory):
    step = directory / 'runout.step'
    reader = STEPControl_Reader()
    if reader.ReadFile(str(step)) != IFSelect_RetDone or not reader.TransferRoots():
        raise ValueError('STEP import failed')
    shape = reader.OneShape()
    valid = not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: len(entities(shape, kind)) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE),
        ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    bounds = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
    wanted = [[2., 0., 0.], [8., 10., 10.]]
    bounds_error = max(abs(a-b) for row, goal in zip(bounds, wanted) for a, b in zip(row, goal))
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props)
    points = [BRep_Tool.Pnt_s(TopoDS.Vertex(v)) for v in entities(shape, TopAbs_VERTEX)]
    setbacks = []
    for x in [2., 8.]:
        floor = [p for p in points if abs(p.X()-x)<1e-6 and abs(p.Z())<1e-6 and p.Y()>9.]
        cap = [p for p in points if abs(p.X()-x)<1e-6 and abs(p.Y()-10.)<1e-6 and p.Z()<1.]
        if len(floor)!=1 or len(cap)!=1:
            raise ValueError('Contact vertices are ambiguous')
        setbacks.append(dict(xMm=x, floorSetbackMm=10.-floor[0].Y(), capSetbackMm=cap[0].Z()))
    error = max(abs(value-0.1) for row in setbacks for key, value in row.items() if key!='xMm')
    passed = (valid and topology==dict(solids=1, faces=7, edges=15, vertices=10)
              and bounds_error<=1e-6 and error<=1e-6 and 0<props.Mass()<600.)
    report = dict(stepSha256=hashlib.sha256(step.read_bytes()).hexdigest(), valid=valid,
                  topology=topology, boundsMm=bounds, maximumBoundsErrorMm=bounds_error,
                  setbacks=setbacks, maximumSetbackErrorMm=error, volumeMm3=props.Mass(),
                  prescribedRadiusClaim=False, wholeBodySmoothnessClaim=False, passed=passed)
    (directory/'occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if passed else 1


if __name__=='__main__':
    sys.exit(main(Path(sys.argv[1])))
