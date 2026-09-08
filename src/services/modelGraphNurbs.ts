import { z } from 'zod/v4';
import { sha256Hex } from '../core/sha256';
const id = z.string().regex(/^[A-Za-z][A-Za-z0-9_]{0,31}$/);
const number = z.number().finite().min(-1e6).max(1e6);
const scalar = z.union([number, z.object({ param: id }).strict()]);
const vector = z.tuple([scalar, scalar, scalar]), uv = z.tuple([scalar, scalar]), interval = z.tuple([scalar, scalar]);
const knots = z.array(scalar).min(4).max(512), degree = z.number().int().min(1).max(25);
const insert = z.array(z.object({ value: scalar, count: z.number().int().min(1).max(25) }).strict()).max(32);
export const modelGraphNurbsSchema = z.object({
    language: z.literal('modelgraph/nurbs-1'), units: z.literal('mm'), parameters: z.array(z.object({ id, value: number }).strict()).max(64).default([]),
    nodes: z.array(z.discriminatedUnion('op', [
        z.object({id,op:z.literal('polygon_profile'),outer:z.array(uv).min(3).max(128),holes:z.array(z.array(uv).min(3).max(128)).max(16).default([])}).strict(),
        z.object({id,op:z.literal('extrude'),input:id,height:scalar}).strict(),
        z.object({id,op:z.literal('revolve'),input:id,angle:scalar,segments:z.number().int().min(3).max(64).default(32)}).strict(),
        z.object({id,op:z.literal('polygon_extrude'),input:id,vector}).strict(),
        z.object({id,op:z.literal('polygon_sweep'),input:id,path:z.array(vector).min(2).max(64),up:vector}).strict(),
        z.object({id,op:z.literal('polygon_loft'),sections:z.array(z.array(vector).min(3).max(128)).min(2).max(64)}).strict(),
        z.object({id,op:z.literal('surface_sweep'),inputs:z.tuple([id,id])}).strict(),
        z.object({id,op:z.literal('surface_loft'),inputs:z.array(id).min(2).max(32)}).strict(),
        z.object({id,op:z.literal('triangle_mesh'),vertices:z.array(vector).min(3).max(6144),triangles:z.array(z.tuple([z.number().int().min(0),z.number().int().min(0),z.number().int().min(0)])).min(1).max(2048)}).strict(),
        z.object({id,op:z.literal('mesh_to_nurbs_brep'),input:id}).strict(),
        z.object({id,op:z.literal('mesh_to_sdf'),input:id}).strict(),
        z.object({id,op:z.literal('mesh_to_subdivision'),input:id,iterations:z.number().int().min(0).max(32).default(8)}).strict(),
        z.object({id,op:z.literal('subdivision_tessellate'),input:id,levels:z.number().int().min(0).max(5).default(1)}).strict(),
        z.object({id,op:z.literal('mesh_to_nurbs'),input:id}).strict(),
        z.object({id,op:z.literal('mesh_fit_nurbs'),input:id,max_deviation:scalar}).strict(),
        z.object({id,op:z.literal('nurbs_patches_tessellate'),input:id,segments:z.number().int().min(1).max(16).default(2)}).strict(),
        z.object({id,op:z.literal('subdivision'),vertices:z.array(vector).min(3).max(4096),faces:z.array(z.array(z.number().int().min(0).max(4095)).min(3).max(64)).min(1).max(4096),levels:z.number().int().min(0).max(5).default(2)}).strict(),
        z.object({id,op:z.literal('sdf_sphere'),center:vector,radius:scalar}).strict(),
        z.object({id,op:z.literal('sdf_box'),center:vector,half_size:vector}).strict(),
        z.object({id,op:z.literal('sdf_torus'),center:vector,major_radius:scalar,minor_radius:scalar}).strict(),
        z.object({id,op:z.literal('sdf_union'),inputs:z.tuple([id,id])}).strict(),
        z.object({id,op:z.literal('sdf_intersection'),inputs:z.tuple([id,id])}).strict(),
        z.object({id,op:z.literal('sdf_difference'),inputs:z.tuple([id,id])}).strict(),
        z.object({id,op:z.literal('sdf_smooth_union'),inputs:z.tuple([id,id]),radius:scalar}).strict(),
        z.object({id,op:z.literal('sdf_offset'),input:id,distance:scalar}).strict(),
        z.object({id,op:z.literal('sdf_translate'),input:id,vector}).strict(),
        z.object({id,op:z.literal('sdf_tessellate'),input:id,min:vector,max:vector,cells:z.tuple([z.number().int().min(1).max(64),z.number().int().min(1).max(64),z.number().int().min(1).max(64)])}).strict(),
        z.object({id,op:z.literal('brep_box'),min:vector,max:vector}).strict(),
        z.object({id,op:z.literal('brep_tessellate'),input:id,segments:z.number().int().min(1).max(32).default(4)}).strict(),
        z.object({ id, op: z.literal('curve'), degree, knots, control_points: z.array(z.union([uv, vector])).min(2).max(256), weights: z.array(scalar).min(2).max(256), periodic: z.boolean().default(false) }).strict(),
        z.object({ id, op: z.literal('surface'), degree_u: degree, degree_v: degree, knots_u: knots, knots_v: knots, control_points: z.array(z.array(vector).min(2).max(32)).min(2).max(32), weights: z.array(z.array(scalar).min(2).max(32)).min(2).max(32), periodic_u: z.boolean().default(false), periodic_v: z.boolean().default(false) }).strict(),
        z.object({ id, op: z.literal('curve_edit'), input: id, insert_knots: insert.optional(), degree: degree.optional(), trim: interval.optional(), reverse: z.boolean().optional() }).strict(),
        z.object({ id, op: z.literal('surface_edit'), input: id, insert_knots_u: insert.optional(), insert_knots_v: insert.optional(), degree_u: degree.optional(), degree_v: degree.optional(), trim: z.tuple([scalar, scalar, scalar, scalar]).optional(), reverse_u: z.boolean().optional(), reverse_v: z.boolean().optional() }).strict(),
        z.object({ id, op: z.literal('iso_curve'), input: id, direction: z.enum(['u', 'v']), parameter: scalar }).strict(),
        z.object({ id, op: z.literal('ruled_surface'), inputs: z.array(id).min(2).max(32) }).strict(),
        z.object({ id, op: z.literal('surface_extrude'), input: id, vector }).strict(),
        z.object({ id, op: z.literal('surface_revolve'), input: id, origin: vector, axis: vector, angle: scalar }).strict(),
        z.object({ id, op: z.literal('transform'), input: id, matrix: z.array(z.array(scalar).length(4)).length(4) }).strict(),
        z.object({ id, op: z.literal('tessellate'), input: id, segments_u: z.number().int().min(1).max(64).default(16), segments_v: z.number().int().min(1).max(64).default(16), trim_segments: z.number().int().min(8).max(128).default(32), trim_curves: z.object({ outer: id, holes: z.array(id).max(8).default([]) }).strict().optional(), trim: z.object({ outer: z.array(uv).min(3).max(128), holes: z.array(z.array(uv).min(3).max(128)).max(8).default([]) }).strict().optional() }).strict(),
        z.object({ id, op: z.literal('mesh_boolean'), inputs: z.tuple([id, id]), operation: z.enum(['union', 'intersection', 'difference']), relative_tolerance: z.number().finite().min(1e-12).max(1e-5).optional() }).strict(),
        z.object({ id, op: z.literal('thicken'), input: id, vector }).strict(),
    ])).min(1).max(128), root: id,
}).strict();
export type ModelGraphNurbs = z.infer<typeof modelGraphNurbsSchema>;
type Resolved<T> = T extends {
    param: string;
} ? number : T extends object ? {
    [K in keyof T]: Resolved<T[K]>;
} : T;
export type ResolvedModelGraphNurbs = Resolved<ModelGraphNurbs>;
export class ModelGraphNurbsError extends Error {
    constructor(readonly code: string, readonly path: string, message: string) { super(message); this.name = 'ModelGraphNurbsError'; }
}
function fail(code: string, path: string, message: string): never { throw new ModelGraphNurbsError(code, path, message); }
export function compileModelGraphNurbs(input: unknown) {
    const raw = JSON.stringify(input);
    if (raw.length > 250000)
        fail('document_limit', '/', 'Document exceeds 250000 characters.');
    const document = modelGraphNurbsSchema.parse(input), params = new Map(document.parameters.map(p => [p.id, p.value]));
    if (params.size !== document.parameters.length)
        fail('duplicate_parameter', '/parameters', 'Duplicate parameter ID.');
    let visitedValues = 0;
    const resolve = (value: unknown, path: string): unknown => {
        if (++visitedValues > 30000)
            fail('value_limit', path, 'Maximum 30000 document values.');
        if (Array.isArray(value))
            return value.map((v, i) => resolve(v, `${path}/${i}`));
        if (value && typeof value === 'object') {
            const obj = value as Record<string, unknown>;
            if ('param' in obj) {
                const v = params.get(String(obj.param));
                if (v === undefined)
                    fail('unknown_parameter', path, 'Unknown parameter ' + String(obj.param));
                return v;
            }
            return Object.fromEntries(Object.entries(obj).map(([k, v]) => [k, resolve(v, `${path}/${k}`)]));
        }
        return value;
    };
    const resolved_document = resolve(document, '') as ResolvedModelGraphNurbs;
    const nodes = new Map(resolved_document.nodes.map(n => [n.id, n]));
    if (nodes.size !== resolved_document.nodes.length)
        fail('duplicate_node', '/nodes', 'Duplicate node ID.');
    const reached = new Set<string>(), active = new Set<string>(), heights = new Map<string, number>();
    const visit = (key: string, depth: number): number => {
        const n = nodes.get(key);
        if (!n)
            fail('unknown_node', '/nodes', 'Unknown node ' + key);
        if (active.has(key))
            fail('cycle', `/nodes/${key}`, 'Cyclic NURBS graph.');
        const cached = heights.get(key);
        if (cached !== undefined) {
            if (depth + cached - 1 > 32)
                fail('depth_limit', `/nodes/${key}`, 'Maximum graph depth 32.');
            return cached;
        }
        if (depth > 32)
            fail('depth_limit', `/nodes/${key}`, 'Maximum graph depth 32.');
        active.add(key);
        const refs = 'inputs' in n ? [...n.inputs] : 'input' in n ? [n.input] : [];
        if (n.op === 'tessellate' && n.trim_curves)
            refs.push(n.trim_curves.outer, ...n.trim_curves.holes);
        const height = 1 + Math.max(0, ...refs.map(r => visit(r, depth + 1)));
        active.delete(key);
        reached.add(key);
        heights.set(key, height);
        return height;
    };
    visit(document.root, 1);
    if (reached.size !== nodes.size)
        fail('unreachable_node', '/nodes', 'Every node must be reachable from root.');
    const canonical = (v: unknown): string => Array.isArray(v) ? `[${v.map(canonical).join(',')}]` : v && typeof v === 'object' ? `{${Object.entries(v).sort(([a], [b]) => a.localeCompare(b)).map(([k, x]) => JSON.stringify(k) + ':' + canonical(x)).join(',')}}` : JSON.stringify(v);
    return { document, resolved_document, document_sha256: sha256Hex(canonical(document)), execution_target: 'own-nurbs' as const };
}
export const MODELGRAPH_NURBS_GUIDE = `ModelGraph NURBS uses our own Rust numerical kernel, with no third-party spline or B-rep kernel. Select language:"modelgraph/nurbs-1",units:"mm". Scalars are finite numbers or {param:"id"}; define parameters:[{id,value}],nodes and root. Read the accompanying schema. Graph IDs are unique, all nodes reachable, cycles forbidden. Maximum128nodes, depth32 and30000values. This contract uses own Rust geometry and the shared brep-topology library, without routing into Manifold.
curve nodes specify degree, expanded knots, control_points (all2D orall3D), positive weights and periodic. surface nodes specify degree_u/v, knots_u/v, control_points[u][v][xyz], weights[u][v], periodic_u/v. Expanded knots have controlCount+degree+1 entries, finite nondecreasing values and active domain[knots[degree],knots[controlCount]]. Periodic data uses explicitly wrapped control points and extended knots, not an implicit one-period kernel-specific convention. Evaluation is homogeneous rational B-spline evaluation with first and second derivatives. At insufficient-continuity knots derivatives can be unavailable. Degenerate surface normals/curvatures are null; inspect derivative_status.
curve_edit supports insert_knots:[{value,count}], degree elevation, trim:[a,b] and reverse, in that order. surface_edit supports the same peraxis and trim:[uMin,uMax,vMin,vMax]. No degree reduction, knot removal or fitted approximation is implied. Elevation uses rational Bezier pieces and may increase interior knot multiplicities while preserving the curve image; reduced encoded continuity can make knot derivatives unavailable even for a smooth image. Insertion, trimming and elevation of periodic data operate on the active period and return a clamped nonperiodic representation; reversing preserves wrapped periodic storage. To split, create two trimmed references to one curve. iso_curve fixes the named u or v parameter. transform accepts a finite affine4x4 matrix and transforms control points; no perspective matrices.
polygon_profile stores a planar outer ring and holes. extrude dispatches profiles to native polygon extrusion and curves to NURBS surface extrusion; revolve dispatches similarly (polygon profiles with holes are unsupported). polygon_extrude accepts a vector; polygon_sweep uses a polyline path and initial up vector; polygon_loft joins corresponding planar sections with caps. surface_sweep constructs an exact translational rational sweep with fixed profile orientation. surface_loft aligns curve degrees, domains and knots before a piecewise-linear loft.
ruled_surface joins compatible3D curves with matching degree, knots and control count. This is a rational ruled construction, not automatic interpolation through arbitrary incompatible sections. surface_extrude translates a3D curve into a rational surface. surface_revolve uses exact rational quadratic arc representations for a nonzero angle within +/-360degrees about origin/axis; degree limits still apply.
tessellate converts a surface into a derived mesh with explicit segments_u/v and optional polygonal UV trim{outer,holes}. Trims may be concave but must be simple and nonintersecting. Alternatively trim_curves:{outer:curveId,holes:[curveId,...]} samples closed2D NURBS in the surface UV parameter space (dimensionless), using trim_segments perloop (8..128, default32). These retained rational trim definitions produce polygonal boundary approximations; neither their distance error nor topology correspondence is certified. Use only one of trim and trim_curves. Sampling reports are not certified global error bounds. thicken applies a fixed vector to a mesh, builds boundary walls and checks edge topology; this is not a general normal-offset operation. Curves/surfaces remain the authoritative definitions in the document. Build reports include evaluated definitions, bounds, mesh topology and preview images when available; evaluate returns numerical jets. STL (ASCII or stl_binary), 3MF and AMF export require a closed, consistently oriented mesh; OBJ, PLY and OFF also support open surfaces; self-intersections, wall thickness and printer suitability are not certified. JSON export retains the native NURBS document. Mesh formats: stl, stl_binary, 3mf, obj, ply, off, amf. Printing files contain geometry in millimeters; they do not include slicer profiles or textures. Exports are limited to 4 MiB and 100000 triangles. mesh_boolean takes inputs:[a,b] and operation:union|intersection|difference (a minus b), with optional relative_tolerance (1e-12..1e-5, default1e-9). It runs bounded numerical BSP CSG in the own Rust polygon kernel on closed oriented solids; invalid, ambiguous or non-manifold geometry and exhausted budgets return errors. Limits:10000 combined input triangles,20000 output triangles,8000000 work units. UV coordinates are discarded. Empty results have null bounds. NURBS definitions remain retained; mesh CSG does not reconstruct NURBS surfaces. Explicit reverse conversion: triangle_mesh(vertices,triangles) accepts indexed input. mesh_to_sdf() builds signed triangle distance for a closed oriented mesh; sdf_offset and sdf_tessellate work on the resulting field. mesh_to_subdivision(iterations:0..32) retains source topology and fits original-vertex samples at one refinement step; subdivision_tessellate(levels) emits its mesh. mesh_to_nurbs() creates exact faceted NURBS patches; mesh_fit_nurbs(max_deviation) creates approximate cubic point-normal patches, checking deviation on 9x9 samples per patch, not a certified bound. nurbs_patches_tessellate(segments) emits their mesh. mesh_to_nurbs_brep() builds exact planar trimmed NURBS topology, limited to 256 triangles, and works with brep_tessellate. These are explicit reconstruction choices, not recovery of unknown original CAD or coarse subdivision controls. Mesh sources are bounded; signed distance sources at most4096 triangles, NURBS/subdivision at most2048, sampled reconstruction work8million. Subdivision: subdivision(vertices,faces,levels) uniformly refines a convex polygon cage using own Rust Catmull-Clark (levels 0..5), emits a mesh and retains original face IDs. This is finite refinement, not exact limit evaluation; crease weights and adaptive subdivision are absent. Implicit fields: sdf_sphere(center,radius), sdf_box(center,half_size), sdf_torus(center,major_radius,minor_radius); sdf_union(a,b), sdf_intersection(a,b), sdf_difference(a,b), sdf_smooth_union(a,b,radius:3mm); pipe fields through sdf_offset(distance), sdf_translate(vector), sdf_tessellate(min,max,cells). Cells is a 3-vector of integers 1..64. Bounds must enclose the surface with positive boundary samples. CSG fields are not generally exact signed distances; extraction can miss sub-cell features. Field trees are bounded to 256 nodes/depth32. Both libraries exchange meshes with polygon-kernel; mesh CSG does not reconstruct control cages or implicit fields. brep_box creates a topological NURBS body from min/max vectors; brep_tessellate samples it with segments:1..32 and preserves faceIds. Shared B-rep topology includes vertices, edges, oriented coedges, loops, faces, shells and bodies. Geometry agreement is sampled, not certified; shell containment and self-intersections are not certified. General B-rep booleans, exact surface/surface trimming, STEP, fillets and offset-shell construction are not implemented.`;
export const MODELGRAPH_NURBS_EXAMPLE: ModelGraphNurbs = { language: 'modelgraph/nurbs-1', units: 'mm', parameters: [], nodes: [{ id: 'arc', op: 'curve', degree: 2, knots: [0, 0, 0, 1, 1, 1], control_points: [[10, 0, 0], [10, 10, 0], [0, 10, 0]], weights: [1, Math.SQRT1_2, 1], periodic: false }], root: 'arc' };
export const MODELGRAPH_NURBS_SURFACE_EXAMPLE: ModelGraphNurbs = { language: 'modelgraph/nurbs-1', units: 'mm', parameters: [{ id: 'height', value: 8 }], nodes: [{ id: 'patch', op: 'surface', degree_u: 2, degree_v: 2, knots_u: [0, 0, 0, 1, 1, 1], knots_v: [0, 0, 0, 1, 1, 1], control_points: [[[0, 0, 0], [0, 10, 0], [0, 20, 0]], [[10, 0, 0], [10, 10, { param: 'height' }], [10, 20, 0]], [[20, 0, 0], [20, 10, 0], [20, 20, 0]]], weights: [[1, 1, 1], [1, 2, 1], [1, 1, 1]], periodic_u: false, periodic_v: false }, { id: 'skin', op: 'tessellate', input: 'patch', segments_u: 16, segments_v: 16, trim_segments: 32 }, { id: 'plate', op: 'thicken', input: 'skin', vector: [0, 0, -2] }], root: 'plate' };
