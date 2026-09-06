import { buildModelGraphGear, extrudeGearProfile, type GearOptions } from './modelGraphGears'

export type PlanetaryOptions = {
  sun_teeth: number
  planet_teeth: number
  planet_count: number
  module: number
  pressure_angle: number
  thickness: number
  bore: number
  backlash: number
  clearance: number
  rim_width: number
  flank_segments: number
  carrier_angle: number
}

export type PlanetaryPart = {
  id: string
  role: 'sun' | 'planet' | 'ring'
  source: string
  local_source: string
  gear_options: GearOptions
  pose: { origin: [number, number, number]; rotation: [number, number, number] }
}

const degrees = Math.PI / 180
const number = (value: number) => Math.abs(value) < 1e-10 ? 0 : Number(value.toFixed(10))
const angle = (value: number) => ((value % 360) + 540) % 360 - 180

/** Standard, unshifted spur planetary gearset; a carrier pose is kinematic, not a physical carrier. */
export function buildModelGraphPlanetary(options: PlanetaryOptions) {
  for (const [key, value] of Object.entries(options)) {
    if (!Number.isFinite(value)) throw new Error(`Planetary ${key} must be finite.`)
  }
  const { sun_teeth: sunTeeth, planet_teeth: planetTeeth, planet_count: count } = options
  if (!Number.isInteger(count) || count < 2 || count > 6) throw new Error('Planetary planet_count must be an integer from 2 to 6.')
  if (!Number.isInteger(sunTeeth) || !Number.isInteger(planetTeeth) || sunTeeth < 1 || planetTeeth < 1) throw new Error('Planetary tooth counts must be positive integers.')
  if (Math.abs(options.carrier_angle) > 360000) throw new Error('Planetary carrier_angle must be within ±360000 degrees.')
  const ringTeeth = sunTeeth + 2 * planetTeeth
  // KHK, Internal Gears, "Gear tooth conditions for planetary mechanisms":
  // https://khkgears.net/pdf/2025/internal-gears.pdf (page 2).
  if ((sunTeeth + ringTeeth) % count !== 0) throw new Error('Equally spaced planets require (sun_teeth + ring_teeth) / planet_count to be an integer.')

  const common = {
    module: options.module, pressure_angle: options.pressure_angle, thickness: options.thickness,
    bore: options.bore, backlash: options.backlash, clearance: options.clearance,
    rim_width: options.rim_width, flank_segments: options.flank_segments,
  }
  const sunOptions: GearOptions = { ...common, teeth: sunTeeth, internal: false }
  const planetOptions: GearOptions = { ...common, teeth: planetTeeth, internal: false }
  const ringOptions: GearOptions = { ...common, teeth: ringTeeth, internal: true, bore: 0 }
  const sun = buildModelGraphGear(sunOptions)
  const planet = buildModelGraphGear(planetOptions)
  const ring = buildModelGraphGear(ringOptions)
  const orbitRadius = options.module * (sunTeeth + planetTeeth) / 2
  const adjacentCenterDistance = 2 * orbitRadius * Math.sin(Math.PI / count)
  const adjacentTipGap = adjacentCenterDistance - 2 * planet.report.tip_radius_mm
  if (adjacentTipGap <= options.module * 1e-8) throw new Error('Adjacent planet addendum circles overlap or touch; reduce planet_count or increase sun_teeth.')

  // At the internal tooth tip, the contact must remain on the planet's involute,
  // above its base circle. Distance along the common line of action provides
  // this check without relying on stock-gear lookup tables or tessellation.
  const internalContactMargin = Math.sqrt(Math.max(0, ring.report.tip_radius_mm ** 2 - ring.report.base_radius_mm ** 2)) - orbitRadius * Math.sin(options.pressure_angle * degrees)
  if (internalContactMargin <= options.module * 1e-8) throw new Error('Internal involute interference: ring tooth tips reach below the planet base circle; increase planet_teeth.')

  const carrierAngle = options.carrier_angle
  const sunAngle = (1 + ringTeeth / sunTeeth) * carrierAngle
  // With material tooth centers at +X on both gear kinds, odd-tooth planets
  // require a half ring pitch at the reference carrier position.
  const ringAngle = (planetTeeth % 2) * 180 / ringTeeth
  const parts: PlanetaryPart[] = []
  const place = (id: string, role: PlanetaryPart['role'], gear: ReturnType<typeof buildModelGraphGear>, gearOptions: GearOptions, x: number, y: number, rotation: number) => {
    const rotationAngle = angle(rotation)
    const origin: [number, number, number] = [number(x), number(y), 0]
    const c = number(Math.cos(rotationAngle * degrees)), s = number(Math.sin(rotationAngle * degrees))
    parts.push({ id, role, local_source: gear.source, source: extrudeGearProfile(gear.loops.map(loop=>loop.map(([x,y])=>[number(c*x-s*y+origin[0]),number(s*x+c*y+origin[1])])),options.thickness), gear_options: gearOptions, pose: { origin, rotation: [0, 0, number(rotationAngle)] } })
  }
  place('sun', 'sun', sun, sunOptions, 0, 0, sunAngle)
  place('ring', 'ring', ring, ringOptions, 0, 0, ringAngle)
  const planetAngles: number[] = []
  for (let i = 0; i < count; i++) {
    const orbitAngle = carrierAngle + 360 * i / count
    // External mesh: S*(phi-theta_s) + P*(phi+180-theta_p) = 180 mod 360.
    // Combined with a stationary ring this is the Willis relation at every pose.
    const planetAngle = (1 + sunTeeth / planetTeeth) * orbitAngle - sunTeeth / planetTeeth * sunAngle + 180 - 180 / planetTeeth
    planetAngles.push(planetAngle)
    const polar = angle(orbitAngle) * degrees
    place(`planet_${i + 1}`, 'planet', planet, planetOptions, orbitRadius * Math.cos(polar), orbitRadius * Math.sin(polar), planetAngle)
  }
  // Separate top-level solids keep each printable part addressable and avoid
  // joining the teeth at a contact point through an implicit boolean operation.
  const source = parts.map(part => part.source).join('\n')
  if (source.length > 240000) throw new Error('Planetary generated source exceeds the geometry budget; reduce tooth counts, planet_count or flank_segments.')
  return {
    source,
    parts,
    report: {
      generator: 'planetary_gears', construction: 'unshifted_involute_spur_gearset',
      sun_teeth: sunTeeth, planet_teeth: planetTeeth, ring_teeth: ringTeeth, planet_count: count,
      module_mm: options.module, pressure_angle_deg: options.pressure_angle, thickness_mm: options.thickness,
      orbit_radius_mm: orbitRadius, adjacent_planet_tip_gap_mm: adjacentTipGap,
      internal_involute_contact_margin_mm: internalContactMargin,
      equal_spacing_assembly_index: (sunTeeth + ringTeeth) / count,
      fixed_member: 'ring', input_member: 'sun', output_member: 'carrier',
      sun_to_carrier_ratio: 1 + ringTeeth / sunTeeth,
      carrier_angle_deg: carrierAngle, sun_angle_deg: sunAngle, ring_angle_deg: ringAngle,
      planet_angles_deg: planetAngles,
      backlash_per_gear_mm: options.backlash, pair_circumferential_backlash_mm: 2 * options.backlash,
      generated_parts: parts.map(({ id, role, pose }) => ({ id, role, pose })),
      omitted_components: ['carrier_plate', 'axles', 'bearings', 'housing', 'fasteners'],
      load_capacity: 'not_evaluated', manufacturing_tolerance_class: 'not_assigned',
      profile_note: 'Sampled involute flanks with radial root transitions; no cutter-generated trochoid or root fillet.',
    },
  }
}
