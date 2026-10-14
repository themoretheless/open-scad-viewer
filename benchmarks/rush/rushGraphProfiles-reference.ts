type Point = readonly [number, number]
/** Accept a simple implicitly closed polygon; holes are modeled with profile difference. */
export function validateProfilePolygon(points: readonly Point[], path: string, fail: (code: string, path: string, message: string) => never) {
  const span = Math.max(...points.map(p => p[0])) - Math.min(...points.map(p => p[0])) + Math.max(...points.map(p => p[1])) - Math.min(...points.map(p => p[1]))
  const epsilon = Math.max(Number.EPSILON, span * span * 1e-12)
  const cross = (a: Point, b: Point, c: Point) => (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
  const on = (a: Point, b: Point, p: Point) => Math.abs(cross(a, b, p)) <= epsilon && p[0] >= Math.min(a[0], b[0]) && p[0] <= Math.max(a[0], b[0]) && p[1] >= Math.min(a[1], b[1]) && p[1] <= Math.max(a[1], b[1])
  let area = 0
  for (let i = 0; i < points.length; i++) {
    const a = points[i]!, b = points[(i + 1) % points.length]!
    if (a[0] === b[0] && a[1] === b[1]) fail('invalid_profile', `${path}/points/${i}`, 'Consecutive points must differ; omit the repeated closing point.')
    area += cross(points[0]!, a, b)
    for (let j = i + 2; j < points.length; j++) {
      if (i === 0 && j === points.length - 1) continue
      const c = points[j]!, d = points[(j + 1) % points.length]!
      const abC = cross(a, b, c), abD = cross(a, b, d), cdA = cross(c, d, a), cdB = cross(c, d, b)
      if ((abC * abD < 0 && cdA * cdB < 0) || on(a, b, c) || on(a, b, d) || on(c, d, a) || on(c, d, b)) fail('invalid_profile', path + '/points', `Polygon edges ${i} and ${j} intersect.`)
    }
  }
  if (Math.abs(area) <= epsilon) fail('invalid_profile', path + '/points', 'Polygon must enclose nonzero area.')
}
