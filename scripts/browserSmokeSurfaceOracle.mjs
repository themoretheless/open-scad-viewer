// Area and bounds of a rational B-spline surface by a fine triangulation of sampled points.
export function nurbsSurfaceMass(surface) {
  const basis = (degree, knots, count, t) => {
    const span = Math.min(
      Math.max(
        knots.findLastIndex(k => k <= t),
        degree,
      ),
      count - 1,
    )
    const values = new Array(count).fill(0)
    const left = [],
      right = [],
      n = [1]
    for (let j = 1; j <= degree; j++) {
      left[j] = t - knots[span + 1 - j]
      right[j] = knots[span + j] - t
      let saved = 0
      for (let r = 0; r < j; r++) {
        const tmp = n[r] / (right[r + 1] + left[j - r])
        n[r] = saved + right[r + 1] * tmp
        saved = left[j - r] * tmp
      }
      n[j] = saved
    }
    for (let j = 0; j <= degree; j++) values[span - degree + j] = n[j]
    return values
  }
  const cp = surface.controlPoints,
    rowsU = cp.length,
    rowsV = cp[0].length
  const weight = (i, j) => surface.weights?.[i]?.[j] ?? 1
  const domain = knots => [knots[0], knots.at(-1)]
  const [u0, u1] = domain(surface.knotsU),
    [v0, v1] = domain(surface.knotsV)
  const at = (u, v) => {
    const bu = basis(surface.degreeU, surface.knotsU, rowsU, u),
      bv = basis(surface.degreeV, surface.knotsV, rowsV, v)
    const sum = [0, 0, 0, 0]
    for (let i = 0; i < rowsU; i++)
      for (let j = 0; j < rowsV; j++) {
        const w = bu[i] * bv[j] * weight(i, j)
        for (let k = 0; k < 3; k++) sum[k] += w * cp[i][j][k]
        sum[3] += w
      }
    return sum.slice(0, 3).map(x => x / sum[3])
  }
  const n = 64,
    grid = []
  for (let i = 0; i <= n; i++) {
    grid.push([])
    for (let j = 0; j <= n; j++) grid[i].push(at(u0 + ((u1 - u0) * i) / n, v0 + ((v1 - v0) * j) / n))
  }
  const triangle = (a, b, c) => {
    const e = [0, 1, 2].map(k => b[k] - a[k]),
      f = [0, 1, 2].map(k => c[k] - a[k])
    return Math.hypot(e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]) / 2
  }
  let area = 0
  for (let i = 0; i < n; i++)
    for (let j = 0; j < n; j++)
      area +=
        triangle(grid[i][j], grid[i + 1][j], grid[i + 1][j + 1]) +
        triangle(grid[i][j], grid[i + 1][j + 1], grid[i][j + 1])
  const flat = grid.flat()
  const bounds = [0, 1].map(side => [0, 1, 2].map(k => (side ? Math.max : Math.min)(...flat.map(p => p[k]))))
  return { area, bounds }
}
