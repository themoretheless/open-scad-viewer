import {expect, it} from 'vitest'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {
  hyperboloidOneSheetNurbsSurface, hyperboloidTwoSheetNurbsSurface,
  polynomialGraphNurbsSurface, polynomialNurbsCurve, polynomialNurbsSurface,
  rationalPolynomialNurbsCurve, rationalPolynomialNurbsSurface,
} from '../src/services/nurbsConstructors'

it('preserves non-unit polynomial domains through the packaged host', () => {
  const c = polynomialNurbsCurve([-2,3], [[1,2,3],[2,-1,0],[0,0,4]])
  const s = polynomialNurbsSurface([-2,3,-1,2], [
    [[0,0,0],[0,1,0]], [[1,0,0],[0,0,1]], [[0,0,2],[0,0,0]],
  ])
  const graph = polynomialGraphNurbsSurface([-2,3,-1,2], [[1,3],[2,4]])
  for (const u of [0,0.17,0.53,1]) {
    const x = -2+5*u
    const curvePoint = evaluateNurbsCurve(c,u).point
    expect(curvePoint[0]).toBeCloseTo(1+2*x,10)
    expect(curvePoint[1]).toBeCloseTo(2-x,10)
    expect(curvePoint[2]).toBeCloseTo(3+4*x*x,10)
    for (const v of [0,0.39,1]) {
      const y = -1+3*v
      const p = evaluateNurbsSurface(s,u,v).point
      expect(p[0]).toBeCloseTo(x,10)
      expect(p[1]).toBeCloseTo(y,10)
      expect(p[2]).toBeCloseTo(x*y+2*x*x,10)
      expect(evaluateNurbsSurface(graph,u,v).point[2]).toBeCloseTo(1+2*x+3*y+4*x*y,10)
    }
  }
})

it('materializes homogeneous curve and surface formulas with safe denominators', () => {
  const curve = rationalPolynomialNurbsCurve([0,1], [[1,0,0,1],[0,2,0,0],[-1,0,0,1]])
  const surface = rationalPolynomialNurbsSurface([0,1,0,1], [
    [[0,0,0,1],[0,1,0,0]], [[1,0,0,1],[0,0,1,0]],
  ])
  for (const u of [0,0.23,0.81,1]) {
    const p = evaluateNurbsCurve(curve,u).point
    expect(p[0]).toBeCloseTo((1-u*u)/(1+u*u),11)
    expect(p[1]).toBeCloseTo(2*u/(1+u*u),11)
    const q = evaluateNurbsSurface(surface,u,0.4).point
    expect(q[0]).toBeCloseTo(u/(1+u),11)
    expect(q[1]).toBeCloseTo(0.4/(1+u),11)
    expect(q[2]).toBeCloseTo(0.4*u/(1+u),11)
  }
  expect(()=>rationalPolynomialNurbsCurve([-1,1], [[1,0,0,0.01],[0,0,0,0],[0,0,0,1]])).toThrow()
})

it('preserves both hyperboloid equations and lower-sheet selection', () => {
  const one = hyperboloidOneSheetNurbsSurface([1,2,3],[2,3,4],-1,2)
  const lower = hyperboloidTwoSheetNurbsSurface([1,2,3],[2,3,4],0,2,true)
  for (const u of [0,0.21,0.68,1]) {
    for (const v of [0,0.7,2.3,4]) {
      const a = evaluateNurbsSurface(one,u,v).point.map((p,i)=>(p-[1,2,3][i]!)/[2,3,4][i]!)
      expect(a[0]!**2+a[1]!**2-a[2]!**2).toBeCloseTo(1,10)
      const b = evaluateNurbsSurface(lower,u,v).point.map((p,i)=>(p-[1,2,3][i]!)/[2,3,4][i]!)
      expect(b[2]!**2-b[0]!**2-b[1]!**2).toBeCloseTo(1,10)
      expect(b[2]).toBeLessThanOrEqual(-1+1e-12)
    }
  }
})
