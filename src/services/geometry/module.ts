/** Legacy-call adapter to our handle-based Rust CAD kernel. No foreign CAD runtime. */
import { callGeometryRust, withCadMesh, importCadMesh } from './kernel';
export type Vec2 = [number, number];
export type Vec3 = [number, number, number];
export type Mat3 = [number, number, number, number, number, number, number, number, number];
export type Mat4 = [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
export type Polygons = Vec2[][] | Vec2[];
export type ErrorStatus = string;
type Inspection = {
    empty: boolean;
    volume: number;
    area: number;
    min: number[];
    max: number[];
    report?: {
        closed: boolean;
    };
};
const call = <T>(action: string, args: object = {}) => callGeometryRust<T>('cad', { action, ...args });
const identity = () => [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]];
let original = 0;
class Handle {
    private deleted = false;
    protected cached?: Inspection;
    constructor(readonly handle: number) { }
    protected inspect() { if (this.deleted)
        throw new Error('Deleted own CAD handle'); return this.cached ??= call<Inspection>('inspect', { id: this.handle }); }
    isEmpty() { return this.inspect().empty; }
    isDeleted() { return this.deleted; }
    delete() { if (this.deleted)
        return; call('delete', { ids: [this.handle] }); this.deleted = true; }
}
export class Mesh {
    numProp: number;
    vertProperties: Float32Array;
    triVerts: Uint32Array;
    mergeFromVert = new Uint32Array();
    mergeToVert = new Uint32Array();
    runIndex = new Uint32Array();
    runOriginalID = new Uint32Array();
    runFlags = new Uint8Array();
    faceID = new Uint32Array();
    constructor(v: {
        numProp: number;
        vertProperties: Float32Array;
        triVerts: Uint32Array;
        [key: string]: unknown;
    }) { this.numProp = v.numProp; this.vertProperties = v.vertProperties; this.triVerts = v.triVerts; }
    get numTri() { return this.triVerts.length / 3; }
    get numVert() { return this.vertProperties.length / this.numProp; }
    merge() { return false; }
    position(index: number): Vec3 { return Array.from(this.vertProperties.slice(index * this.numProp, index * this.numProp + 3)) as Vec3; }
}
export class Manifold extends Handle {
    private normals = false;
    private normalAngle = 52.5;
    private invalidImport = false;
    private original = ++original;
    constructor(value: number | Mesh) { super(typeof value === 'number' ? value : importCadMesh(value.numProp, value.vertProperties, value.triVerts)); if (typeof value !== 'number' && value.triVerts.length > 0 && this.isEmpty())
        this.invalidImport = true; }
    static cube(size: Vec3 | number, center = false) { return new Manifold(call('cube', { size: typeof size === 'number' ? [size, size, size] : size, center })); }
    static sphere(radius: number, segments = 32) { return new Manifold(call('sphere', { radius, segments })); }
    static cylinder(height: number, bottom: number, top = bottom, segments = 32, center = false) { return new Manifold(call('cylinder', { height, bottom, top, segments, center })); }
    static ofMesh(mesh: Mesh) { const m = new this(mesh); if (m.status() !== 'NoError') {
        m.delete();
        throw new Error('Not manifold');
    } return m; }
    static union(values: Manifold[]) { return this.combine(values, 'union'); }
    static intersection(values: Manifold[]) { return this.combine(values, 'intersection'); }
    static difference(values: Manifold[]) { return this.combine(values, 'difference'); }
    static compose(values: Manifold[]) { return this.combine(values, 'compose'); }
    private static combine(values: Manifold[], operation: string) { return new Manifold(call('combine', { ids: values.map(v => v.handle), dimension: 3, operation })); }
    static hull(values: Manifold[]) { return new Manifold(call('hull', { ids: values.map(v => v.handle), dimension: 3 })); }
    static extrude(section: CrossSection, height: number, slices = 0, twist = 0, scale: Vec2 = [1, 1], center = false) { return new Manifold(call('extrude', { id: section.handle, height, slices: slices + 1, twist, scale, center })); }
    static revolve(section: CrossSection, segments = 32, angle = 360) { return new Manifold(call('revolve', { id: section.handle, segments, angle })); }
    add(b: Manifold) { return Manifold.union([this, b]); }
    subtract(b: Manifold) { return Manifold.difference([this, b]); }
    intersect(b: Manifold) { return Manifold.intersection([this, b]); }
    private matrix(matrix: number[][]) { const result = new Manifold(call('transform', { id: this.handle, matrix })); result.original = this.original; return result; }
    transform(m: Mat4) { const rows = Array.from({ length: 4 }, (_, i) => Array.from({ length: 4 }, (_, j) => m[j * 4 + i])); rows[3] = [0, 0, 0, 1]; return this.matrix(rows); }
    translate(v: Vec3) { const m = identity(); for (let i = 0; i < 3; i++)
        m[i]![3] = v[i]!; return this.matrix(m); }
    scale(v: Vec3 | number) { const values = typeof v === 'number' ? [v, v, v] : v; const m = identity(); for (let i = 0; i < 3; i++)
        m[i]![i] = values[i]!; return this.matrix(m); }
    rotate(v: Vec3) { const [x, y, z] = v.map(a => a * Math.PI / 180); const [a, b, c, d, e, f] = [Math.cos(x!), Math.sin(x!), Math.cos(y!), Math.sin(y!), Math.cos(z!), Math.sin(z!)]; return this.matrix([[e * c, e * d * b - f * a, e * d * a + f * b, 0], [f * c, f * d * b + e * a, f * d * a - e * b, 0], [-d, c * b, c * a, 0], [0, 0, 0, 1]]); }
    mirror(v: Vec3) { const n = Math.hypot(...v); if (!n)
        return Manifold.union([]); const u = v.map(x => x / n); const m = identity(); for (let i = 0; i < 3; i++)
        for (let j = 0; j < 3; j++)
            m[i]![j] -= 2 * u[i]! * u[j]!; return this.matrix(m); }
    originalID() { return this.original; }
    asOriginal() { const m = new Manifold(call('copy', { id: this.handle })); return m; }
    status(): ErrorStatus { return !this.invalidImport && (this.isEmpty() || this.inspect().report?.closed) ? 'NoError' : 'NotManifold'; }
    volume() { return Math.abs(this.inspect().volume); }
    surfaceArea() { return this.inspect().area; }
    boundingBox() { const r = this.inspect(); return { min: r.min as Vec3, max: r.max as Vec3 }; }
    calculateNormals(_index = 0, _angle = 52.5) { const m = new Manifold(call('copy', { id: this.handle })); m.original = this.original; m.normals = true; m.normalAngle = _angle; return m; }
    getMesh() {
        return withCadMesh(this.handle, raw => {
        let mesh: Mesh;
        if (this.normals) {
            const normals: number[][] = [], adjacent: number[][] = Array.from({ length: raw.positions.length / 3 }, () => []);
            for (let t = 0; t < raw.indices.length; t += 3) {
                const p = Array.from(raw.indices.subarray(t, t + 3), i => raw.positions.subarray(i * 3, i * 3 + 3));
                const a = p[1]!.map((v, k) => v - p[0]![k]!), b = p[2]!.map((v, k) => v - p[0]![k]!);
                const n = [a[1]! * b[2]! - a[2]! * b[1]!, a[2]! * b[0]! - a[0]! * b[2]!, a[0]! * b[1]! - a[1]! * b[0]!];
                const length = Math.hypot(...n);
                normals.push(n.map(v => length ? v / length : 0));
                for (const i of raw.indices.subarray(t, t + 3))
                    adjacent[i]!.push(t / 3);
            }
            const vertices: number[] = [], indices: number[] = [], rawIds: number[] = [], unique = new Map<string, number>();
            const cosine = Math.cos(this.normalAngle * Math.PI / 180);
            for (let i = 0; i < raw.indices.length; i++) {
                const id = raw.indices[i]!, face = normals[Math.floor(i / 3)]!, normal = [0, 0, 0];
                for (const t of adjacent[id]!) {
                    const n = normals[t]!;
                    if (n.reduce((s, v, k) => s + v * face[k]!, 0) >= cosine - 1e-10)
                        for (let k = 0; k < 3; k++)
                            normal[k]! += n[k]!;
                }
                const length = Math.hypot(...normal);
                for (let k = 0; k < 3; k++)
                    normal[k] = length ? normal[k]! / length : 0;
                const key = id + ':' + normal.map(v => Math.round(v * 1e7)).join(',');
                let index = unique.get(key);
                if (index === undefined) {
                    index = vertices.length / 6;
                    unique.set(key, index);
                    vertices.push(...raw.positions.subarray(id * 3, id * 3 + 3), ...normal);
                    rawIds.push(id);
                }
                indices.push(index);
            }
            mesh = new Mesh({ numProp: 6, vertProperties: new Float32Array(vertices), triVerts: new Uint32Array(indices) });
            const first = new Map<number, number>(), from: number[] = [], to: number[] = [];
            rawIds.forEach((id, i) => { const previous = first.get(id); if (previous === undefined)
                first.set(id, i);
            else {
                from.push(i);
                to.push(previous);
            } });
            mesh.mergeFromVert = new Uint32Array(from);
            mesh.mergeToVert = new Uint32Array(to);
        }
        else
            mesh = new Mesh({ numProp: 3, vertProperties: new Float32Array(raw.positions), triVerts: new Uint32Array(raw.indices) });
        mesh.runIndex = new Uint32Array([0, mesh.triVerts.length]);
        mesh.runOriginalID = new Uint32Array([this.original]);
        mesh.runFlags = new Uint8Array([0]);
        mesh.faceID = new Uint32Array(raw.faceIds);
        return mesh;
        });
    }
    project(): CrossSection { return new CrossSection(call<number>('project', { id: this.handle })); }
    slice(height: number): CrossSection { return new CrossSection(call<number>('slice', { id: this.handle, height })); }
    minkowskiSum(other: Manifold): Manifold { return new Manifold(call('minkowski', { id: this.handle, other: other.handle })); }
    splitByPlane(normal: Vec3, offset = 0): Manifold[] { return call<number[]>('split', { id: this.handle, normal, offset }).map(id => new Manifold(id)); }
}
export class CrossSection extends Handle {
    constructor(value: number | Polygons, fill = 'Positive') { super(typeof value === 'number' ? value : call<number>('profile', { rings: typeof value[0]?.[0] === 'number' ? [value] : value, fill })); }
    static square(size: Vec2, center = false) { const [x, y] = size; if (x === 0 || y === 0)
        return new CrossSection([]); const a = center ? -x / 2 : 0, b = center ? -y / 2 : 0; return new CrossSection([[[a, b], [a + x, b], [a + x, b + y], [a, b + y]]]); }
    static circle(radius: number, segments = 32) { return new CrossSection([Array.from({ length: segments }, (_, i) => [radius * Math.cos(i * 2 * Math.PI / segments), radius * Math.sin(i * 2 * Math.PI / segments)] as Vec2)]); }
    static ofPolygons(p: Polygons, fill = 'Positive') { if (!p.length || !p[0]?.length)
        throw new TypeError("Cannot read properties of undefined (reading 'length')"); return new CrossSection(p, fill); }
    private static combine(values: CrossSection[], operation: string) { return new CrossSection(call<number>('combine', { ids: values.map(v => v.handle), dimension: 2, operation })); }
    static union(values: CrossSection[]) { return this.combine(values, 'union'); }
    static intersection(values: CrossSection[]) { return this.combine(values, 'intersection'); }
    static difference(values: CrossSection[]) { return this.combine(values, 'difference'); }
    static xor(values: CrossSection[]) { return this.combine(values, 'xor'); }
    static exclude(values: CrossSection[]) { return this.xor(values); }
    static hull(values: CrossSection[]) { return new CrossSection(call<number>('hull', { ids: values.map(v => v.handle), dimension: 2 })); }
    static divide(values: CrossSection[]) { return new CrossSection(call<number>('divide', { ids: values.map(v => v.handle) })); }
    static crop(values: CrossSection[]) { return call<number[]>('crop', { ids: values.map(v => v.handle) }).map(id => new CrossSection(id)); }
    static trim(values: CrossSection[]) { return call<number[]>('trim', { ids: values.map(v => v.handle) }).map(id => new CrossSection(id)); }
    static minusFront(values: CrossSection[]) { return new CrossSection(call<number>('minus_front', { ids: values.map(v => v.handle) })); }
    static minusBack(values: CrossSection[]) { return new CrossSection(call<number>('minus_back', { ids: values.map(v => v.handle) })); }
    static shapeBuilderExtract(values: CrossSection[], point: Vec2) { return new CrossSection(call<number>('shape_builder_extract', { ids: values.map(v => v.handle), point })); }
    static shapeBuilderDelete(values: CrossSection[], point: Vec2) { return new CrossSection(call<number>('shape_builder_delete', { ids: values.map(v => v.handle), point })); }
    static makeCompound(values: CrossSection[]) { return call<number[]>('make_compound', { ids: values.map(v => v.handle) }).map(id => new CrossSection(id)); }
    add(b: CrossSection) { return CrossSection.union([this, b]); }
    subtract(b: CrossSection) { return CrossSection.difference([this, b]); }
    intersect(b: CrossSection) { return CrossSection.intersection([this, b]); }
    private matrix(matrix: number[][]) { return new CrossSection(call<number>('transform', { id: this.handle, matrix })); }
    transform(v: Mat3) { return this.matrix([[v[0], v[3], 0, v[6]], [v[1], v[4], 0, v[7]], [0, 0, 1, 0], [0, 0, 0, 1]]); }
    translate(v: readonly [
        number,
        number
    ]) { const m = identity(); m[0]![3] = v[0]; m[1]![3] = v[1]; return this.matrix(m); }
    mirror(v: Vec2) { const n = Math.hypot(...v); if (!n)
        return new CrossSection([]); const x = v[0] / n, y = v[1] / n; return this.matrix([[1 - 2 * x * x, -2 * x * y, 0, 0], [-2 * x * y, 1 - 2 * y * y, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]); }
    scale(v: Vec2 | number) { const u = typeof v === 'number' ? [v, v] : v; const m = identity(); m[0]![0] = u[0]!; m[1]![1] = u[1]!; return this.matrix(m); }
    rotate(angle: number) { const a = angle * Math.PI / 180; return this.matrix([[Math.cos(a), -Math.sin(a), 0, 0], [Math.sin(a), Math.cos(a), 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]); }
    offset(distance: number, join = 'Round', _miter = 2, segments = 8) { return new CrossSection(call<number>('offset', { id: this.handle, distance, join, segments })); }
    extrude(height: number, slices = 0, twist = 0, scale: Vec2 = [1, 1], center = false) { return Manifold.extrude(this, height, slices, twist, scale, center); }
    revolve(segments = 32, angle = 360) { return Manifold.revolve(this, segments, angle); }
    decompose(): CrossSection[] { return call<number[]>('decompose', { id: this.handle }).map(id => new CrossSection(id)); }
    toPolygons() { return call<Vec2[][]>('polygons', { id: this.handle }); }
    area() { return this.inspect().area; }
    bounds() { const r = this.inspect(); return { min: r.min as Vec2, max: r.max as Vec2 }; }
}
export class ManifoldError extends Error {
    constructor(code: string) { super(code === 'NotManifold' ? 'Not manifold' : code); this.name = 'OwnCadError'; }
}
export type ManifoldToplevel = {
    ManifoldError: typeof ManifoldError;
    Manifold: typeof Manifold;
    CrossSection: typeof CrossSection;
    Mesh: typeof Mesh;
    setup(): void;
};
export default async function Module(): Promise<ManifoldToplevel> {
    // Each session family gets its own prototypes. Instrumentation must never wrap
    // global base classes and accidentally assign another runtime's allocations.
    class Solid extends Manifold {
    }
    ;
    class Section extends CrossSection {
    }
    const localize = (value: unknown): unknown => {
        if (value instanceof Manifold)
            Object.setPrototypeOf(value, Solid.prototype);
        else if (value instanceof CrossSection)
            Object.setPrototypeOf(value, Section.prototype);
        else if (Array.isArray(value))
            return value.map(localize);
        return value;
    };
    for (const [base, local] of [[Manifold, Solid], [CrossSection, Section]] as const) {
        for (const name of Object.getOwnPropertyNames(base)) {
            const descriptor = Object.getOwnPropertyDescriptor(base, name)!;
            if (typeof descriptor.value === 'function')
                Object.defineProperty(local, name, { ...descriptor, value: function (...args: unknown[]) { return localize(Reflect.apply(descriptor.value, this, args)); } });
        }
        for (const name of Object.getOwnPropertyNames(base.prototype)) {
            if (name === 'constructor')
                continue;
            const descriptor = Object.getOwnPropertyDescriptor(base.prototype, name)!;
            if (typeof descriptor.value === 'function')
                Object.defineProperty(local.prototype, name, { ...descriptor, value: function (...args: unknown[]) { return localize(Reflect.apply(descriptor.value, this, args)); } });
        }
    }
    return { Manifold: Solid, CrossSection: Section, Mesh, ManifoldError, setup() { } };
}
