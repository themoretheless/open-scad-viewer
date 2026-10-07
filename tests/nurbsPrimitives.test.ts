import { expect, it } from 'vitest'
import { ellipseNurbsArc, ellipsoidNurbsSurface, torusNurbsSurface } from '../src/services/nurbsConstructors'
import { evaluateNurbsCurve } from '../src/services/nurbsCurve'
import { evaluateNurbsSurface } from '../src/services/nurbsSurface'
import { parabolaNurbsCurve, hyperbolaNurbsCurve, ellipticCylinderNurbsSurface, coneFrustumNurbsSurface, quadraticNurbsPatch } from '../src/services/nurbsConstructors'

it('preserves conic equations and surface dimensions through WASM', () => {
  const parabola = parabolaNurbsCurve([0,0,0], [2,0,0], [0,3,0], -2,3)
  const hyperbola = hyperbolaNurbsCurve([0,0,0], [2,0,0], [0,3,0], -1,2)
  for (const u of [0,0.13,0.47,0.8,1]) {
    const t = -2+5*u
    const p = evaluateNurbsCurve(parabola,u).point
    expect(p[0]).toBeCloseTo(2*t,11)
    expect(p[1]).toBeCloseTo(3*t*t,11)
    const h = evaluateNurbsCurve(hyperbola,u).point
    expect(h[0]!**2/4-h[1]!**2/9).toBeCloseTo(1,11)
  }
  const cylinder = ellipticCylinderNurbsSurface([0,0,0],2,3,5)
  const c = evaluateNurbsSurface(cylinder,0.17,0.4).point
  expect(c[0]**2/4+c[1]**2/9).toBeCloseTo(1,11)
  expect(c[2]).toBeCloseTo(2,11)
  const cone = coneFrustumNurbsSurface([0,0,0],3,0,5)
  const q = evaluateNurbsSurface(cone,0.6,1.23).point
  expect(Math.hypot(q[0],q[1])).toBeCloseTo(1.2,11)
  expect(q[2]).toBeCloseTo(3,11)
  const patch = quadraticNurbsPatch([-2,3,-4,1],[1,0,-1,0,0,0])
  const s = evaluateNurbsSurface(patch,0.3,0.8).point
  expect(s[2]).toBeCloseTo(s[0]**2-s[1]**2,11)
})

it('constructs rational primitives through the packaged WASM bridge', () => {
  const c = ellipseNurbsArc([1, 2, 3], [4, 0, 0], [0, 2, 0])
  const p = evaluateNurbsCurve(c, 0.125).point
  expect((p[0]! - 1) ** 2 / 16 + (p[1]! - 2) ** 2 / 4).toBeCloseTo(1, 12)
  expect(c.controlPoints[0]).toEqual(c.controlPoints.at(-1))
  const e = ellipsoidNurbsSurface([0, 0, 0], [2, 3, 4])
  const q = evaluateNurbsSurface(e, 0.37, 1.23).point
  expect(q[0]! ** 2 / 4 + q[1]! ** 2 / 9 + q[2]! ** 2 / 16).toBeCloseTo(1, 12)
  const t = torusNurbsSurface([0, 0, 0], 5, 2, 1)
  const r = evaluateNurbsSurface(t, 0.13, 2.34).point
  expect((Math.hypot(r[0]!, r[1]!) - 5) ** 2 / 4 + r[2]! ** 2).toBeCloseTo(1, 12)
})

it('propagates primitive input errors to TypeScript callers', () => {
  expect(() => torusNurbsSurface([0, 0, 0], 1, 2)).toThrow()
  expect(() => ellipsoidNurbsSurface([0, 0, 0], [1, 0, 1])).toThrow()
  expect(() => ellipseNurbsArc([0, 0, 0], [1, 0, 0], [2, 0, 0])).toThrow()
})

it('represents an elliptic paraboloid with an independent positive quadratic equation',()=>{
  const patch=quadraticNurbsPatch([-10,10,-8,8],[.05,0,.08,0,0,0])
  for(const u of [0,.19,.5,.82,1])for(const v of [0,.13,.5,.91,1]){
    const p=evaluateNurbsSurface(patch,u,v).point
    expect(p[2]).toBeCloseTo(.05*p[0]**2+.08*p[1]**2,11)
    expect(p[2]).toBeGreaterThanOrEqual(-1e-12)
  }
})
