function addAdjacent(adjacent: Map<number, number[]>, v: number, edgeIndex: number): void {
  const list = adjacent.get(v)
  if (list) list.push(edgeIndex)
  else adjacent.set(v, [edgeIndex])
}

function extendEndpoint(
  adjacent: Map<number, number[]>,
  selected: Set<number>,
  pending: number[],
  v: number,
): void {
  const list = adjacent.get(v)
  if (!list || list.length !== 2) return
  for (let k = 0; k < 2; k++) {
    const idx = list[k]
    if (!selected.has(idx)) {
      selected.add(idx)
      pending.push(idx)
    }
  }
}

/** Extend through unbranched topological vertices; never guess a turn at a junction. */
export function connectedEdgeChain(edges: readonly { a: number; b: number }[], seeds: readonly number[]): number[] {
  const selected = new Set<number>()
  for (let i = 0; i < seeds.length; i++) {
    const s = seeds[i]
    if (Number.isInteger(s) && s >= 0 && s < edges.length) selected.add(s)
  }
  const adjacent = new Map<number, number[]>()
  for (let i = 0; i < edges.length; i++) {
    const e = edges[i]
    addAdjacent(adjacent, e.a, i)
    addAdjacent(adjacent, e.b, i)
  }
  const pending = [...selected]
  while (pending.length) {
    const edge = edges[pending.pop()!]
    extendEndpoint(adjacent, selected, pending, edge.a)
    extendEndpoint(adjacent, selected, pending, edge.b)
  }
  return [...selected].sort((a, b) => a - b)
}
