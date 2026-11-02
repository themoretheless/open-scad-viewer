"""Independent OCCT measurements of this declared polynomial control only."""
import argparse
import hashlib
import importlib.metadata
import json
from fractions import Fraction as F
from pathlib import Path

from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.IntCurvesFace import IntCurvesFace_ShapeIntersector
from OCP.gp import gp_Lin, gp_Pnt, gp_Dir


def h_integral(a, b):
    # Direct physical-coordinate polynomial; no native Bernstein code is used.
    return F(280, 1 << 18) * sum(
        F(c, k + 1) * (b ** (k + 1) - a ** (k + 1))
        for k, c in [(4, 16), (5, -32), (6, 24), (7, -8), (8, 1)]
    )


def main():
    args = argparse.ArgumentParser()
    args.add_argument('source', type=Path)
    args.add_argument('edited', type=Path)
    args.add_argument('--report', type=Path, required=True)
    options = args.parse_args()
    source_volume = F(32) + F(49, 24) - h_integral(F(-1), F(3))
    volume_drop = h_integral(F(0), F(2)) * F(1, 16) / 5
    height_drop = F(280, 1 << 18) * F(3, 32)
    expected = [(source_volume, F(7, 64), 7),
                (source_volume - volume_drop, F(7, 64) - height_drop, 8)]
    rows = []
    for path, (volume, height, faces) in zip([options.source, options.edited], expected):
        before = hashlib.sha256(path.read_bytes()).hexdigest()
        reader = STEPControl_Reader()
        if reader.ReadFile(str(path)) != IFSelect_RetDone or reader.TransferRoots() < 1:
            raise RuntimeError(f'Cannot read STEP: {path}')
        shape = reader.OneShape()

        def count(kind):
            explorer, total = TopExp_Explorer(shape, kind), 0
            while explorer.More():
                total += 1
                explorer.Next()
            return total

        props = GProp_GProps()
        BRepGProp.VolumeProperties_s(shape, props, 1e-12, True)
        ray = IntCurvesFace_ShapeIntersector()
        ray.Load(shape, 1e-9)
        ray.Perform(gp_Lin(gp_Pnt(1., 0., 0.), gp_Dir(0., 0., 1.)), -10., 10.)
        if ray.NbPnt() < 1:
            raise RuntimeError(f'Missing independent height intersection: {path}')
        measured_height = max(ray.Pnt(i).Z() for i in range(1, ray.NbPnt() + 1))
        row = dict(file=path.name, sha256=before,
                   valid=BRepCheck_Analyzer(shape).IsValid(), solids=count(TopAbs_SOLID),
                   faces=count(TopAbs_FACE), volumeMm3=props.Mass(),
                   expectedVolumeMm3=float(volume), volumeErrorMm3=abs(props.Mass()-float(volume)),
                   heightMm=measured_height, expectedHeightMm=float(height),
                   heightErrorMm=abs(measured_height-float(height)))
        if (not row['valid'] or row['solids'] != 1 or row['faces'] != faces
                or row['volumeErrorMm3'] >= 1e-7 or row['heightErrorMm'] >= 1e-8):
            raise RuntimeError(f'Independent STEP measurement failed: {row}')
        if hashlib.sha256(path.read_bytes()).hexdigest() != before:
            raise RuntimeError(f'STEP changed during independent measurement: {path}')
        rows.append(row)
    actual_drop = rows[0]['volumeMm3'] - rows[1]['volumeMm3']
    if abs(actual_drop-float(volume_drop)) >= 1e-7:
        raise RuntimeError('Independent changed volume was not preserved')
    report = dict(schema='compatible-nurbs-contact-independent-step/2', passed=True,
                  oracle='OpenCascade via cadquery-ocp', wrapperVersion=importlib.metadata.version('cadquery-ocp'),
                  verifierSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  parts=rows, expectedVolumeDropMm3=float(volume_drop), actualVolumeDropMm3=actual_drop,
                  expectedHeightDropMm=float(height_drop),
                  actualHeightDropMm=rows[0]['heightMm']-rows[1]['heightMm'],
                  scope='Declared polynomial control: STEP validity, solid/face count, volume and ray height. Not radius-law or independent global-intersection qualification.')
    options.report.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
