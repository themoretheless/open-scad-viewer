import { describe, expect, it } from 'vitest'
import type { DirectSketch } from '../src/app/directModeling'
import {
  combineSketchProfiles,
  selectSketchProfileRegions,
  sketchProfile,
  sketchProfileRegionAt,
  sketchProfileRegions,
} from '../src/app/retainedSketchProfile'
import { profileBooleanOperation } from '../src/features/directModeler/helpers'

const rectangle = (id: string, x: number, y: number, w: number, h: number): DirectSketch => ({
  id,
  name: id,
  closed: true,
  points: [
    [x, y],
    [x + w, y],
    [x + w, y + h],
    [x, y + h],
  ],
})
const circle = (id: string, cx: number, cy: number, radius: number): DirectSketch => ({
  id,
  name: id,
  closed: true,
  points: [],
  analytic: { kind: 'circle', center: [cx, cy], radius, start: 0, sweep: 360 },
})
const TOL = 1e-6
const sortedAreas = (sketch: DirectSketch) =>
  sketchProfileRegions(sketchProfile(sketch))
    .map(r => r.areaMm2)
    .sort((x, y) => x - y)

describe('C9 profile xor', () => {
  it('maps the palette command to the xor operation', () => {
    expect(profileBooleanOperation('profile-xor')).toBe('xor')
  })

  it('keeps the symmetric difference of two overlapping rectangles as two regions', () => {
    const source = [rectangle('a', 0, 0, 10, 4), rectangle('b', 6, 0, 10, 4)]
    const before = structuredClone(source)
    const result = combineSketchProfiles(source, 'xor')
    expect(source).toEqual(before)
    expect(result.id).toBe('a')
    const profile = sketchProfile(result)
    expect(Math.abs(profile.areaMm2 - 48)).toBeLessThan(TOL)
    const areas = sortedAreas(result)
    expect(areas).toHaveLength(2)
    expect(Math.abs(areas[0] - 24)).toBeLessThan(TOL)
    expect(Math.abs(areas[1] - 24)).toBeLessThan(TOL)
    expect(sketchProfileRegionAt(profile, [8, 2])).toBe(-1)
    expect(sketchProfileRegionAt(profile, [2, 2])).toBeGreaterThanOrEqual(0)
    expect(sketchProfileRegionAt(profile, [14, 2])).toBeGreaterThanOrEqual(0)
  })

  it('keeps a notched plate and the circle overhang', () => {
    const result = combineSketchProfiles([rectangle('plate', -4, -3, 8, 6), circle('c', 4, 0, 2)], 'xor')
    // Half of the disk lies inside the plate, half outside.
    expect(Math.abs(sketchProfile(result).areaMm2 - 48)).toBeLessThan(TOL)
    const areas = sortedAreas(result)
    expect(areas).toHaveLength(2)
    expect(Math.abs(areas[0] - 2 * Math.PI)).toBeLessThan(TOL)
    expect(Math.abs(areas[1] - (48 - 2 * Math.PI))).toBeLessThan(TOL)
  })

  it('treats a contained profile as a hole', () => {
    const profile = sketchProfile(combineSketchProfiles([rectangle('plate', 0, 0, 8, 6), circle('c', 4, 3, 1)], 'xor'))
    expect(Math.abs(profile.areaMm2 - (48 - Math.PI))).toBeLessThan(TOL)
    expect(sketchProfileRegions(profile)).toHaveLength(1)
    expect(profile.loops).toHaveLength(2)
  })

  it('lets the user keep only some xor regions', () => {
    const result = combineSketchProfiles([rectangle('a', 0, 0, 10, 4), rectangle('b', 6, 0, 10, 4)], 'xor')
    expect(Math.abs(sketchProfile(selectSketchProfileRegions(result, [0])).areaMm2 - 24)).toBeLessThan(TOL)
    expect(() => selectSketchProfileRegions(result, [0, 1])).toThrow('Keep at least one profile region.')
  })

  it('returns both inputs for disjoint profiles', () => {
    const result = combineSketchProfiles([rectangle('a', 0, 0, 4, 4), circle('c', 20, 0, 2)], 'xor')
    expect(Math.abs(sketchProfile(result).areaMm2 - (16 + 4 * Math.PI))).toBeLessThan(TOL)
    const areas = sortedAreas(result)
    expect(areas).toHaveLength(2)
    expect(Math.abs(areas[0] - 4 * Math.PI)).toBeLessThan(TOL)
    expect(Math.abs(areas[1] - 16)).toBeLessThan(TOL)
  })

  it('refuses identical inputs because nothing remains', () => {
    const source = [rectangle('a', 0, 0, 4, 4), rectangle('b', 0, 0, 4, 4)]
    const before = structuredClone(source)
    expect(() => combineSketchProfiles(source, 'xor')).toThrow('contains no material')
    expect(source).toEqual(before)
  })

  it('refuses fewer than two profiles', () => {
    expect(() => combineSketchProfiles([rectangle('a', 0, 0, 4, 4)], 'xor')).toThrow(
      'Select at least two closed profiles.',
    )
  })
})
