/** Own sampled involute profiles. Dimension conventions follow KHK's gear technical
 * reference: https://khkgears.net/gear-knowledge/gear-technical-reference/calculation-gear-dimensions/
 * No cutter simulation, trochoidal root fillet or profile shift is implied. */
export type GearOptions = {
  teeth: number
  module: number
  pressure_angle: number
  thickness: number
  bore: number
  backlash: number
  clearance: number
  internal: boolean
  rim_width: number
  flank_segments: number
}

export type GearReport = {
  kind: 'external_spur_gear' | 'internal_spur_gear'
  teeth: number
  module_mm: number
  pressure_angle_deg: number
  pitch_radius_mm: number
  base_radius_mm: number
  tip_radius_mm: number
  root_radius_mm: number
  outside_radius_mm: number
  pitch_diameter_mm: number
  outside_diameter_mm: number
  thickness_mm: number
  bore_diameter_mm: number
  tooth_thickness_at_pitch_mm: number
  backlash_per_gear_mm: number
  clearance_mm: number
  tooth_center_angle_deg: 0
  profile_vertices: number
  profile_area_mm2: number
  expected_volume_mm3: number
  minimum_external_teeth_without_undercut: number
  root_transition: 'radial_below_base_circle' | 'involute_to_root_circle'
  warnings: string[]
}

type Point = [number, number]
const TAU = 2 * Math.PI
const round = (v: number) => Math.round(v * 1e7) / 1e7
const polar = (r: number, a: number): Point => [round(r * Math.cos(a)), round(r * Math.sin(a))]
const area = (points: Point[]) => Math.abs(points.reduce((sum, p, i) => {
  const next = points[(i + 1) % points.length]!
  return sum + p[0] * next[1] - p[1] * next[0]
}, 0)) / 2

/** Planar loops are also exposed for rigid placement without growing the worker's
 * source-provenance hierarchy. The first loop is material; later loops are holes. */
export function buildModelGraphGearProfile(options: GearOptions): { loops: Point[][]; report: GearReport } {
  const o = options
  for (const [key, value] of Object.entries(o)) {
    if (key !== 'internal' && (typeof value !== 'number' || !Number.isFinite(value))) throw new Error(`Gear ${key} must be finite.`)
  }
  if (typeof o.internal !== 'boolean') throw new Error('Gear internal must be boolean.')
  if (!Number.isInteger(o.teeth) || o.teeth < 8 || o.teeth > 128) throw new Error('Gear teeth must be an integer from 8 to 128.')
  if (!Number.isInteger(o.flank_segments) || o.flank_segments < 3 || o.flank_segments > 12) throw new Error('Gear flank_segments must be an integer from 3 to 12.')
  if (o.module < 0.1 || o.module > 100) throw new Error('Gear module must be from 0.1 to 100 mm.')
  if (o.pressure_angle < 14.5 || o.pressure_angle > 30) throw new Error('Gear pressure_angle must be from 14.5 to 30 degrees.')
  if (o.thickness < 0.1 || o.thickness > 1000) throw new Error('Gear thickness must be from 0.1 to 1000 mm.')
  if (o.backlash < 0 || o.backlash > o.module / 2) throw new Error('Gear backlash must be from zero to half the module; it reduces each gear tooth thickness.')
  if (o.clearance < 0 || o.clearance > o.module) throw new Error('Gear clearance must be from zero to one module.')
  if (o.bore < 0 || o.rim_width < 0 || o.rim_width > 10000) throw new Error('Gear bore and rim_width must be nonnegative, with rim_width at most 10000 mm.')
  const alpha = o.pressure_angle * Math.PI / 180
  const minimumTeeth = Math.ceil(2 / Math.sin(alpha) ** 2 - 1e-12)
  if (!o.internal && o.teeth < minimumTeeth) throw new Error(`Gear requires at least ${minimumTeeth} teeth at this pressure angle; undercut and profile shift are not implemented.`)
  const pitch = o.module * o.teeth / 2
  const base = pitch * Math.cos(alpha)
  const tip = pitch + (o.internal ? -o.module : o.module)
  const root = pitch + (o.internal ? 1 : -1) * (o.module + o.clearance)
  const outside = o.internal ? root + o.rim_width : tip
  if (root <= 0 || outside > 10000) throw new Error('Gear root radius must be positive and outside radius at most 10000 mm.')
  if (o.internal && tip <= base + 1e-9) throw new Error('Internal gear tip must remain outside the base circle; increase teeth or pressure_angle.')
  if (o.internal && o.bore !== 0) throw new Error('Internal gear uses a toothed central opening; bore must be zero.')
  if (o.internal && o.rim_width < o.module / 4) throw new Error('Internal gear rim_width must be at least one quarter module.')
  if (!o.internal && o.bore > 2 * root - o.module / 2) throw new Error('Gear bore must leave at least one quarter module of material inside the root circle.')

  // An internal cavity is the complement of an involute lobe with increased
  // pitch thickness, centered in the space between two material teeth.
  const low = o.internal ? tip : root
  const high = o.internal ? root : tip
  const start = Math.max(base, low)
  const invPitch = Math.tan(alpha) - alpha
  const halfPitch = Math.PI / (2 * o.teeth) + (o.internal ? 1 : -1) * o.backlash / (2 * pitch)
  const involute = (r: number) => {
    const t = Math.sqrt(Math.max(0, (r / base) ** 2 - 1))
    return t - Math.atan(t)
  }
  const half = (r: number) => halfPitch + invPitch - involute(r)
  const lowHalf = half(start), highHalf = half(high), toothStep = TAU / o.teeth
  if (highHalf <= 1e-6 || lowHalf >= toothStep / 2 - 1e-6) throw new Error('Gear tooth profile collapses or overlaps; reduce backlash/clearance or change teeth/pressure_angle.')
  const t0 = Math.sqrt(Math.max(0, (start / base) ** 2 - 1))
  const t1 = Math.sqrt(Math.max(0, (high / base) ** 2 - 1))
  // Include the pitch circle explicitly so the declared backlash is represented
  // exactly there, even with a coarse display resolution.
  const samples = Array.from({ length: o.flank_segments + 1 }, (_, i) => t0 + (t1 - t0) * i / o.flank_segments)
  samples.push(Math.tan(alpha))
  samples.sort((a, b) => a - b)
  const radii = samples.filter((t, i) => i === 0 || t - samples[i - 1]! > 1e-10).map(t => base * Math.sqrt(1 + t * t))
  const profile: Point[] = []
  const add = (r: number, angle: number) => {
    const point = polar(r, angle), previous = profile.at(-1)
    if (!previous || previous[0] !== point[0] || previous[1] !== point[1]) profile.push(point)
  }
  const arc = (r: number, from: number, to: number) => {
    const count = Math.max(2, Math.ceil((to - from) / toothStep * 8))
    for (let i = 1; i <= count; i++) add(r, from + (to - from) * i / count)
  }
  for (let tooth = 0; tooth < o.teeth; tooth++) {
    const center = tooth * toothStep + (o.internal ? toothStep / 2 : 0)
    add(low, center - lowHalf)
    for (const r of radii) add(r, center - half(r))
    arc(high, center - highHalf, center + highHalf)
    for (let i = radii.length - 2; i >= 0; i--) add(radii[i]!, center + half(radii[i]!))
    add(low, center + lowHalf)
    arc(low, center + lowHalf, center + toothStep - lowHalf)
  }
  if (profile[0]![0] === profile.at(-1)![0] && profile[0]![1] === profile.at(-1)![1]) profile.pop()
  const circle = (r: number) => Array.from({ length: Math.max(64, o.teeth * 8) }, (_, i) => polar(r, i * TAU / Math.max(64, o.teeth * 8)))
  const loops = o.internal ? [circle(outside), profile.slice().reverse()] : o.bore > 0 ? [profile, circle(o.bore / 2).reverse()] : [profile]
  const profileArea = area(loops[0]!) - loops.slice(1).reduce((sum, loop) => sum + area(loop), 0)
  const vertices = loops.reduce((sum, loop) => sum + loop.length, 0)
  if (vertices > 6000 || !Number.isFinite(profileArea) || profileArea <= 0) throw new Error('Gear exceeds the 6000 profile vertex budget or has no material area.')
  return { loops, report: {
    kind: o.internal ? 'internal_spur_gear' : 'external_spur_gear', teeth: o.teeth, module_mm: o.module,
    pressure_angle_deg: o.pressure_angle, pitch_radius_mm: pitch, base_radius_mm: base,
    tip_radius_mm: tip, root_radius_mm: root, outside_radius_mm: outside, pitch_diameter_mm: 2 * pitch,
    outside_diameter_mm: 2 * outside, thickness_mm: o.thickness, bore_diameter_mm: o.bore,
    tooth_thickness_at_pitch_mm: Math.PI * o.module / 2 - o.backlash, backlash_per_gear_mm: o.backlash,
    clearance_mm: o.clearance, tooth_center_angle_deg: 0, profile_vertices: vertices,
    profile_area_mm2: profileArea, expected_volume_mm3: profileArea * o.thickness,
    minimum_external_teeth_without_undercut: minimumTeeth,
    root_transition: low < base ? 'radial_below_base_circle' : 'involute_to_root_circle',
    warnings: ['Flanks and circles are sampled; tooth roots have no generated trochoidal fillet. Mating interference, strength and printer tolerances require separate validation.'],
  } }
}

export function extrudeGearProfile(loops: Point[][], thickness: number): string {
  let offset = 0
  const paths = loops.map(loop => { const path = loop.map((_, i) => offset + i); offset += loop.length; return path })
  return `linear_extrude(height=${thickness})polygon(points=${JSON.stringify(loops.flat())},paths=${JSON.stringify(paths)});`
}

export function buildModelGraphGear(options: GearOptions): { source: string; report: GearReport; loops: Point[][] } {
  const { loops, report } = buildModelGraphGearProfile(options)
  return { source: extrudeGearProfile(loops, options.thickness), report, loops }
}
