/** Exact CPU Catmull-Clark refinement (WebGPU fallback). */
import type { SubdivisionCage, SubdivisionRefined } from './geometry/subdivision'
import { packUndirectedEdgeKey } from './math3d'

const edgeKey = packUndirectedEdgeKey

interface EdgeUse {
  a: number
  b: number
  faces: number[]
  edgeIndex: number
}

/**
 * Exact CPU reference implementation of Catmull–Clark refinement matching `subdivision_core::Cage::subdivide`.
 */
export function referenceRefineSubdivision(cage: SubdivisionCage, levels = 2): SubdivisionRefined {
  if (!Number.isInteger(levels) || levels < 0 || levels > 5) {
    throw new Error('Subdivision levels must be 0..5')
  }
  if (
    cage.vertices.length === 0 ||
    cage.vertices.length > 100_000 ||
    cage.faces.length === 0 ||
    cage.faces.length > 25_000
  ) {
    throw new Error('Invalid control cage or budget exceeded')
  }

  let currentVertices = cage.vertices.map((p) => [p[0]!, p[1]!, p[2]!])
  let currentFaces = cage.faces.map((f) => [...f])
  let faceIds = cage.faces.map((_, idx) => idx)

  for (let step = 0; step < levels; step++) {
    let cornerSum = 0
    for (let i = 0; i < currentFaces.length; i++) cornerSum += currentFaces[i]!.length
    if (cornerSum > 25_000) {
      throw new Error('Subdivision face budget exceeded')
    }

    const edgeMap = new Map<number | string, EdgeUse>()
    const edgeList: EdgeUse[] = []

    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const flen = f.length
      for (let i = 0; i < flen; i++) {
        const u = f[i]!
        const w = f[(i + 1) % flen]!
        const a = u < w ? u : w
        const b = u < w ? w : u
        const key = edgeKey(a, b)
        let entry = edgeMap.get(key)
        if (!entry) {
          entry = { a, b, faces: [], edgeIndex: edgeList.length }
          edgeMap.set(key, entry)
          edgeList.push(entry)
        }
        entry.faces.push(fi)
      }
    }


    // 1. Face points
    const facePoints: [number, number, number][] = new Array(currentFaces.length)
    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const inv = 1 / f.length
      let sx = 0, sy = 0, sz = 0
      for (let i = 0; i < f.length; i++) {
        const p = currentVertices[f[i]!]!
        sx += p[0]! * inv
        sy += p[1]! * inv
        sz += p[2]! * inv
      }
      facePoints[fi] = [sx, sy, sz]
    }

    // Build vertex adjacency
    const vertexEdges: EdgeUse[][] = Array.from({ length: currentVertices.length }, () => [])
    for (let ei = 0; ei < edgeList.length; ei++) {
      const e = edgeList[ei]!
      vertexEdges[e.a]!.push(e)
      vertexEdges[e.b]!.push(e)
    }

    // 2. Updated vertex points
    const nextVertices: number[][] = new Array(
      currentVertices.length + edgeList.length + currentFaces.length
    )

    for (let v = 0; v < currentVertices.length; v++) {
      const p = currentVertices[v]!
      const adj = vertexEdges[v]!
      const boundaryNbrs: number[] = []
      for (let i = 0; i < adj.length; i++) {
        const e = adj[i]!
        if (e.faces.length === 1) {
          boundaryNbrs.push(e.a === v ? e.b : e.a)
        }
      }

      if (boundaryNbrs.length === 2) {
        const b0 = currentVertices[boundaryNbrs[0]!]!
        const b1 = currentVertices[boundaryNbrs[1]!]!
        nextVertices[v] = [
          (6 * p[0]! + b0[0]! + b1[0]!) / 8,
          (6 * p[1]! + b0[1]! + b1[1]!) / 8,
          (6 * p[2]! + b0[2]! + b1[2]!) / 8,
        ]
      } else {
        const fSet = new Set<number>()
        let rx = 0, ry = 0, rz = 0
        const n = adj.length
        const invN = 1 / n
        for (let i = 0; i < n; i++) {
          const e = adj[i]!
          const pa = currentVertices[e.a]!
          const pb = currentVertices[e.b]!
          rx += 0.5 * (pa[0]! + pb[0]!) * invN
          ry += 0.5 * (pa[1]! + pb[1]!) * invN
          rz += 0.5 * (pa[2]! + pb[2]!) * invN
          for (let j = 0; j < e.faces.length; j++) {
            fSet.add(e.faces[j]!)
          }
        }
        let fx = 0, fy = 0, fz = 0
        const invF = 1 / fSet.size
        for (const fi of fSet) {
          const fp = facePoints[fi]!
          fx += fp[0] * invF
          fy += fp[1] * invF
          fz += fp[2] * invF
        }
        nextVertices[v] = [
          (fx + 2 * rx + (n - 3) * p[0]!) / n,
          (fy + 2 * ry + (n - 3) * p[1]!) / n,
          (fz + 2 * rz + (n - 3) * p[2]!) / n,
        ]
      }
    }

    // 3. Edge points
    const edgeOffset = currentVertices.length
    for (let ei = 0; ei < edgeList.length; ei++) {
      const e = edgeList[ei]!
      const pa = currentVertices[e.a]!
      const pb = currentVertices[e.b]!
      if (e.faces.length === 2) {
        const fp0 = facePoints[e.faces[0]!]!
        const fp1 = facePoints[e.faces[1]!]!
        nextVertices[edgeOffset + ei] = [
          (pa[0]! + pb[0]! + fp0[0] + fp1[0]) * 0.25,
          (pa[1]! + pb[1]! + fp0[1] + fp1[1]) * 0.25,
          (pa[2]! + pb[2]! + fp0[2] + fp1[2]) * 0.25,
        ]
      } else {
        nextVertices[edgeOffset + ei] = [
          (pa[0]! + pb[0]!) * 0.5,
          (pa[1]! + pb[1]!) * 0.5,
          (pa[2]! + pb[2]!) * 0.5,
        ]
      }
    }

    // 4. Append face points
    const faceOffset = edgeOffset + edgeList.length
    for (let fi = 0; fi < currentFaces.length; fi++) {
      nextVertices[faceOffset + fi] = facePoints[fi]!
    }

    // 5. Assemble subdivided quad faces
    const nextFaces: number[][] = []
    const nextIds: number[] = []
    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const flen = f.length
      for (let i = 0; i < flen; i++) {
        const curr = f[i]!
        const next = f[(i + 1) % flen]!
        const prev = f[(i + flen - 1) % flen]!
        const eNext = edgeOffset + edgeMap.get(edgeKey(curr, next))!.edgeIndex
        const ePrev = edgeOffset + edgeMap.get(edgeKey(prev, curr))!.edgeIndex
        nextFaces.push([curr, eNext, faceOffset + fi, ePrev])
        nextIds.push(faceIds[fi]!)
      }
    }

    currentVertices = nextVertices
    currentFaces = nextFaces
    faceIds = nextIds
  }

  return {
    cage: { vertices: currentVertices, faces: currentFaces },
    faceIds,
  }
}
