import { compileModelGraphNurbs } from './modelGraphNurbs';
import { validateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot, elevateNurbsCurve, trimNurbsCurve, reverseNurbsCurve, nurbsCurveBounds, type NurbsCurve } from './nurbsCurve';
import { validateNurbsSurface, evaluateNurbsSurface, insertNurbsSurfaceKnot, elevateNurbsSurface, trimNurbsSurface, reverseNurbsSurface, isoNurbsCurve, nurbsSurfaceBounds, type NurbsSurface } from './nurbsSurface';
import { loftNurbsCurves, extrudeNurbsCurve, revolveNurbsCurve } from './nurbsConstructors';
import { tessellateNurbsSurface, thickenNurbsMesh, exportNurbsStl } from './nurbsTessellation';
type Mesh = ReturnType<typeof tessellateNurbsSurface>;
type Value = {
    kind: 'curve';
    data: NurbsCurve;
} | {
    kind: 'surface';
    data: NurbsSurface;
} | {
    kind: 'mesh';
    data: Mesh;
};
export type OwnNurbsRequest = {
    action: 'build' | 'evaluate' | 'export';
    format?: 'json' | 'stl';
    evaluations?: Array<{
        node: string;
        u: number;
        v?: number;
    }>;
};
export function buildOwnNurbs(document: unknown, request: OwnNurbsRequest) {
    if (!['build', 'evaluate', 'export'].includes(request.action) || (request.evaluations?.length ?? 0) > 64)
        throw new Error('Invalid NURBS request or evaluation budget exceeded.');
    const compiled = compileModelGraphNurbs(document), nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n])), cache = new Map<string, Value>();
    const needCurve = (key: string) => { const result = get(key); if (result.kind !== 'curve')
        throw new Error(`Expected curve at ${key}.`); return result.data; };
    const needSurface = (key: string) => { const result = get(key); if (result.kind !== 'surface')
        throw new Error(`Expected surface at ${key}.`); return result.data; };
    const needMesh = (key: string) => { const result = get(key); if (result.kind !== 'mesh')
        throw new Error(`Expected tessellated mesh at ${key}.`); return result.data; };
    let meshTriangles = 0;
    function get(key: string): Value {
        const cached = cache.get(key);
        if (cached)
            return cached;
        const n = nodes.get(key);
        if (!n)
            throw new Error('Unknown node ' + key);
        let result: Value;
        try {
            switch (n.op) {
                case 'curve': {
                    const c: NurbsCurve = { degree: n.degree, knots: n.knots, controlPoints: n.control_points, weights: n.weights, periodic: n.periodic };
                    validateNurbsCurve(c);
                    result = { kind: 'curve', data: c };
                    break;
                }
                case 'surface': {
                    const s: NurbsSurface = { degreeU: n.degree_u, degreeV: n.degree_v, knotsU: n.knots_u, knotsV: n.knots_v, controlPoints: n.control_points, weights: n.weights, periodicU: n.periodic_u, periodicV: n.periodic_v };
                    validateNurbsSurface(s);
                    result = { kind: 'surface', data: s };
                    break;
                }
                case 'curve_edit': {
                    let c = needCurve(n.input);
                    for (const knot of n.insert_knots ?? [])
                        c = insertNurbsKnot(c, knot.value, knot.count);
                    if (n.degree !== undefined)
                        c = elevateNurbsCurve(c, n.degree);
                    if (n.trim)
                        c = trimNurbsCurve(c, ...n.trim);
                    if (n.reverse)
                        c = reverseNurbsCurve(c);
                    result = { kind: 'curve', data: c };
                    break;
                }
                case 'surface_edit': {
                    let s = needSurface(n.input);
                    for (const k of n.insert_knots_u ?? [])
                        s = insertNurbsSurfaceKnot(s, 'u', k.value, k.count);
                    for (const k of n.insert_knots_v ?? [])
                        s = insertNurbsSurfaceKnot(s, 'v', k.value, k.count);
                    if (n.degree_u !== undefined)
                        s = elevateNurbsSurface(s, 'u', n.degree_u);
                    if (n.degree_v !== undefined)
                        s = elevateNurbsSurface(s, 'v', n.degree_v);
                    if (n.trim)
                        s = trimNurbsSurface(s, ...n.trim);
                    if (n.reverse_u)
                        s = reverseNurbsSurface(s, 'u');
                    if (n.reverse_v)
                        s = reverseNurbsSurface(s, 'v');
                    result = { kind: 'surface', data: s };
                    break;
                }
                case 'iso_curve':
                    result = { kind: 'curve', data: isoNurbsCurve(needSurface(n.input), n.direction, n.parameter) };
                    break;
                case 'ruled_surface':
                    result = { kind: 'surface', data: loftNurbsCurves(n.inputs.map(needCurve)) };
                    break;
                case 'surface_extrude':
                    result = { kind: 'surface', data: extrudeNurbsCurve(needCurve(n.input), n.vector) };
                    break;
                case 'surface_revolve':
                    result = { kind: 'surface', data: revolveNurbsCurve(needCurve(n.input), n.origin, n.axis, n.angle) };
                    break;
                case 'transform': {
                    const m = n.matrix;
                    if (m[3].some((v, i) => v !== (i === 3 ? 1 : 0)))
                        throw new Error('Transform must be affine.');
                    const point = (p: number[]) => { if (p.length !== 3)
                        throw new Error('Affine transform requires 3D controls.'); const q = [0, 1, 2].map(i => m[i][3] + p.reduce((sum, v, j) => sum + m[i][j] * v, 0)); if (q.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6))
                        throw new Error('Transformed coordinates exceed numeric limits.'); return q; };
                    const v = get(n.input);
                    if (v.kind === 'curve') {
                        const c = { ...v.data, controlPoints: v.data.controlPoints.map(point) };
                        validateNurbsCurve(c);
                        result = { kind: 'curve', data: c };
                    }
                    else if (v.kind === 'surface') {
                        const s = { ...v.data, controlPoints: v.data.controlPoints.map(row => row.map(point)) };
                        validateNurbsSurface(s);
                        result = { kind: 'surface', data: s };
                    }
                    else
                        throw new Error('Transform spline control data before tessellation.');
                    break;
                }
                case 'tessellate': {
                    if (n.trim && n.trim_curves)
                        throw new Error('Use only one trim representation.');
                    const sample = (id: string) => { const c = needCurve(id); if (c.controlPoints[0].length !== 2)
                        throw new Error('UV trim requires a 2D curve.'); const a = c.knots[c.degree], b = c.knots[c.controlPoints.length]; const first = evaluateNurbsCurve(c, a).point, last = evaluateNurbsCurve(c, b).point; if (Math.hypot(...first.map((v, i) => v - last[i])) > 1e-9)
                        throw new Error('UV trim curve must be closed.'); return Array.from({ length: n.trim_segments }, (_, i) => evaluateNurbsCurve(c, a + (b - a) * i / n.trim_segments).point); };
                    const trim = n.trim_curves ? { outer: sample(n.trim_curves.outer), holes: n.trim_curves.holes.map(sample) } : n.trim;
                    result = { kind: 'mesh', data: tessellateNurbsSurface(needSurface(n.input), { segmentsU: n.segments_u, segmentsV: n.segments_v, ...(trim ? { trim } : {}) }) };
                    break;
                }
                case 'thicken':
                    result = { kind: 'mesh', data: thickenNurbsMesh(needMesh(n.input), n.vector) };
                    break;
            }
        }
        catch (error) {
            throw new Error(`Node ${key}: ${error instanceof Error ? error.message : 'NURBS operation failed.'}`);
        }
        if (result.kind === 'mesh' && (meshTriangles += result.data.indices.length / 3) > 60000)
            throw new Error('NURBS graph mesh work exceeds 60000 triangles.');
        cache.set(key, result);
        return result;
    }
    const root = get(compiled.document.root);
    const definitions = Object.fromEntries([...cache].filter(([, v]) => v.kind !== 'mesh').map(([id, v]) => [id, { kind: v.kind, ...v.data }]));
    const meshBounds = root.kind === 'mesh' ? { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] } : null;
    if (root.kind === 'mesh' && meshBounds)
        for (let i = 0; i < root.data.positions.length; i++) {
            const axis = i % 3, v = root.data.positions[i];
            meshBounds.min[axis] = Math.min(meshBounds.min[axis], v);
            meshBounds.max[axis] = Math.max(meshBounds.max[axis], v);
        }
    const report = { bounds_scope: root.kind === 'mesh' ? 'derived mesh vertices' : 'conservative control hull', kernel: 'own-typescript-nurbs', geometry_authority: 'rational control data', root: compiled.document.root, root_kind: root.kind, bounds: root.kind === 'curve' ? nurbsCurveBounds(root.data) : root.kind === 'surface' ? nurbsSurfaceBounds(root.data) : meshBounds, definitions, mesh: root.kind === 'mesh' ? root.data.report : null, printability: 'unknown', error_bound_certified: false };
    const evaluations = (request.evaluations ?? []).map(q => { const v = get(q.node); if (v.kind === 'curve')
        return { node: q.node, ...evaluateNurbsCurve(v.data, q.u) }; if (v.kind === 'surface' && q.v !== undefined)
        return { node: q.node, ...evaluateNurbsSurface(v.data, q.u, q.v) }; throw new Error('Evaluation requires a curve or surface, and v for a surface.'); });
    const base = { ok: true, document_sha256: compiled.document_sha256, execution_target: 'own-nurbs', automatic_fallback: false, report, evaluations };
    if (request.action === 'export') {
        if (request.format === 'json')
            return { ...base, artifact: { format: 'json', mime_type: 'application/json', text: JSON.stringify(compiled.document, null, 2) } };
        if (request.format !== 'stl' || root.kind !== 'mesh')
            throw new Error('STL export requires a closed tessellated mesh. Export JSON to retain native NURBS definitions.');
        const text = exportNurbsStl(root.data);
        if (text.length > 4 * 1024 * 1024)
            throw new Error('STL export exceeds 4 MiB.');
        return { ...base, artifact: { format: 'stl', mime_type: 'model/stl', text } };
    }
    return { ...base, ...(request.action === 'build' && root.kind === 'mesh' ? { mesh: root.data } : {}) };
}
