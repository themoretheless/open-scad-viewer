import {createNativeGeometryArtifact} from '../core/nativeGeometry';
import {extrudePolygonProfile,revolvePolygonProfile,loftPolygonSections,sweepPolygonProfile,type PolygonProfile} from './geometry/polygon';
import {meshToNurbsBrep,meshToSdf,meshToSubdivision,meshToNurbs,tessellateNurbsPatches,type NurbsPatchSet} from './geometry/reconstruction';
import type {SubdivisionCage} from './geometry/subdivision';
import {tessellateSubdivision} from './geometry/subdivision';
import {tessellateSdfGpuAware as tessellateSdf,evaluateSdf,type SdfField} from './geometry/sdf';
import {transformNurbsBrep,createBrepSphere,createBrepTorus,createBrepBox,revolveBrepProfile,createBrepCylinder,createBrepFrustum,createBrepTube,extrudeBrepCurves,extrudeBrepPolygon,booleanNurbsBrep,chamferNurbsBrepEdges,filletNurbsBrepEdges,tessellateNurbsBrep,type NurbsBrep} from './geometry/brep';
import { inspectPolygonMesh,booleanPolygonMeshes } from './geometry/polygon';
import { exportMeshFormat, meshExportBase64, type MeshExportFormat } from './meshExportFormats';
import { compileModelGraphNurbs } from './modelGraphNurbs';
import { validateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot, elevateNurbsCurve, trimNurbsCurve, reverseNurbsCurve, nurbsCurveBounds, type NurbsCurve } from './nurbsCurve';
import { validateNurbsSurface, evaluateNurbsSurface, insertNurbsSurfaceKnot, elevateNurbsSurface, trimNurbsSurface, reverseNurbsSurface, isoNurbsCurve, nurbsSurfaceBounds, type NurbsSurface } from './nurbsSurface';
import {certifyNurbsCurveFoundation, certifyNurbsSurfaceFoundation} from './nurbsFoundation';
import { loftAlignedNurbsCurves,sweepNurbsCurve,loftNurbsCurves, extrudeNurbsCurve, revolveNurbsCurve } from './nurbsConstructors';
import { tessellateNurbsSurface, thickenNurbsMesh, exportNurbsStl } from './geometry/tessellation';
type Mesh = ReturnType<typeof tessellateNurbsSurface>;
type Value = {kind:'profile';data:PolygonProfile} | {kind:'patches';data:NurbsPatchSet} | {kind:'subdivision';data:SubdivisionCage} | {kind:'sdf';data:SdfField} | {kind:'brep';data:NurbsBrep} | {
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
    format?: 'json' | MeshExportFormat;
    /** Viewer-only derived display policy; does not change the authored graph. */
    display?: {segments:number;subdivisionLevels:number};
    evaluations?: Array<{
        node: string;
        u: number;
        v?: number;
    }>;
};
/** An sdf_tessellate job whose input subtree is pure SDF (no mesh inputs). */
export interface SdfTessellationJob {
    field: import('./geometry/sdf').SdfField
    grid: { min: number[]; max: number[]; cells: number[] }
}

/** Collects sdf_tessellate jobs resolvable without mesh evaluation, so the
 * caller can run their grid sampling on the GPU before the synchronous build. */
export function collectSdfJobs(document: unknown): SdfTessellationJob[] {
    type SdfField = import('./geometry/sdf').SdfField
    const compiled = compileModelGraphNurbs(document)
    const nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n] as const))
    const memo = new Map<string, SdfField | null>()
    const pure = (key: string): SdfField | null => {
        const hit = memo.get(key)
        if (hit !== undefined) return hit
        const n = nodes.get(key) as any
        const field: SdfField | null = !n ? null
            : n.op === 'sdf_sphere' ? { kind: 'sphere', center: n.center, radius: n.radius }
            : n.op === 'sdf_box' ? { kind: 'box', center: n.center, half_size: n.half_size }
            : n.op === 'sdf_torus' ? { kind: 'torus', center: n.center, major_radius: n.major_radius, minor_radius: n.minor_radius }
            : n.op === 'sdf_union' || n.op === 'sdf_intersection' || n.op === 'sdf_difference'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b
                    ? { kind: n.op === 'sdf_union' ? 'union' : n.op === 'sdf_intersection' ? 'intersection' : 'difference', a, b } : null })()
            : n.op === 'sdf_smooth_union'
                ? (() => { const a = pure(n.inputs[0]); const b = pure(n.inputs[1]); return a && b ? { kind: 'smooth_union', a, b, radius: n.radius } : null })()
            : n.op === 'sdf_offset' ? (() => { const input = pure(n.input); return input ? { kind: 'offset', input, distance: n.distance } : null })()
            : n.op === 'sdf_translate' ? (() => { const input = pure(n.input); return input ? { kind: 'translate', input, vector: n.vector } : null })()
            : null
        memo.set(key, field)
        return field
    }
    const jobs: SdfTessellationJob[] = []
    for (const n of compiled.resolved_document.nodes) {
        if ((n as any).op !== 'sdf_tessellate') continue
        const node = n as any
        const field = pure(node.input)
        if (field) jobs.push({ field, grid: { min: node.min, max: node.max, cells: node.cells } })
    }
    return jobs
}

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
    const needBrep=(key:string):NurbsBrep=>{const v=get(key);if(v.kind!=='brep')throw new Error('Expected a B-rep body');return v.data;};
    const needSdf=(key:string):SdfField=>{const v=get(key);if(v.kind!=='sdf')throw new Error('Expected SDF field');return v.data;};
    const reconstructionReports:Record<string,unknown>={};
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
                case 'polygon_profile': result={kind:'profile',data:{outer:n.outer,holes:n.holes}};break;
                case 'extrude': {
                    const v=get(n.input);const vector=[0,0,n.height];
                    if(v.kind==='curve')result={kind:'surface',data:extrudeNurbsCurve(v.data,vector)};
                    else if(v.kind==='profile')result={kind:'mesh',data:extrudePolygonProfile(v.data,vector)};
                    else throw new Error('Extrude expects a NURBS curve or polygon profile');break;
                }
                case 'revolve': {
                    const v=get(n.input);
                    if(v.kind==='curve')result={kind:'surface',data:revolveNurbsCurve(v.data,[0,0,0],[0,0,1],n.angle)};
                    else if(v.kind==='profile'){if(v.data.holes?.length)throw new Error('Revolve with profile holes is not supported');result={kind:'mesh',data:revolvePolygonProfile(v.data.outer,n.angle,n.segments)};}
                    else throw new Error('Revolve expects a NURBS curve or polygon profile');break;
                }
                case 'polygon_extrude': {const v=get(n.input);if(v.kind!=='profile')throw new Error('Expected polygon profile');result={kind:'mesh',data:extrudePolygonProfile(v.data,n.vector)};break;}
                case 'polygon_sweep': {const v=get(n.input);if(v.kind!=='profile')throw new Error('Expected polygon profile');if(v.data.holes?.length)throw new Error('Sweep with profile holes is not supported');result={kind:'mesh',data:sweepPolygonProfile(v.data.outer,n.path,n.up)};break;}
                case 'polygon_loft': result={kind:'mesh',data:loftPolygonSections(n.sections)};break;
                case 'surface_sweep': result={kind:'surface',data:sweepNurbsCurve(needCurve(n.inputs[0]),needCurve(n.inputs[1]))};break;
                case 'surface_loft': result={kind:'surface',data:loftAlignedNurbsCurves(n.inputs.map(needCurve))};break;
                case 'triangle_mesh': {
                    const mesh={positions:n.vertices.flat(),indices:n.triangles.flat()};
                    result={kind:'mesh',data:{...mesh,report:inspectPolygonMesh(mesh)}};break;
                }
                case 'mesh_to_nurbs_brep': result={kind:'brep',data:meshToNurbsBrep(needMesh(n.input))};break;
                case 'mesh_to_sdf': result={kind:'sdf',data:meshToSdf(needMesh(n.input))};break;
                case 'mesh_to_subdivision': {const fitted=meshToSubdivision(needMesh(n.input),n.iterations);const {cage,...report}=fitted;reconstructionReports[key]=report;result={kind:'subdivision',data:cage};break;}
                case 'subdivision_tessellate': {
                    const v=get(n.input);if(v.kind!=='subdivision')throw new Error('Expected subdivision cage');result={kind:'mesh',data:tessellateSubdivision(v.data,n.levels)};break;
                }
                case 'mesh_to_nurbs': result={kind:'patches',data:meshToNurbs(needMesh(n.input))};break;
                case 'mesh_fit_nurbs': result={kind:'patches',data:meshToNurbs(needMesh(n.input),'point_normal',n.max_deviation)};break;
                case 'nurbs_patches_tessellate': {
                    const v=get(n.input);if(v.kind!=='patches')throw new Error('Expected NURBS patch set');result={kind:'mesh',data:tessellateNurbsPatches(v.data,n.segments)};break;
                }
                case 'subdivision':
                    result={kind:'mesh',data:tessellateSubdivision({vertices:n.vertices,faces:n.faces},n.levels)};break;
                case 'sdf_sphere': result={kind:'sdf',data:{kind:'sphere',center:n.center,radius:n.radius}};break;
                case 'sdf_box': result={kind:'sdf',data:{kind:'box',center:n.center,half_size:n.half_size}};break;
                case 'sdf_torus': result={kind:'sdf',data:{kind:'torus',center:n.center,major_radius:n.major_radius,minor_radius:n.minor_radius}};break;
                case 'sdf_union': case 'sdf_intersection': case 'sdf_difference':
                    result={kind:'sdf',data:{kind:n.op==='sdf_union'?'union':n.op==='sdf_intersection'?'intersection':'difference',a:needSdf(n.inputs[0]),b:needSdf(n.inputs[1])}};break;
                case 'sdf_smooth_union': result={kind:'sdf',data:{kind:'smooth_union',a:needSdf(n.inputs[0]),b:needSdf(n.inputs[1]),radius:n.radius}};break;
                case 'sdf_offset': result={kind:'sdf',data:{kind:'offset',input:needSdf(n.input),distance:n.distance}};break;
                case 'sdf_translate': result={kind:'sdf',data:{kind:'translate',input:needSdf(n.input),vector:n.vector}};break;
                case 'sdf_tessellate': result={kind:'mesh',data:tessellateSdf(needSdf(n.input),{min:n.min,max:n.max,cells:n.cells})};break;
                case 'brep_box':
                    result={kind:'brep',data:createBrepBox(n.min,n.max)};
                    break;
                case 'brep_sphere': result={kind:'brep',data:createBrepSphere(n.radius)};break;
                case 'brep_torus': result={kind:'brep',data:createBrepTorus(n.major_radius,n.minor_radius)};break;
                case 'brep_cylinder': result={kind:'brep',data:createBrepCylinder(n.radius,n.height)};break;
                case 'brep_frustum': result={kind:'brep',data:createBrepFrustum(n.bottom_radius,n.top_radius,n.height)};break;
                case 'brep_tube': result={kind:'brep',data:createBrepTube(n.outer_radius,n.inner_radius,n.height)};break;
                case 'brep_revolve': {
                    const profile=get(n.input);if(profile.kind!=='profile'||profile.data.holes?.length)throw new Error('Exact revolve requires one polygon profile without holes');
                    result={kind:'brep',data:revolveBrepProfile(profile.data.outer as [number,number][],n.angle)};break;
                }
                case 'brep_extrude': {
                    const profile=get(n.input);if(profile.kind!=='profile')throw new Error('Expected a polygon profile');
                    result={kind:'brep',data:extrudeBrepPolygon(profile.data.outer as [number,number][],Math.min(0,n.height),Math.max(0,n.height),profile.data.holes as [number,number][][])};break;
                }
                case 'brep_extrude_curves':
                    result={kind:'brep',data:extrudeBrepCurves(n.loops.map(wire=>wire.map(needCurve)),n.z_min,n.z_max)};break;
                case 'brep_boolean': result={kind:'brep',data:booleanNurbsBrep(needBrep(n.inputs[0]),needBrep(n.inputs[1]),n.operation)};break;
                case 'brep_chamfer': result={kind:'brep',data:chamferNurbsBrepEdges(needBrep(n.input),n.edges,n.size)};break;
                case 'brep_fillet': result={kind:'brep',data:filletNurbsBrepEdges(needBrep(n.input),n.edges,n.radius,n.segments)};break;
                case 'brep_tessellate': {
                    const input=get(n.input);if(input.kind!=='brep')throw new Error('Expected a B-rep model');
                    result={kind:'mesh',data:tessellateNurbsBrep(input.data,n.segments)};
                    break;
                }
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
                    else if(v.kind==='brep')result={kind:'brep',data:transformNurbsBrep(v.data,m)};
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
                case 'mesh_boolean':
                    result = { kind: 'mesh', data: booleanPolygonMeshes(needMesh(n.inputs[0]), needMesh(n.inputs[1]), n.operation, n.relative_tolerance === undefined ? {} : { relativeTolerance: n.relative_tolerance }) };
                    break;
                case 'thicken':
                    result = { kind: 'mesh', data: thickenNurbsMesh(needMesh(n.input), n.vector) };
                    break;
            }
        }
        catch (error) {
            throw new Error(`Node ${key}: ${error instanceof Error ? error.message : 'NURBS operation failed.'}`);
        }
        if(result.kind==='sdf')evaluateSdf(result.data,[0,0,0]);
        if (result.kind === 'mesh' && (meshTriangles += result.data.indices.length / 3) > 60000)
            throw new Error('NURBS graph mesh work exceeds 60000 triangles.');
        cache.set(key, result);
        return result;
    }
    const root = get(compiled.document.root);
    const definitions = Object.fromEntries([...cache].filter(([, v]) => v.kind !== 'mesh').map(([id, v]) => [id, { kind: v.kind, ...v.data }]));
    const meshBounds = root.kind === 'mesh' && root.data.positions.length > 0 ? { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] } : null;
    if (root.kind === 'mesh' && meshBounds)
        for (let i = 0; i < root.data.positions.length; i++) {
            const axis = i % 3, v = root.data.positions[i];
            meshBounds.min[axis] = Math.min(meshBounds.min[axis], v);
            meshBounds.max[axis] = Math.max(meshBounds.max[axis], v);
        }
    const extendedGeometry=compiled.document.nodes.some(n=>n.op.startsWith('polygon_')||n.op==='subdivision'||n.op==='triangle_mesh'||n.op.startsWith('mesh_to_')||n.op==='mesh_fit_nurbs'||n.op.startsWith('sdf_'));
    const foundationCertificate = root.kind === 'curve' ? certifyNurbsCurveFoundation(root.data)
        : root.kind === 'surface' ? certifyNurbsSurfaceFoundation(root.data) : null;
    const report = { ...(Object.keys(reconstructionReports).length?{reconstruction:reconstructionReports}:{}), bounds_scope: foundationCertificate ? 'outward-rounded rational span hulls' : root.kind === 'mesh' ? 'derived mesh vertices' : 'conservative control hull', kernel: extendedGeometry?'own-rust-geometry':'own-rust-nurbs', polygon_core: 'own-rust-polygons', geometry_authority: extendedGeometry?'source geometry definitions':'rational control data', root: compiled.document.root, root_kind: root.kind, bounds: root.kind === 'curve' ? nurbsCurveBounds(root.data) : root.kind === 'surface' ? nurbsSurfaceBounds(root.data) : meshBounds, foundation_certificate: foundationCertificate, definitions, mesh: root.kind === 'mesh' ? root.data.report : null, printability: 'unknown', error_bound_certified: foundationCertificate !== null };
    const evaluations = (request.evaluations ?? []).map(q => { const v = get(q.node); if (v.kind === 'curve')
        return { node: q.node, ...evaluateNurbsCurve(v.data, q.u) }; if (v.kind === 'surface' && q.v !== undefined)
        return { node: q.node, ...evaluateNurbsSurface(v.data, q.u, q.v) }; throw new Error('Evaluation requires a curve or surface, and v for a surface.'); });
    const base = { ok: true, document_sha256: compiled.document_sha256, execution_target: 'own-nurbs', automatic_fallback: false, report, evaluations };
    if (request.action === 'export') {
        if (request.format === 'json')
            return { ...base, artifact: { format: 'json', mime_type: 'application/json', text: JSON.stringify(compiled.document, null, 2) } };
        if (!request.format || root.kind !== 'mesh')
            throw new Error('STL export requires a closed tessellated mesh. Export JSON to retain native NURBS definitions.');
        if (request.format !== 'stl') { const artifact = exportMeshFormat(root.data, request.format); return {...base, artifact: {format: request.format, mime_type: artifact.mimeType, extension: artifact.extension, base64: meshExportBase64(artifact.data)}}; }
        const text = exportNurbsStl(root.data);
        if (text.length > 4 * 1024 * 1024)
            throw new Error('STL export exceeds 4 MiB.');
        return { ...base, artifact: { format: 'stl', mime_type: 'model/stl', text } };
    }
    if(request.action==='build' && request.display) {
        const {segments,subdivisionLevels}=request.display;
        if(!Number.isInteger(segments)||segments<1||segments>32||!Number.isInteger(subdivisionLevels)||subdivisionLevels<0||subdivisionLevels>4)throw new Error('Invalid display tessellation policy');
        let mesh:Mesh|undefined=root.kind==='mesh'?root.data:undefined;
        if(root.kind==='surface')mesh=tessellateNurbsSurface(root.data,{segmentsU:segments,segmentsV:segments});
        if(root.kind==='brep')mesh=tessellateNurbsBrep(root.data,segments);
        if(root.kind==='subdivision')mesh=tessellateSubdivision(root.data,subdivisionLevels);
        if(root.kind==='patches')mesh=tessellateNurbsPatches(root.data,segments);
        let sourceNode=compiled.document.root,sourceValue=root;
        const rootNode=nodes.get(sourceNode)!;
        // Only known display conversions preserve source face correspondence.
        // Boolean/thicken/edit results own their mesh; do not invent CAD lineage.
        if(['tessellate','brep_tessellate','subdivision_tessellate','sdf_tessellate','nurbs_patches_tessellate'].includes(rootNode.op)&&'input' in rootNode) {
            sourceNode=rootNode.input;sourceValue=get(sourceNode);
        }
        if(rootNode.op==='subdivision')sourceValue={kind:'subdivision',data:{vertices:rootNode.vertices,faces:rootNode.faces}};
        const boundary=rootNode.op==='tessellate'&&(rootNode.trim||rootNode.trim_curves)?{trim:rootNode.trim??null,trimCurves:rootNode.trim_curves?{outer:needCurve(rootNode.trim_curves.outer),holes:rootNode.trim_curves.holes.map(needCurve)}:null}:null;
        const nativeGeometry=createNativeGeometryArtifact(sourceNode,sourceValue.kind,{geometry:sourceValue.data,boundary},compiled.document);
        return {...base,...(mesh?{mesh}:{}),nativeGeometry};
    }
    return { ...base, ...(request.action === 'build' && root.kind === 'mesh' ? { mesh: root.data } : {}) };
}
