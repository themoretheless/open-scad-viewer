import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {thinWalledOpenSection, type TorsionStrip} from '../src/services/structuralSections'

const strip = (a:[number,number],b:[number,number],thicknessMm:number):TorsionStrip =>
  ({a,b,thicknessMm})

it('matches the textbook channel shear center and warping constant through real WASM', () => {
  // Channel b = 50 flanges, h = 100 web, t = 5, opening toward +x.
  const [b,h,t] = [50,100,5]
  const r = thinWalledOpenSection([
    strip([b,h/2],[0,h/2],t),
    strip([0,h/2],[0,-h/2],t),
    strip([0,-h/2],[b,-h/2],t),
  ])
  expect(r.areaMm2).toBeCloseTo(t*(2*b+h), 9)
  expect(r.centroidMm[0]).toBeCloseTo(12.5, 9)
  // Shear center outside the web, opposite the opening: e = 3b²/(h+6b).
  expect(r.shearCenterMm[0]).toBeCloseTo(-3*b*b/(h+6*b), 9)
  expect(r.shearCenterMm[1]).toBeCloseTo(0, 9)
  // Cw = t·b³h²/12·(2h+3b)/(h+6b).
  expect(r.cwMm6).toBeCloseTo(t*b**3*h*h/12*(2*h+3*b)/(h+6*b), 3)
  expect(r.jMm4).toBeCloseTo((2*b+h)*t**3/3, 6)
})

it('handles branched I-beams and rejects closed loops through real WASM', () => {
  // I-beam: web 100 x 4, flanges 100 x 5; web endpoints split the flanges.
  const r = thinWalledOpenSection([
    strip([0,-50],[0,50],4),
    strip([-50,-50],[50,-50],5),
    strip([-50,50],[50,50],5),
  ])
  // Doubly symmetric: shear center at the centroid, Cw = Iy·h²/4.
  expect(Math.abs(r.shearCenterMm[0])).toBeLessThan(1e-6)
  expect(Math.abs(r.shearCenterMm[1])).toBeLessThan(1e-6)
  expect(r.cwMm6).toBeCloseTo(r.i2Mm4*100*100/4, 3)
  // Vertical shear rides the web (A_web = 400 mm²).
  expect(r.shearArea2Mm2).toBeGreaterThan(330)
  expect(r.shearArea2Mm2).toBeLessThan(400)
  // Closed loop refused.
  expect(() => thinWalledOpenSection([
    strip([0,0],[100,0],5), strip([100,0],[100,100],5),
    strip([100,100],[0,100],5), strip([0,100],[0,0],5),
  ])).toThrow(GeometryKernelError)
})
