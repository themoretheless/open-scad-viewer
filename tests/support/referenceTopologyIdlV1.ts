/**
 * Independent G0.6 topology IDL checker helpers.
 * Imports no production brep-topology, Worker protocol, or lineage modules.
 */

export type Json =
  | null
  | boolean
  | number
  | string
  | Json[]
  | { readonly [key: string]: Json }

export type Snapshot = {
  readonly schema: string
  readonly schemaVersion: number
  readonly profile: string
  readonly vertices: readonly { readonly point: readonly number[] }[]
  readonly edges: readonly {
    readonly vertices: readonly [number, number]
    readonly geom: string
  }[]
  readonly faces: readonly {
    readonly loops: readonly {
      readonly coedges: readonly {
        readonly edge: number
        readonly sense: string
      }[]
    }[]
  }[]
  readonly shells: readonly {
    readonly faceUses: readonly { readonly face: number; readonly sense: string }[]
    readonly role?: unknown
  }[]
  readonly solids: readonly {
    readonly shellUses: readonly {
      readonly shell: number
      readonly role: string
    }[]
  }[]
  readonly topoIds: {
    readonly solid: string
    readonly shell: string
    readonly faces: readonly string[]
    readonly edges: readonly string[]
    readonly vertices: readonly string[]
  }
}

const TOPO_ID = /^[0-9a-f]{32}$/

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function applyPatch(
  doc: Json,
  patch: {
    readonly op: string
    readonly path: string
    readonly value?: Json
  },
): Json {
  const root = cloneJson(doc) as { [key: string]: Json } | Json[]
  const parts = patch.path
    .split('/')
    .slice(1)
    .map((p) => p.replaceAll('~1', '/').replaceAll('~0', '~'))
  let parent: Json = root
  for (let i = 0; i < parts.length - 1; i += 1) {
    const key = parts[i]!
    parent = Array.isArray(parent)
      ? parent[Number(key)]!
      : (parent as { [key: string]: Json })[key]!
  }
  const last = parts[parts.length - 1]!
  if (patch.op === 'add' || patch.op === 'replace') {
    if (Array.isArray(parent)) parent[Number(last)] = patch.value as Json
    else (parent as { [key: string]: Json })[last] = patch.value as Json
  } else if (patch.op === 'remove') {
    if (Array.isArray(parent)) parent.splice(Number(last), 1)
    else delete (parent as { [key: string]: Json })[last]
  } else {
    throw new Error(`unsupported patch op ${patch.op}`)
  }
  return root as Json
}

export function schemaReject(snapshot: Snapshot): string | null {
  if (snapshot.schema !== 'open-scad-viewer/brep-topology-idl') return 'schema'
  if (snapshot.schemaVersion !== 1) return 'schemaVersion'
  if (snapshot.profile !== 'SolidManifold') return 'profile'
  for (const shell of snapshot.shells) {
    if ('role' in shell && shell.role !== undefined) return 'shell-role-forbidden'
  }
  for (const solid of snapshot.solids) {
    for (const use of solid.shellUses) {
      if (use.role !== 'Outer' && use.role !== 'Cavity') return 'shell-use-role'
    }
  }
  const ids = [
    snapshot.topoIds.solid,
    snapshot.topoIds.shell,
    ...snapshot.topoIds.faces,
    ...snapshot.topoIds.edges,
    ...snapshot.topoIds.vertices,
  ]
  for (const id of ids) {
    if (!TOPO_ID.test(id)) return 'topo-id'
  }
  for (const vertex of snapshot.vertices) {
    if (vertex && typeof vertex === 'object' && 'arenaKey' in (vertex as object)) {
      return 'arena-key-forbidden'
    }
  }
  return null
}

export function localValidatorReject(snapshot: Snapshot): string | null {
  const useCount = new Map<number, number>()
  for (const face of snapshot.faces) {
    for (const loop of face.loops) {
      for (const coedge of loop.coedges) {
        useCount.set(coedge.edge, (useCount.get(coedge.edge) ?? 0) + 1)
      }
    }
  }
  for (let edge = 0; edge < snapshot.edges.length; edge += 1) {
    if ((useCount.get(edge) ?? 0) !== 2) return `edge-${edge}-use-count`
  }
  return null
}
