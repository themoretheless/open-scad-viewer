import { validateNurbsCurve, type NurbsCurve } from './nurbsCurve';
import { validateNurbsSurface, type NurbsSurface } from './nurbsSurface';
/** Compatible curves become the rows of a rational ruled surface in homogeneous coordinates. */
export function loftNurbsCurves(curves: NurbsCurve[]): NurbsSurface {
    if (curves.length < 2 || curves.length > 32)
        throw new Error('Loft requires 2..32 compatible curves.');
    curves.forEach(validateNurbsCurve);
    const base = curves[0];
    if (base.controlPoints[0].length !== 3 || curves.some(c => c.degree !== base.degree || c.knots.length !== base.knots.length || c.knots.some((k, i) => k !== base.knots[i]) || c.controlPoints.length !== base.controlPoints.length || c.controlPoints[0].length !== 3))
        throw new Error('Loft curves must have identical degree, knots, dimension and control count; refine them first.');
    const surface: NurbsSurface = { degreeU: base.degree, degreeV: 1, knotsU: [...base.knots], knotsV: [0, ...curves.map((_, i) => i), curves.length - 1], controlPoints: base.controlPoints.map((_, i) => curves.map(c => [...c.controlPoints[i]])), weights: base.weights.map((_, i) => curves.map(c => c.weights[i])), periodicU: base.periodic, periodicV: false };
    validateNurbsSurface(surface);
    return surface;
}
export function extrudeNurbsCurve(curve: NurbsCurve, vector: number[]): NurbsSurface {
    if (vector.length !== 3 || !vector.every(Number.isFinite) || Math.hypot(...vector) === 0)
        throw new Error('Extrusion vector must be finite and nonzero.');
    validateNurbsCurve(curve);
    return loftNurbsCurves([curve, { ...curve, controlPoints: curve.controlPoints.map(p => p.map((v, i) => v + vector[i])) }]);
}
/** Rational quadratic arcs (at most 90 degrees each) preserve a circular revolution. */
export function revolveNurbsCurve(curve: NurbsCurve, origin: number[], axis: number[], angle: number): NurbsSurface {
    validateNurbsCurve(curve);
    if (curve.controlPoints[0].length !== 3 || origin.length !== 3 || axis.length !== 3 || ![...origin, ...axis, angle].every(Number.isFinite) || Math.hypot(...axis) === 0 || angle === 0 || Math.abs(angle) > 360)
        throw new Error('Revolution requires 3D data, a nonzero axis and angle within +/-360 degrees.');
    const norm = Math.hypot(...axis), unit = axis.map(v => v / norm), arcs = Math.ceil(Math.abs(angle) / 90), delta = angle * Math.PI / 180 / arcs;
    const knotsV = [0, 0, 0];
    for (let i = 1; i < arcs; i++)
        knotsV.push(i, i);
    knotsV.push(arcs, arcs, arcs);
    const arcWeights = Array.from({ length: 2 * arcs + 1 }, (_, j) => j % 2 ? Math.cos(delta / 2) : 1);
    const controlPoints = curve.controlPoints.map(p => {
        const relative = p.map((v, i) => v - origin[i]), projection = relative.reduce((sum, v, i) => sum + v * unit[i], 0);
        const center = origin.map((v, i) => v + projection * unit[i]), radial = p.map((v, i) => v - center[i]);
        const cross = [unit[1] * radial[2] - unit[2] * radial[1], unit[2] * radial[0] - unit[0] * radial[2], unit[0] * radial[1] - unit[1] * radial[0]];
        return arcWeights.map((w, j) => j === 2 * arcs && Math.abs(angle) === 360 ? [...p] : center.map((v, i) => v + (Math.cos(j * delta / 2) * radial[i] + Math.sin(j * delta / 2) * cross[i]) / w));
    });
    const result: NurbsSurface = { degreeU: curve.degree, degreeV: 2, knotsU: [...curve.knots], knotsV, controlPoints, weights: curve.weights.map(w => arcWeights.map(v => w * v)), periodicU: curve.periodic, periodicV: false };
    validateNurbsSurface(result);
    return result;
}
