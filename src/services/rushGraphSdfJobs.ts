import {compileRushGraphNurbs} from './rushGraphNurbsCompiler'

/** An sdf_tessellate job whose input subtree is pure SDF (no mesh inputs). */
export interface SdfTessellationJob {
    field: import('./geometry/sdf').SdfField
    grid: { min: number[]; max: number[]; cells: number[] }
}

/** Collects sdf_tessellate jobs resolvable without mesh evaluation, so the
 * caller can run their grid sampling on the GPU before the synchronous build. */
export function collectSdfJobs(document: unknown): SdfTessellationJob[] {
    type SdfField = import('./geometry/sdf').SdfField
    const compiled = compileRushGraphNurbs(document)
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

