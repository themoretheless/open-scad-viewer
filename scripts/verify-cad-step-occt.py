#!/usr/bin/env python3
"""Independent STEP geometry acceptance and preview using OpenCascade, not our kernel.

Run with Python 3.12 and cadquery-ocp==8.0.1.0.0. See the companion fixture exporter.
Use --no-preview for the same numerical checks without system font discovery.
"""
import argparse
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import sys

from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.BRep import BRep_Tool
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.TopLoc import TopLoc_Location
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common
from OCP.gp import gp_Pnt


def count_shapes(shape, kind):
    explorer, count = TopExp_Explorer(shape, kind), 0
    while explorer.More():
        count += 1
        explorer.Next()
    return count


def count_unique_shapes(shape, kind):
    # IsSame compares native topology plus location and ignores face-use
    # orientation. Explorer counts alone duplicate the two uses of each edge.
    # This also works with OCP versions that do not bind IndexedMapOfShape.
    explorer, unique = TopExp_Explorer(shape, kind), []
    while explorer.More():
        current = explorer.Current()
        if not any(current.IsSame(other) for other in unique):
            unique.append(current)
        explorer.Next()
    return len(unique)


def triangles(shape):
    mesher = BRepMesh_IncrementalMesh(shape, 0.05, False, 0.2, True)
    if not mesher.IsDone():
        raise RuntimeError('OpenCascade tessellation failed')
    result = []
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while explorer.More():
        face, location = TopoDS.Face(explorer.Current()), TopLoc_Location()
        mesh = BRep_Tool.Triangulation_s(face, location)
        if mesh is None:
            raise RuntimeError('Missing face triangulation')
        for i in range(1, mesh.NbTriangles() + 1):
            points = []
            for index in [mesh.Triangle(i).Value(j) for j in range(1, 4)]:
                p = mesh.Node(index).Transformed(location.Transformation())
                points.append([p.X(), p.Y(), p.Z()])
            result.append(points)
        explorer.Next()
    return result


def main(directory, make_preview=True):
    manifest = json.loads((directory / 'manifest.json').read_text())
    if make_preview:
        os.environ.setdefault('MPLCONFIGDIR', str(directory / '.matplotlib'))
        import matplotlib
        matplotlib.use('Agg')
        from matplotlib import pyplot as plt
        from mpl_toolkits.mplot3d.art3d import Poly3DCollection
        plot_rows = max(1, (len(manifest['parts']) + 1) // 2)
        fig = plt.figure(figsize=(12, 4.5 * plot_rows), layout='constrained')
    rows = []
    for index, part in enumerate(manifest['parts']):
        path = directory / part['file']
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != part['sha256']:
            raise ValueError('Fixture differs from manifest: ' + part['name'])
        reader = STEPControl_Reader()
        if reader.ReadFile(str(path)) != IFSelect_RetDone or reader.TransferRoots() == 0:
            raise ValueError('OpenCascade could not import ' + part['name'])
        shape = reader.OneShape()
        valid = not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
        box = Bnd_Box()
        BRepBndLib.AddOptimal_s(shape, box, False, False)
        bounds = [[point.X(), point.Y(), point.Z()] for point in [box.CornerMin(), box.CornerMax()]]
        properties = GProp_GProps()
        integration_error = BRepGProp.VolumeProperties_s(shape, properties, Eps=1e-10)
        volume = properties.Mass()
        error = max(abs(a - b) for actual, expected in zip(bounds, part['expected']['boundsMm']) for a, b in zip(actual, expected))
        volume_error = abs(volume - part['expected']['volumeMm3'])
        solids = count_shapes(shape, TopAbs_SOLID)
        passed = valid and solids == part['expected'].get('solids', 1) and error <= manifest['toleranceMm'] and volume_error <= max(1e-6, abs(part['expected']['volumeMm3']) * manifest['relativeVolumeTolerance'])
        edges = count_unique_shapes(shape, TopAbs_EDGE)
        vertices = count_unique_shapes(shape, TopAbs_VERTEX)
        topology_expected = {key: part['expected'][key] for key in ('edges', 'vertices')
                             if key in part['expected']}
        topology_passed = (all(actual == topology_expected[key]
                               for key, actual in [('edges', edges), ('vertices', vertices)]
                               if key in topology_expected) if topology_expected else None)
        passed = passed and topology_passed is not False
        faces = triangles(shape)
        row = dict(name=part['name'], stepSha256=digest, valid=valid, solids=solids,
                   faces=count_shapes(shape, TopAbs_FACE), uniqueEdges=edges, uniqueVertices=vertices,
                   topologyPassed=topology_passed, topologyExpectations=topology_expected, boundsMm=bounds, volumeMm3=volume,
                   maxBoundsErrorMm=error, volumeErrorMm3=volume_error,
                   volumeIntegrationRelativeError=integration_error, passed=passed,
                   previewTriangles=len(faces))
        rows.append(row)
        if 'lowerVolumeMm3' in part['expected']:
            lower = BRepPrimAPI_MakeBox(gp_Pnt(bounds[0][0]-1, bounds[0][1]-1, bounds[0][2]-1),
                                       gp_Pnt(bounds[1][0]+1, bounds[1][1]+1, part['expected']['sectionZ'])).Shape()
            cut = BRepAlgoAPI_Common(shape, lower)
            assert cut.IsDone(), 'Independent section failed'
            lower_properties = GProp_GProps()
            BRepGProp.VolumeProperties_s(cut.Shape(), lower_properties)
            row['lowerVolumeMm3'] = lower_properties.Mass()
            row['lowerVolumeErrorMm3'] = abs(row['lowerVolumeMm3']-part['expected']['lowerVolumeMm3'])
            row['passed'] = passed = passed and row['lowerVolumeErrorMm3'] <= max(1e-6, abs(part['expected']['lowerVolumeMm3'])*manifest['relativeVolumeTolerance'])
        if make_preview:
            ax = fig.add_subplot(plot_rows, 2, index + 1, projection='3d')
            ax.add_collection3d(Poly3DCollection(faces, facecolor='#91b6d8', edgecolor='#45647f', linewidth=0.08, alpha=1))
            for axis, lo, hi in zip('xyz', bounds[0], bounds[1]):
                getattr(ax, 'set_' + axis + 'lim')(lo, hi)
                getattr(ax, 'set_' + axis + 'ticks')([lo, (lo+hi)/2, hi])
                getattr(ax, 'set_' + axis + 'label')(axis.upper() + ' / mm')
            ax.set_box_aspect([max(hi-lo, 1e-6) for lo, hi in zip(*bounds)])
            ax.view_init(elev=27, azim=-55)
            ax.set_title(f"{part['name']} · {'PASS' if passed else 'FAIL'}\nVolume {volume:.6f} mm³")
    if make_preview:
        fig.suptitle('Independent OpenCascade STEP import — current edited geometry')
        fig.savefig(directory / 'occt-preview.png', dpi=150)
        plt.close(fig)
    report = dict(schema='cad-roadmap-occt/1', oracle='OpenCascade via cadquery-ocp',
                  version=importlib.metadata.version('cadquery-ocp'),
                  scope='Independent STEP read, solid validity, optional unique topology counts, exact bounding box, volume and OCCT tessellation. Does not certify all STEP inputs.',
                  previewProduced=make_preview,
                  passed=all(row['passed'] for row in rows), parts=rows)
    (directory / 'occt-report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', nargs='?', default='output/qualification/cad-roadmap-step')
    parser.add_argument('--no-preview', action='store_true', help='Measure and tessellate without generating the PNG preview')
    args = parser.parse_args()
    sys.exit(main(Path(args.directory).resolve(), make_preview=not args.no_preview))
