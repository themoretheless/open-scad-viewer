export interface ThreadOptions {
  diameter: number
  pitch: number
  length: number
  internal: boolean
  wall: number
  clearance: number
  starts: number
  left_handed: boolean
  segments_per_turn: number
}

const PROFILE_BREAKS = [1 / 16, 3 / 8, 5 / 8, 15 / 16]
const MAX_TRIANGLES = 3500
type Point2 = [number, number]

/**
 * ISO 68-1 basic axial profile: 60 degrees, H1=5*sqrt(3)*P/16,
 * crest width P/8, root width P/4. This is a printable faceted basic
 * profile, without the design-profile root rounding or ISO 965 tolerances.
 * https://www.iso.org/standard/85107.html
 */
export function threadRadiusAt(options: ThreadOptions, angleRadians: number, z: number): number {
  const phase = z / options.pitch - (options.left_handed ? -1 : 1) * options.starts * angleRadians / (2 * Math.PI)
  const wrapped = phase - Math.floor(phase)
  const distance = Math.min(wrapped, 1 - wrapped)
  const depth = Math.sqrt(3) * options.pitch * Math.max(0, Math.min(5 / 16, distance - 1 / 16))
  return options.diameter / 2 - depth + (options.internal ? 1 : -1) * options.clearance / 2
}

function validate(options: ThreadOptions) {
  for (const name of ['diameter', 'pitch', 'length', 'wall'] as const) {
    if (!Number.isFinite(options[name]) || options[name] < 0.01 || options[name] > 10000) {
      throw new Error(`Thread ${name} must be between 0.01 and 10000 mm.`)
    }
  }
  if (options.diameter < 2 * options.pitch) throw new Error('Thread diameter must be at least twice the pitch.')
  if (options.length < options.pitch / 4 || options.length > options.pitch * 64) throw new Error('Thread length must be between 0.25 and 64 pitches.')
  if (!Number.isFinite(options.clearance) || options.clearance < 0 || options.clearance >= 5 * Math.sqrt(3) * options.pitch / 16) {
    throw new Error('Thread radial clearance must be nonnegative and smaller than the thread depth.')
  }
  if (!Number.isInteger(options.starts) || options.starts < 1 || options.starts > 4) throw new Error('Thread starts must be an integer from 1 to 4.')
  if (!Number.isInteger(options.segments_per_turn) || options.segments_per_turn < 16 || options.segments_per_turn > 96 || options.segments_per_turn < 8 * options.starts) {
    throw new Error('Thread segments_per_turn must be an integer from 16 to 96, and at least eight times starts.')
  }
  if (typeof options.internal !== 'boolean' || typeof options.left_handed !== 'boolean') throw new Error('Thread internal and left_handed must be boolean.')
}

function clip(poly: Point2[], phase: (p: Point2) => number, bound: number, lower: boolean): Point2[] {
  const result: Point2[] = []
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!, b = poly[(i + 1) % poly.length]!
    const da = (phase(a) - bound) * (lower ? 1 : -1), db = (phase(b) - bound) * (lower ? 1 : -1)
    const insideA = da >= -1e-11, insideB = db >= -1e-11
    if (insideA) result.push(a)
    if (insideA !== insideB) {
      const f = da / (da - db)
      result.push([a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])])
    }
  }
  return result
}

/** A closed, phase-aligned helical rod or internally threaded cylindrical sleeve. */
export function buildModelGraphThread(options: ThreadOptions) {
  validate(options)
  const { pitch, length, starts, internal, segments_per_turn: segments } = options
  const turns = length / pitch, direction = options.left_handed ? -1 : 1
  const slope = direction * starts
  // Include every intersection of a profile corner with either cap. The same
  // angular columns then close the caps and outer sleeve without T-junctions.
  const angles = Array.from({ length: segments + 1 }, (_, i) => i / segments)
  for (const y of [0, turns]) for (let k = Math.floor(y) - starts - 1; k <= Math.ceil(y) + starts + 1; k++) {
    for (const corner of PROFILE_BREAKS) {
      const t = (y - k - corner) / slope
      if (t > 1e-10 && t < 1 - 1e-10) angles.push(t)
    }
  }
  angles.sort((a, b) => a - b)
  const columns = angles.filter((t, i) => !i || t - angles[i - 1]! > 1e-10)
  const points: number[][] = [], faces: number[][] = [], vertexIds = new Map<string, number>()
  const round = (n: number) => Number(n.toPrecision(13))
  const point = (t: number, y: number, outer = false) => {
    if (Math.abs(t - 1) < 1e-10 || Math.abs(t) < 1e-10) t = 0
    if (Math.abs(y) < 1e-10) y = 0
    if (Math.abs(y - turns) < 1e-10) y = turns
    const key = `${outer ? 'o' : 'i'}:${t.toFixed(11)}:${y.toFixed(11)}`
    const existing = vertexIds.get(key)
    if (existing !== undefined) return existing
    const angle = 2 * Math.PI * t
    const radius = outer ? options.diameter / 2 + options.clearance / 2 + options.wall : threadRadiusAt(options, angle, y * pitch)
    const id = points.length
    points.push([round(radius * Math.cos(angle)), round(radius * Math.sin(angle)), round(y * pitch)])
    vertexIds.set(key, id)
    return id
  }
  const triangle = (a: number, b: number, c: number, reverse = false) => {
    if (a === b || b === c || c === a) return
    faces.push(reverse ? [a, c, b] : [a, b, c])
    if (faces.length > MAX_TRIANGLES) throw new Error(`Thread mesh exceeds ${MAX_TRIANGLES} triangles; shorten length, increase pitch, or reduce segments_per_turn.`)
  }
  const phase = ([t, y]: Point2) => y - slope * t
  for (let column = 0; column < columns.length - 1; column++) {
    const a = columns[column]!, b = columns[column + 1]!
    const min = Math.min(phase([a, 0]), phase([b, 0])), max = Math.max(phase([a, turns]), phase([b, turns]))
    const breaks: number[] = []
    for (let k = Math.floor(min) - 1; k <= Math.ceil(max); k++) for (const corner of PROFILE_BREAKS) breaks.push(k + corner)
    for (let j = 0; j < breaks.length - 1; j++) {
      const lo = breaks[j]!, hi = breaks[j + 1]!
      if (hi <= min + 1e-11 || lo >= max - 1e-11) continue
      const poly = clip(clip([[a, 0], [b, 0], [b, turns], [a, turns]], phase, lo, true), phase, hi, false)
      const ids = poly.map(([t, y]) => point(Math.abs(t - a) < 1e-10 ? a : Math.abs(t - b) < 1e-10 ? b : t, y))
        .filter((id, i, all) => id !== all[(i + all.length - 1) % all.length])
      // OpenSCAD's exterior face convention is clockwise, hence reverse for rod.
      for (let k = 1; k < ids.length - 1; k++) triangle(ids[0]!, ids[k]!, ids[k + 1]!, !internal)
    }
  }
  const bottomCenter = points.length, topCenter = bottomCenter + 1
  if (!internal) points.push([0, 0, 0], [0, 0, length])
  for (let i = 0; i < columns.length - 1; i++) {
    const a = columns[i]!, b = columns[i + 1]!
    const a0 = point(a, 0), b0 = point(b, 0), a1 = point(a, turns), b1 = point(b, turns)
    if (!internal) {
      triangle(bottomCenter, a0, b0)
      triangle(topCenter, b1, a1)
    } else {
      const ao = point(a, 0, true), bo = point(b, 0, true), at = point(a, turns, true), bt = point(b, turns, true)
      triangle(ao, b0, a0); triangle(ao, bo, b0)
      triangle(at, a1, b1); triangle(at, b1, bt)
      triangle(ao, bt, bo); triangle(ao, at, bt)
    }
  }
  const source = `polyhedron(points=${JSON.stringify(points)},faces=${JSON.stringify(faces)},convexity=10);`
  if (source.length > 200000) throw new Error('Thread generated source exceeds 200000 characters; reduce segments_per_turn or length.')
  const depth = 5 * Math.sqrt(3) * pitch / 16
  const radialOffset = (internal ? 1 : -1) * options.clearance / 2
  return {
    source,
    mesh: {positions: points.flat(), indices: faces.flatMap(face => [...face].reverse())},
    report: {
      generator: 'own_helical_thread', profile: 'metric_60_degree_basic_faceted', units: 'mm',
      nominal_diameter_mm: options.diameter, pitch_mm: pitch, lead_mm: starts * pitch,
      length_mm: length, starts, handedness: options.left_handed ? 'left' : 'right',
      internal, radial_clearance_mm: options.clearance,
      clearance_convention: 'External radius decreases by clearance/2; internal cavity radius increases by clearance/2. Matching settings give nominal radial clearance.',
      major_diameter_mm: options.diameter + 2 * radialOffset,
      minor_diameter_mm: options.diameter - 2 * depth + 2 * radialOffset,
      outer_diameter_mm: internal ? options.diameter + options.clearance + 2 * options.wall : options.diameter - options.clearance,
      minimum_wall_mm: internal ? options.wall : null,
      profile_depth_mm: depth, flank_included_angle_degrees: 60,
      segments_per_turn: segments, angular_columns: columns.length - 1,
      vertex_count: points.length, triangle_count: faces.length,
      tolerance_class: null,
      limitations: ['Faceted basic profile, without root rounding, lead-in chamfers, runout or a tolerance class.', 'Matching pitch, starts, handedness and angular phase are required; printing fit is not certified.'],
    },
  }
}
