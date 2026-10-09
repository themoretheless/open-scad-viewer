"""Independent OCCT import/topology/volume checks for exported sweep fixtures.
Does not certify continuous sweep error, general embedding or seam smoothness.
"""
from fractions import Fraction
import hashlib
import json
import math
import sys
from pathlib import Path
from step_rational_basis import same_positive_projective_weights
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.gp import gp_Pnt, gp_Vec, gp_Pnt2d, gp_Vec2d
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface, BRepAdaptor_Curve, BRepAdaptor_Curve2d
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_SHELL, TopAbs_WIRE, TopAbs_EDGE, TopAbs_FORWARD, TopAbs_REVERSED
from OCP.TopExp import TopExp_Explorer, TopExp
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools, BRepTools_WireExplorer
from OCP.collections import IndexedDataMap_TopoDS_Shape_List_TopoDS_Shape_TopTools_ShapeMapHasher
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.GeomAbs import GeomAbs_Line, GeomAbs_BSplineCurve, GeomAbs_BSplineSurface
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.TopAbs import TopAbs_IN, TopAbs_OUT

def full_domain_cap_coedge(surface, pcurve, curve, tolerance):
    """Exact rational upper enclosure of S(pcurve(t))-curve(t).

    Normalize the cap's affine knot rectangle: S=a+b*u+c*v+d*u*v.
    Shared positive rational curve bases enclose the affine residual by its
    control residuals. Convex UV hulls bound the remaining bilinear term.
    No sampled values are premises of this whole-parameter certificate.
    """
    if (surface.GetType() != GeomAbs_BSplineSurface or
        pcurve.GetType() != GeomAbs_BSplineCurve or curve.GetType() != GeomAbs_BSplineCurve):
        return False
    s, uv, xyz = surface.BSpline(), pcurve.BSpline(), curve.BSpline()
    if (s.UDegree() != 1 or s.VDegree() != 1 or s.NbUPoles() != 2 or s.NbVPoles() != 2
        or s.IsUPeriodic() or s.IsVPeriodic()):
        return False
    domains = []
    for axis in ['U', 'V']:
        knot = getattr(s, axis+'Knot')
        multiplicity = getattr(s, axis+'Multiplicity')
        if getattr(s, 'Nb'+axis+'Knots')() != 2 or not knot(1) < knot(2) or [multiplicity(1),multiplicity(2)] != [2,2]:
            return False
        domains.append([Fraction(knot(1)),Fraction(knot(2))])
    weights = [s.Weight(i,j) for i in [1,2] for j in [1,2]]
    if not (weights[0] > 0 and all(w == weights[0] for w in weights)):
        return False
    if (uv.Degree() != xyz.Degree() or uv.NbPoles() != xyz.NbPoles()
        or uv.IsPeriodic() or xyz.IsPeriodic() or uv.NbKnots() != xyz.NbKnots()):
        return False
    if any(uv.Knot(i) != xyz.Knot(i) or uv.Multiplicity(i) != xyz.Multiplicity(i)
           for i in range(1,uv.NbKnots()+1)):
        return False
    if any(uv.Weight(i) <= 0 or uv.Weight(i) != xyz.Weight(i) for i in range(1,uv.NbPoles()+1)):
        return False
    pole = lambda i,j: [Fraction(x) for x in s.Pole(i,j).Coord()]
    a, q, r, z = pole(1,1), pole(2,1), pole(1,2), pole(2,2)
    b, c = [q[k]-a[k] for k in range(3)], [r[k]-a[k] for k in range(3)]
    d = [z[k]-q[k]-r[k]+a[k] for k in range(3)]
    uv_poles = [[(Fraction(x)-domains[k][0])/(domains[k][1]-domains[k][0])
                 for k,x in enumerate(uv.Pole(i).Coord())] for i in range(1,uv.NbPoles()+1)]
    if any(x < 0 or x > 1 for p in uv_poles for x in p):
        return False
    max_u, max_v = (max(abs(p[k]) for p in uv_poles) for k in [0,1])
    affine_error = [max(abs(a[k]+b[k]*p[0]+c[k]*p[1]-Fraction(xyz.Pole(i+1).Coord()[k]))
                        for i,p in enumerate(uv_poles)) for k in range(3)]
    upper = [affine_error[k]+abs(d[k])*max_u*max_v for k in range(3)]
    return sum(x*x for x in upper) <= Fraction(tolerance)**2

def exact_cap_planarity(case):
    """Audit authored binary64 edge poles, independently of OCCT tolerances.

    A nonzero exact determinant proves no planar surface contains these poles.
    This diagnostic never promotes a sampled or tolerance certificate.
    """
    reports = []
    caps = case.get('capFaces', list(range(case['faces'] - 2, case['faces'])) if case.get('capHoleFaces') else [])
    for face_id in caps:
        face = case['faceLoops'][face_id]
        edges = face['outer'] + [edge for wire in face['holes'] for edge in wire]
        poles = list(dict.fromkeys(tuple(Fraction(v) for v in pole)
                     for edge in edges for pole in case['edgeCurves'][edge]['curve']['controlPoints']))
        origin = poles[0]
        delta = lambda p: tuple(p[k] - origin[k] for k in range(3))
        direction = next((delta(p) for p in poles[1:] if p != origin), None)
        normal = None
        witness = None
        if direction is not None:
            for pole in poles[1:]:
                v = delta(pole)
                n = (direction[1]*v[2]-direction[2]*v[1],
                     direction[2]*v[0]-direction[0]*v[2],
                     direction[0]*v[1]-direction[1]*v[0])
                if any(n):
                    normal = n
                    break
        if normal is not None:
            for index, pole in enumerate(poles):
                determinant = sum(normal[k]*delta(pole)[k] for k in range(3))
                if determinant:
                    witness = dict(pole=index, exactDeterminant=str(determinant))
                    break
        reports.append(dict(face=face_id, controlPoles=len(poles),
                            exactCoplanar=witness is None, nonplanarityWitness=witness))
    return reports

def converged_occt_volume(shape):
    # Span-aware Gauss-Kronrod remains entirely independent of source/reference
    # volumes. Require agreement at two requested accuracies; the old adaptive
    # Gauss error estimate alone can understate error on rational straight edges.
    values = []
    for eps in [1e-10, 1e-12]:
        props = GProp_GProps()
        error = BRepGProp.VolumePropertiesGK_s(shape, props, eps, True, True)
        values.append(dict(eps=eps, volume=props.Mass(), estimatedError=error))
    volume = values[-1]['volume']
    threshold = 1e-10*max(1., abs(volume))
    converged = (all(math.isfinite(v['volume']) and math.isfinite(v['estimatedError'])
                     and v['estimatedError'] >= 0 for v in values)
                 and abs(values[0]['volume']-volume) <= threshold)
    return volume, values[-1]['estimatedError'], dict(method='OpenCascade/span-aware-Gauss-Kronrod',
        values=values, agreementThreshold=threshold, converged=converged)


root = Path(sys.argv[1])
manifest = json.loads((root / 'manifest.json').read_text())
if not manifest['cases']:
    raise ValueError('Empty sweep oracle matrix')
reports = []
for case_index, case in enumerate(manifest['cases'], start=1):
    print(f"OCCT {case_index}/{len(manifest['cases'])}: {case['file']}", file=sys.stderr, flush=True)
    if case.get('volumeReferenceMethod') == 'analytic-hollow-rectangular-prism':
        reference = case.get('volumeReference', {})
        if reference.get('method') != case['volumeReferenceMethod']:
            raise ValueError('Compute the independent rectangular prism reference before OCCT verification')
        exact_volume = Fraction(int(reference['numerator']), int(reference['denominator']))
        if exact_volume <= 0 or float(exact_volume) != case['expectedVolume']:
            raise ValueError('Rectangular prism volume reference metadata mismatch')
    if case.get('volumeReferenceMethod') == 'polynomial-boundary-divergence-integral':
        reference = case.get('volumeReference', {})
        if reference.get('method') != case['volumeReferenceMethod']:
            raise ValueError('Compute the independent polynomial volume reference before OCCT verification')
        exact_volume = Fraction(int(reference['numerator']), int(reference['denominator']))
        if exact_volume <= 0 or float(exact_volume) != case['expectedVolume']:
            raise ValueError('Polynomial volume reference metadata mismatch')
    if case.get('volumeReferenceMethod') == 'rational-boundary-gauss-reference':
        reference = case.get('volumeReference', {})
        values = reference.get('values', [])
        if reference.get('method') != case['volumeReferenceMethod'] or reference.get('orders') != [16,32,64] or len(values) != 3:
            raise ValueError('Compute independent converged rational volume before OCCT verification')
        threshold = 1e-12*max(1.,abs(values[-1]))
        if not all(math.isfinite(v) and v > 0 for v in values) or values[-1] != case['expectedVolume'] or any(abs(values[i+1]-values[i]) > threshold for i in range(2)):
            raise ValueError('Rational numerical volume reference metadata mismatch')
    path = root / case['file']
    if hashlib.sha256(path.read_bytes()).hexdigest() != case['sha256']:
        raise ValueError('Fixture hash mismatch: ' + case['file'])
    reader = STEPControl_Reader()
    if reader.ReadFile(str(path)) != IFSelect_RetDone or reader.TransferRoots() != 1:
        raise ValueError('STEP import failed: ' + case['file'])
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    counts = []
    for kind in [TopAbs_FACE, TopAbs_SOLID, TopAbs_SHELL]:
        explorer = TopExp_Explorer(shape, kind)
        count = 0
        while explorer.More():
            count += 1
            explorer.Next()
        counts.append(count)
    surfaces = []
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while explorer.More():
        surfaces.append(BRepAdaptor_Surface(TopoDS.Face(explorer.Current())))
        explorer.Next()
    matched = set()
    imported_wall_charts = {}
    surface_error = 0.0
    derivative_error = 0.0
    coefficient_preserved = True
    exact_shared_basis = True
    # Rust STEP canonicalizes reversed shell face uses by U -> u0+u1-U.
    # Verify that exact chart correspondence, including coefficients and UVs,
    # rather than comparing different parameters on the same retained face.
    wall_reversed = case.get('wallFaceReversed', [False]*len(case['wallSamples']))
    if len(wall_reversed) != len(case['wallSamples']):
        raise ValueError('Missing wall face orientation correspondence')
    wall_u_sums = []
    for expected in case['wallSurfaces']:
        degree = expected['degreeU']
        wall_u_sums.append(expected['knotsU'][degree]+expected['knotsU'][-degree-1])
    for chart_index, samples in enumerate(case['wallSamples']):
        reversed_u = wall_reversed[chart_index]
        u_sum = wall_u_sums[chart_index]
        def imported_u(u):
            return u_sum-u if reversed_u else u
        candidates = []
        for index, surface in enumerate(surfaces):
            if index in matched:
                continue
            error = max(max(abs(a-b) for a,b in zip(surface.Value(imported_u(sample['u']), sample['v']).Coord(), sample['point'])) for sample in samples)
            candidates.append((error, index))
        error, index = min(candidates)
        matched.add(index)
        imported_wall_charts[index] = chart_index
        expected = case['wallSurfaces'][chart_index]
        spline = surfaces[index].BSpline()
        compatible = (spline.UDegree() == expected['degreeU'] and
                      spline.VDegree() == expected['degreeV'] and
                      spline.NbUPoles() == len(expected['controlPoints']) and
                      spline.NbVPoles() == len(expected['controlPoints'][0]) and
                      spline.IsUPeriodic() == bool(expected.get('periodicU', False)) and
                      spline.IsVPeriodic() == bool(expected.get('periodicV', False)))
        knot_u = [spline.UKnot(i) for i in range(1, spline.NbUKnots()+1) for _ in range(spline.UMultiplicity(i))]
        knot_v = [spline.VKnot(i) for i in range(1, spline.NbVKnots()+1) for _ in range(spline.VMultiplicity(i))]
        expected_u_knots = [u_sum-k for k in reversed(expected['knotsU'])] if reversed_u else expected['knotsU']
        shared_basis = compatible and knot_u == expected_u_knots and knot_v == expected['knotsV']
        if compatible:
            for u, row in enumerate(expected['controlPoints']):
                for v, control in enumerate(row):
                    imported_pole_u = len(expected['controlPoints'])-u if reversed_u else u+1
                    shared_basis &= spline.Weight(imported_pole_u,v+1) == expected['weights'][u][v] and expected['weights'][u][v] > 0
                    squared = sum((Fraction(a)-Fraction(b))**2 for a,b in zip(spline.Pole(imported_pole_u,v+1).Coord(),control))
                    coefficient_preserved &= squared <= Fraction(case['surfaceToleranceMm'])**2
        exact_shared_basis &= shared_basis
        coefficient_preserved &= compatible
        surface_error = max(surface_error, error)
        for sample in samples:
            point, du, dv = gp_Pnt(), gp_Vec(), gp_Vec()
            surfaces[index].D1(imported_u(sample['u']), sample['v'], point, du, dv)
            if reversed_u:
                du.Reverse()
            if sample['du'] is None or sample['dv'] is None:
                raise ValueError('Missing source derivative sample')
            for actual, expected in [(du.Coord(), sample['du']), (dv.Coord(), sample['dv'])]:
                derivative_error = max(derivative_error, max(abs(a-b) for a,b in zip(actual,expected)))
    surface_preserved = bool(case['wallSamples']) and max(surface_error, derivative_error) <= case['surfaceToleranceMm']
    edge_faces = IndexedDataMap_TopoDS_Shape_List_TopoDS_Shape_TopTools_ShapeMapHasher()
    TopExp.MapShapesAndAncestors_s(shape, TopAbs_EDGE, TopAbs_FACE, edge_faces)
    edge_uses = [edge_faces.FindFromIndex(i).Size() for i in range(1,edge_faces.Extent()+1)]
    imported_curves = [BRepAdaptor_Curve(TopoDS.Edge(edge_faces.FindKey(i))) for i in range(1,edge_faces.Extent()+1)]
    matched_edges = set()
    imported_to_source_edge = {}
    edge_coefficients_preserved = True
    for source_index, source in enumerate(case['edgeCurves']):
        errors = [(max(max(abs(a-b) for a,b in zip(curve.Value(sample['u']).Coord(),sample['point'])) for sample in source['samples']), index) for index,curve in enumerate(imported_curves) if index not in matched_edges]
        error, index = min(errors)
        matched_edges.add(index)
        imported_to_source_edge[index] = source_index
        spline = imported_curves[index].BSpline()
        expected = source['curve']
        knots = [spline.Knot(i) for i in range(1,spline.NbKnots()+1) for _ in range(spline.Multiplicity(i))]
        compatible = (spline.Degree() == expected['degree'] and spline.NbPoles() == len(expected['controlPoints']) and knots == expected['knots'] and spline.IsPeriodic() == bool(expected.get('periodic',False)))
        edge_coefficients_preserved &= compatible
        if compatible:
            edge_coefficients_preserved &= same_positive_projective_weights(
                [spline.Weight(i+1) for i in range(spline.NbPoles())], expected['weights'])
            for i, control in enumerate(expected['controlPoints']):
                squared = sum((Fraction(a)-Fraction(b))**2 for a,b in zip(spline.Pole(i+1).Coord(),control))
                edge_coefficients_preserved &= squared <= Fraction(case['surfaceToleranceMm'])**2
        edge_coefficients_preserved &= error <= case['surfaceToleranceMm']
    def cycle_key(ids):
        if not ids:
            raise ValueError('Empty boundary loop')
        rotations = []
        for order in [ids, list(reversed(ids))]:
            rotations.extend(tuple(order[i:]+order[:i]) for i in range(len(order)))
        return min(rotations)
    def loop_key(outer, holes):
        return (cycle_key(outer), tuple(sorted(cycle_key(hole) for hole in holes)))
    expected_loops = sorted(loop_key(face['outer'],face['holes']) for face in case['faceLoops'])
    actual_loops = []
    face_explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while face_explorer.More():
        face = TopoDS.Face(face_explorer.Current())
        outer_wire = BRepTools.OuterWire_s(face)
        outer, holes = None, []
        wires = TopExp_Explorer(face, TopAbs_WIRE)
        while wires.More():
            wire = wires.Current()
            if not BRep_Tool.IsClosed_s(wire):
                raise ValueError('Open imported boundary wire')
            edges = BRepTools_WireExplorer(TopoDS.Wire(wire),face)
            ids = []
            while edges.More():
                ids.append(imported_to_source_edge[edge_faces.FindIndex(edges.Current())-1])
                edges.Next()
            if wire.IsSame(outer_wire):
                outer = ids
            else:
                holes.append(ids)
            wires.Next()
        if outer is None:
            raise ValueError('Missing imported outer wire')
        actual_loops.append(loop_key(outer,holes))
        face_explorer.Next()
    face_loop_ownership_preserved = sorted(actual_loops) == expected_loops
    pcurve_samples = 0
    pcurve_error = 0.0
    pcurve_derivative_error = 0.0
    same_parameter = True
    wall_pcurve_full_domain = True if "wallCoedges" in case else None
    wall_pcurve_uses = 0
    cap_coedges_full_domain = True
    cap_coedge_uses = 0
    imported_face_index = 0
    oriented_uses = [[] for _ in edge_uses]
    face_explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while face_explorer.More():
        edge_explorer = TopExp_Explorer(face_explorer.Current(), TopAbs_EDGE)
        while edge_explorer.More():
            edge = edge_explorer.Current()
            face = TopoDS.Face(face_explorer.Current())
            edge3d = TopoDS.Edge(edge)
            pcurve = BRepAdaptor_Curve2d(edge3d,face)
            curve = BRepAdaptor_Curve(edge3d)
            surface = BRepAdaptor_Surface(face)
            same_parameter &= BRep_Tool.SameParameter_s(edge3d)
            if pcurve.FirstParameter() != curve.FirstParameter() or pcurve.LastParameter() != curve.LastParameter():
                raise ValueError('Pcurve parameter interval mismatch')

            if imported_face_index in imported_wall_charts and wall_pcurve_full_domain is not None:
                chart = imported_wall_charts[imported_face_index]
                edge_id = imported_to_source_edge[edge_faces.FindIndex(edge)-1]
                expected_uses = [u for u in case['wallCoedges'][chart] if u['edge'] == edge_id]
                linear = pcurve.GetType() == GeomAbs_Line
                exact_uv_endpoints = None
                if linear:
                    line2d = pcurve.Line()
                    origin, direction = line2d.Location().Coord(), line2d.Direction().Coord()
                    exact_uv_endpoints = [[Fraction(o)+Fraction(t)*Fraction(d) for o,d in zip(origin,direction)]
                                          for t in [pcurve.FirstParameter(),pcurve.LastParameter()]]
                if pcurve.GetType() == GeomAbs_BSplineCurve:
                    spline2d = pcurve.BSpline()
                    linear = (spline2d.Degree() == 1 and spline2d.NbPoles() == 2
                              and not spline2d.IsPeriodic() and spline2d.NbKnots() == 2
                              and spline2d.Multiplicity(1) == 2 and spline2d.Multiplicity(2) == 2
                              and spline2d.Knot(1) == pcurve.FirstParameter()
                              and spline2d.Knot(2) == pcurve.LastParameter()
                              and spline2d.Weight(1) == spline2d.Weight(2) and spline2d.Weight(1) > 0)
                if pcurve.GetType() == GeomAbs_BSplineCurve and linear:
                    exact_uv_endpoints = [[Fraction(x) for x in spline2d.Pole(i).Coord()] for i in [1,2]]
                identity = False
                if linear and len(expected_uses) == 1:
                    use = expected_uses[0]
                    expected_uv = use['pcurve']
                    points = expected_uv['controlPoints']
                    compatible_uv = (expected_uv['degree'] == 1 and expected_uv['knots'] == [0,0,1,1]
                                     and not expected_uv.get('periodic',False) and len(points) == 2
                                     and expected_uv['weights'][0] == expected_uv['weights'][1]
                                     and expected_uv['weights'][0] > 0)
                    if wall_reversed[chart]:
                        points = [[wall_u_sums[chart]-point[0], point[1]] for point in points]
                    if use['reversed']:
                        points = list(reversed(points))
                    identity = compatible_uv and all(
                        sum((a-Fraction(b))**2 for a,b in zip(endpoint, point))
                        <= Fraction(case['surfaceToleranceMm'])**2
                        for endpoint,point in zip(exact_uv_endpoints,points))
                wall_pcurve_full_domain &= identity
                wall_pcurve_uses += 1
            if imported_face_index not in imported_wall_charts:
                cap_coedges_full_domain &= full_domain_cap_coedge(surface,pcurve,curve,case['surfaceToleranceMm'])
                cap_coedge_uses += 1
            first, last = curve.FirstParameter(), curve.LastParameter()
            for fraction in [0.,.125,.25,.5,.75,.875,1.]:
                parameter = first*(1-fraction)+last*fraction
                uv = pcurve.Value(parameter)
                actual = surface.Value(uv.X(),uv.Y()).Coord()
                expected = curve.Value(parameter).Coord()
                pcurve_error = max(pcurve_error,max(abs(a-b) for a,b in zip(actual,expected)))
                uv_point, uv_derivative = gp_Pnt2d(),gp_Vec2d()
                pcurve.D1(parameter,uv_point,uv_derivative)
                surface_point,du,dv = gp_Pnt(),gp_Vec(),gp_Vec()
                surface.D1(uv.X(),uv.Y(),surface_point,du,dv)
                curve_point,curve_derivative = gp_Pnt(),gp_Vec()
                curve.D1(parameter,curve_point,curve_derivative)
                composed = [a*uv_derivative.X()+b*uv_derivative.Y() for a,b in zip(du.Coord(),dv.Coord())]
                pcurve_derivative_error = max(pcurve_derivative_error,max(abs(a-b) for a,b in zip(composed,curve_derivative.Coord())))
                pcurve_samples += 1
            oriented_uses[edge_faces.FindIndex(edge)-1].append(edge.Orientation())
            edge_explorer.Next()
        face_explorer.Next()
        imported_face_index += 1
    opposite_edge_uses = all(uses.count(TopAbs_FORWARD) == 1 and uses.count(TopAbs_REVERSED) == 1 and len(uses) == 2 for uses in oriented_uses)
    manifold_edges = bool(edge_uses) and all(uses == 2 for uses in edge_uses)
    shell_volumes = []
    shell_volume_integrations = []
    shell_closed = []
    explorer = TopExp_Explorer(shape, TopAbs_SHELL)
    while explorer.More():
        shell_closed.append(BRep_Tool.IsClosed_s(explorer.Current()))
        shell_volume, _, shell_integration = converged_occt_volume(explorer.Current())
        shell_volumes.append(shell_volume)
        shell_volume_integrations.append(shell_integration)
        explorer.Next()
    hole_faces = 0
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    while explorer.More():
        wires = TopExp_Explorer(explorer.Current(), TopAbs_WIRE)
        wire_count = 0
        while wires.More():
            wire_count += 1
            wires.Next()
        hole_faces += int(wire_count > 1)
        explorer.Next()
    shell_orientation = (sum(v > 0 for v in shell_volumes) == 1 and
                         sum(v < 0 for v in shell_volumes) == case['shells'] - 1)
    props = GProp_GProps()
    legacy_integration_error = BRepGProp.VolumeProperties_s(shape, props, 1e-10, True)
    legacy_volume = props.Mass()
    volume, integration_error, volume_integration = converged_occt_volume(shape)
    relative = abs(volume-case['expectedVolume'])/case['expectedVolume']
    solid_explorer = TopExp_Explorer(shape, TopAbs_SOLID)
    if not solid_explorer.More():
        raise ValueError('Missing imported solid for material-side probes')
    imported_solid = TopoDS.Solid(solid_explorer.Current())
    classifier = BRepClass3d_SolidClassifier(imported_solid)
    material_side_probes = 0
    material_side_agreement = True
    face_explorer = TopExp_Explorer(shape, TopAbs_FACE)
    face_index = 0
    while face_explorer.More():
        if face_index in imported_wall_charts:
            face = TopoDS.Face(face_explorer.Current())
            surface = BRepAdaptor_Surface(face)
            point, du, dv = gp_Pnt(), gp_Vec(), gp_Vec()
            surface.D1(.5,.5,point,du,dv)
            normal = du.Crossed(dv)
            if normal.Magnitude() == 0:
                raise ValueError('Singular wall probe normal')
            normal.Normalize()
            if face.Orientation() == TopAbs_REVERSED:
                normal.Reverse()
            for multiplier in [100.,1000.]:
                distance = case['surfaceToleranceMm']*multiplier
                for sign, expected_state in [(-1.,TopAbs_IN),(1.,TopAbs_OUT)]:
                    probe = point.Translated(normal.Multiplied(sign*distance))
                    classifier.Perform(probe,case['surfaceToleranceMm'])
                    material_side_agreement &= classifier.State() == expected_state
                    material_side_probes += 1
        face_explorer.Next()
        face_index += 1
    full_uv_gate = (wall_pcurve_full_domain is True and wall_pcurve_uses == 4*len(case["wallCoedges"])) if manifest.get("schema") == "sweep-external-step/2" else True
    passed = material_side_agreement and material_side_probes>0 and full_uv_gate and same_parameter and pcurve_samples > 0 and max(pcurve_error,pcurve_derivative_error) <= case['surfaceToleranceMm'] and face_loop_ownership_preserved and edge_coefficients_preserved and len(matched_edges) == case['edges'] and len(edge_uses) == case['edges'] and opposite_edge_uses and manifold_edges and all(shell_closed) and coefficient_preserved and exact_shared_basis and surface_preserved and shell_orientation and hole_faces == case['capHoleFaces'] and valid and counts == [case['faces'], case['solids'], case['shells']] and volume > 0 and relative <= case['relativeVolumeTolerance']
    passed &= volume_integration['converged'] and all(s['converged'] for s in shell_volume_integrations)
    expected_cap_uses = sum(len(case['faceLoops'][i]['outer'])+sum(len(h) for h in case['faceLoops'][i]['holes']) for i in case.get('capFaces',[]))
    cap_full_domain_gate = cap_coedges_full_domain and cap_coedge_uses == expected_cap_uses
    passed &= cap_full_domain_gate
    native = case.get('nativeVolume')
    cap_planarity = exact_cap_planarity(case)
    # Positive retained-region evidence must agree with an independent exact
    # rational audit of every original boundary pole, including hole poles.
    retained_caps = case.get('retainedCaps') or case.get('construction', {}).get('retainedCaps')
    retained_cap_planarity_agreement = None
    cap_decomposition = case.get('retainedCapDecomposition') or case.get('construction', {}).get('retainedCapDecomposition')
    decomposed_regions_claimed = cap_decomposition and cap_decomposition.get('certified') is True
    if (retained_caps and retained_caps.get('exact')) or decomposed_regions_claimed:
        # Numeric decomposition proves identity only with the decomposed loops.
        # Its positive region claim still requires exact planarity of the actual
        # stored cap poles, independently of OCCT's modeling tolerance.
        regions = (cap_decomposition.get('regions') or {}) if decomposed_regions_claimed else None
        region_evidence = (not decomposed_regions_claimed or
                           (regions.get('exact') is True and regions.get('faces') == [case['faces']-2, case['faces']-1]))
        retained_cap_planarity_agreement = (region_evidence and len(cap_planarity) == len(case.get('capFaces', []))
                                            and all(p['exactCoplanar'] for p in cap_planarity))
        passed = passed and retained_cap_planarity_agreement
    material_agreement = None
    if native and native.get('solidGeometryCertified') is True:
        parents = (native.get('nesting') or {}).get('parents')
        orientations = native.get('orientations', [])
        material_agreement = (native.get('boundaryEmbeddingCertified') is True
                              and (native.get('nesting') or {}).get('rolesConsistent') is True
                              and isinstance(parents, list) and len(parents) == counts[2]
                              and sum(p is None for p in parents) == 1
                              and len(orientations) == counts[2]
                              and all(s.get('outward') is s.get('expectedOutward') for s in orientations)
                              and sum(s.get('outward') is True for s in orientations) == sum(v > 0 for v in shell_volumes)
                              and sum(s.get('outward') is False for s in orientations) == sum(v < 0 for v in shell_volumes))
        passed = passed and material_agreement
    reports.append(dict(file=case['file'], stepSha256=case['sha256'], retained_cap_planarity_agreement=retained_cap_planarity_agreement, authored_cap_exact_planarity=cap_planarity, native_volume_certified=native.get('solidGeometryCertified') if native else None, native_external_material_agreement=material_agreement, valid=valid, faces=counts[0], solids=counts[1], shells=counts[2], shell_closed=shell_closed, edges=len(edge_uses), same_parameter=same_parameter, sampled_material_side_probe_agreement=material_side_agreement, material_side_probes=material_side_probes, pcurve_samples=pcurve_samples, full_domain_wall_pcurve_uv_preserved=wall_pcurve_full_domain, full_domain_wall_pcurve_uses=wall_pcurve_uses, sampled_pcurve_surface_max_error_mm=pcurve_error, sampled_pcurve_derivative_max_error=pcurve_derivative_error, face_loop_ownership_preserved=face_loop_ownership_preserved, matched_source_edges=len(matched_edges), full_domain_edge_distance_within_tolerance=edge_coefficients_preserved, edge_face_uses=sorted(set(edge_uses)), manifold_edges=manifold_edges, opposite_edge_uses=opposite_edge_uses, sampled_wall_faces=len(matched), sampled_surface_max_error_mm=surface_error, sampled_derivative_max_error=derivative_error, surface_preserved=surface_preserved, control_net_within_tolerance=coefficient_preserved, exact_shared_basis=exact_shared_basis, full_domain_wall_distance_within_tolerance=coefficient_preserved and exact_shared_basis, shell_volumes=shell_volumes, cap_hole_faces=hole_faces, shell_orientation=shell_orientation, volume=volume, expected_volume=case['expectedVolume'], relative_volume_error=relative, integration_error=integration_error, passed=passed))
    reports[-1]['full_domain_cap_coedge_distance_within_tolerance'] = cap_full_domain_gate
    reports[-1]['volume_integration'] = volume_integration
    reports[-1]['shell_volume_integrations'] = shell_volume_integrations
    reports[-1]['legacy_adaptive_gauss'] = dict(volume=legacy_volume, estimatedError=legacy_integration_error)
    reports[-1]['full_domain_cap_coedge_uses'] = cap_coedge_uses
result = dict(oracle='OpenCascade/cadquery-ocp', manifestSha256=hashlib.sha256((root / 'manifest.json').read_bytes()).hexdigest(), artifactProvenance=manifest.get('artifactProvenance'), cases=reports, passed=all(r['passed'] for r in reports), scope='fixture import, topology and analytic volume; surface/seam/containment matrix remains separate')
(root / 'opencascade-sweep.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
if not result['passed']:
    sys.exit(1)
