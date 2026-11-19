export interface ImageContourSegment2D {
  p0: [number, number]
  p1: [number, number]
}

function quantKey(p: [number, number]): number {
  const qx = Math.round(p[0] * 1e4) + 50_000_000
  const qy = Math.round(p[1] * 1e4) + 50_000_000
  return qx * 100_000_000 + qy
}

/**
 * Stitches directed 2D Marching Squares segments into closed or open polyline contours.
 */
export function stitchSegmentsToContours(segments: readonly ImageContourSegment2D[]): [number, number][][] {
  const n = segments.length
  if (n === 0) return []

  const startKeys = new Float64Array(n)
  const endKeys = new Float64Array(n)
  const startMap = new Map<number, { indices: number[]; cursor: number }>()

  for (let i = 0; i < n; i++) {
    const seg = segments[i]!
    const sk = quantKey(seg.p0)
    startKeys[i] = sk
    endKeys[i] = quantKey(seg.p1)
    let bucket = startMap.get(sk)
    if (!bucket) {
      bucket = { indices: [i], cursor: 0 }
      startMap.set(sk, bucket)
    } else {
      bucket.indices.push(i)
    }
  }

  const visited = new Uint8Array(n)
  const contours: [number, number][][] = []

  for (let i = 0; i < n; i++) {
    if (visited[i]) continue
    visited[i] = 1

    const seg0 = segments[i]!
    const ring: [number, number][] = [
      [seg0.p0[0], seg0.p0[1]],
      [seg0.p1[0], seg0.p1[1]],
    ]

    let endKey = endKeys[i]!
    const firstKey = startKeys[i]!

    while (true) {
      if (endKey === firstKey && ring.length >= 3) {
        ring.pop()
        break
      }

      const bucket = startMap.get(endKey)
      if (!bucket) break

      const candidates = bucket.indices
      let nextIdx = -1
      let k = bucket.cursor
      while (k < candidates.length) {
        const cand = candidates[k]!
        if (!visited[cand]) {
          nextIdx = cand
          bucket.cursor = k + 1
          break
        }
        k++
      }
      if (nextIdx === -1) {
        bucket.cursor = candidates.length
        break
      }

      visited[nextIdx] = 1
      const currentEnd = segments[nextIdx]!.p1
      endKey = endKeys[nextIdx]!
      ring.push([currentEnd[0], currentEnd[1]])
    }

    if (ring.length >= 3) {
      contours.push(ring)
    }
  }

  return contours
}
