import { expect, it } from 'vitest';
import { compileModelGraphNurbs, MODELGRAPH_NURBS_EXAMPLE, MODELGRAPH_NURBS_SURFACE_EXAMPLE } from '../src/services/modelGraphNurbs';
import { buildOwnNurbs } from '../src/services/modelGraphNurbsKernel';
import { runOwnNurbs } from '../src/mcp/modelGraphNurbsRuntime';
import { extrudeNurbsCurve, revolveNurbsCurve, loftNurbsCurves } from '../src/services/nurbsConstructors';
import { evaluateNurbsSurface } from '../src/services/nurbsSurface';
import type { NurbsCurve } from '../src/services/nurbsCurve';
const line: NurbsCurve = { degree: 1, knots: [0, 0, 1, 1], controlPoints: [[10, 0, 0], [10, 0, 20]], weights: [1, 1] };
it('builds an exact rational cylinder surface from its generating line', () => {
    const surface = revolveNurbsCurve(line, [0, 0, 0], [0, 0, 1], 360);
    for (let u = 0; u <= 1; u += 0.2)
        for (let v = 0; v <= 4; v += 0.2) {
            const p = evaluateNurbsSurface(surface, u, v).point;
            expect(Math.hypot(p[0], p[1])).toBeCloseTo(10, 10);
            expect(p[2]).toBeCloseTo(20 * u, 10);
        }
});
it('extrudes and joins compatible curves without a spline dependency', () => {
    const s = extrudeNurbsCurve(line, [0, 5, 0]);
    expect(evaluateNurbsSurface(s, 0.5, 0.5).point).toEqual([10, 2.5, 10]);
    expect(() => loftNurbsCurves([line, { ...line, knots: [0, 0, 2, 2] }])).toThrow('identical');
});
it('resolves parameters and rejects cycles, unreachable nodes and undefined parameters', () => {
    const compiled = compileModelGraphNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE);
    expect(compiled.document_sha256).toHaveLength(64);
    const document = JSON.parse(JSON.stringify(MODELGRAPH_NURBS_SURFACE_EXAMPLE));
    document.parameters = [];
    expect(() => compileModelGraphNurbs(document)).toThrow('Unknown parameter');
    expect(() => compileModelGraphNurbs({ ...MODELGRAPH_NURBS_EXAMPLE, nodes: [{ id: 'a', op: 'curve_edit', input: 'a' }], root: 'a' })).toThrow('Cyclic');
    expect(() => compileModelGraphNurbs({ ...MODELGRAPH_NURBS_EXAMPLE, nodes: [...MODELGRAPH_NURBS_EXAMPLE.nodes, { id: 'b', op: 'curve_edit', input: 'arc' }] })).toThrow('reachable');
});
it('builds and exports a closed curved plate through the actual own-kernel process', async () => {
    const r = await runOwnNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE, { action: 'build' });
    expect(r.ok, JSON.stringify(r)).toBe(true);
    if (!r.ok)
        throw new Error('Build failed');
    expect(r.execution_target).toBe('own-nurbs');
    expect(r.report.mesh?.closed).toBe(true);
    expect(r.report.mesh?.nonManifoldEdges).toBe(0);
    expect(Math.abs(r.report.mesh!.signedVolumeMm3)).toBeCloseTo(800, 5);
    const exported = await runOwnNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE, { action: 'export', format: 'stl' });
    expect(exported.ok).toBe(true);
    if (!exported.ok || !('artifact' in exported))
        throw new Error('Export failed');
    expect(exported.artifact?.text).toContain('facet normal');
}, 15000);
it('returns readable rational definitions and derivative queries after editing', async () => {
    const document = { ...MODELGRAPH_NURBS_EXAMPLE, nodes: [...MODELGRAPH_NURBS_EXAMPLE.nodes, { id: 'refined', op: 'curve_edit', input: 'arc', insert_knots: [{ value: 0.5, count: 1 }], degree: 4 }], root: 'refined' };
    const r = buildOwnNurbs(document, { action: 'evaluate', evaluations: [{ node: 'refined', u: 0.25 }, { node: 'arc', u: 0.25 }] });
    expect(r.report.definitions.refined).toBeTruthy();
    const a = r.evaluations[0]!.point, b = r.evaluations[1]!.point;
    a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 10));
    const failure = await runOwnNurbs({ ...MODELGRAPH_NURBS_EXAMPLE, nodes: [{ ...MODELGRAPH_NURBS_EXAMPLE.nodes[0], weights: [1, 0, 1] }] }, { action: 'build' });
    expect(failure.ok).toBe(false);
    await expect(runOwnNurbs(MODELGRAPH_NURBS_EXAMPLE, { action: 'build' }, AbortSignal.abort())).rejects.toThrow('cancelled');
    expect((await runOwnNurbs(MODELGRAPH_NURBS_EXAMPLE, { action: 'build' })).ok).toBe(true);
});
it('retains native JSON and refuses STL for an open curve', () => {
    const result = buildOwnNurbs(MODELGRAPH_NURBS_EXAMPLE, { action: 'export', format: 'json' });
    expect('artifact' in result && JSON.parse(result.artifact!.text).language).toBe('modelgraph/nurbs-1');
    expect(() => buildOwnNurbs(MODELGRAPH_NURBS_EXAMPLE, { action: 'export', format: 'stl' })).toThrow('closed tessellated mesh');
});
it('retains rational UV trim definitions while producing a bounded closed disk mesh', () => {
    const w = Math.SQRT1_2, r = 0.4;
    const circle = [[r, 0], [r, r], [0, r], [-r, r], [-r, 0], [-r, -r], [0, -r], [r, -r], [r, 0]].map(p => p.map(v => v + 0.5));
    const doc = { language: 'modelgraph/nurbs-1', units: 'mm', parameters: [], nodes: [
            { id: 'plane', op: 'surface', degree_u: 1, degree_v: 1, knots_u: [0, 0, 1, 1], knots_v: [0, 0, 1, 1], control_points: [[[0, 0, 0], [0, 20, 0]], [[20, 0, 0], [20, 20, 0]]], weights: [[1, 1], [1, 1]] },
            { id: 'trim', op: 'curve', degree: 2, knots: [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4], control_points: circle, weights: [1, w, 1, w, 1, w, 1, w, 1] },
            { id: 'skin', op: 'tessellate', input: 'plane', trim_curves: { outer: 'trim', holes: [] }, trim_segments: 64 },
            { id: 'disk', op: 'thicken', input: 'skin', vector: [0, 0, 2] },
        ], root: 'disk' };
    const result = buildOwnNurbs(doc, { action: 'build' });
    expect(result.report.definitions.trim).toBeTruthy();
    expect(result.report.mesh?.closed).toBe(true);
    expect(Math.abs(result.report.mesh!.signedVolumeMm3) / (Math.PI * 8 * 8 * 2)).toBeCloseTo(1, 2);
    expect(result.report.mesh?.errorBoundCertified).toBe(false);
});
it('enforces depth through a shared subgraph reached earlier on a shallow branch', () => {
    const nodes: Record<string, unknown>[] = [...MODELGRAPH_NURBS_EXAMPLE.nodes];
    for (let i = 0; i < 29; i++)
        nodes.push({ id: `a${i}`, op: 'curve_edit', input: i ? `a${i - 1}` : 'arc' });
    for (let i = 0; i < 29; i++)
        nodes.push({ id: `b${i}`, op: 'curve_edit', input: i ? `b${i - 1}` : 'a28' });
    nodes.push({ id: 'join', op: 'ruled_surface', inputs: ['a28', 'b28'] });
    expect(() => compileModelGraphNurbs({ ...MODELGRAPH_NURBS_EXAMPLE, nodes, root: 'join' })).toThrow('depth');
});

it('builds mesh CSG through the NURBS graph and handles empty intersections', async () => {
    const nodes = [
        {id:'patch',op:'surface',degree_u:1,degree_v:1,knots_u:[0,0,1,1],knots_v:[0,0,1,1],control_points:[[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]],weights:[[1,1],[1,1]]},
        {id:'skin',op:'tessellate',input:'patch',segments_u:2,segments_v:3},
        {id:'a',op:'thicken',input:'skin',vector:[0,0,2]},
        {id:'shifted',op:'transform',input:'patch',matrix:[[1,0,0,1],[0,1,0,1],[0,0,1,1],[0,0,0,1]]},
        {id:'skin_b',op:'tessellate',input:'shifted',segments_u:3,segments_v:2},
        {id:'b',op:'thicken',input:'skin_b',vector:[0,0,2]},
    ];
    for (const [operation,volume] of [['union',15],['intersection',1],['difference',7]] as const) {
        const doc={language:'modelgraph/nurbs-1',units:'mm',nodes:[...nodes,{id:'result',op:'mesh_boolean',inputs:['a','b'],operation}],root:'result'};
        const result=await runOwnNurbs(doc,{action:'export',format:'stl'});
        expect(result.ok,JSON.stringify(result)).toBe(true);
        if (!result.ok) throw new Error('CSG failed');
        expect(result.report.mesh?.signedVolumeMm3).toBeCloseTo(volume,8);
        expect(result.report.definitions.patch).toBeTruthy();
        expect(result.report.mesh?.boolean?.operation).toBe(operation);
    }
    const empty=buildOwnNurbs({language:'modelgraph/nurbs-1',units:'mm',nodes:[...nodes.slice(0,3),{id:'empty',op:'mesh_boolean',inputs:['a','a'],operation:'difference'}],root:'empty'},{action:'build'});
    expect(empty.report.bounds).toBeNull();
    expect(empty.mesh?.indices).toEqual([]);
});
