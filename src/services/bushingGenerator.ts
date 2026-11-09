import { z } from 'zod/v4'

export const BUSHING_DEFAULTS = {
  thread_mode: 0,
  length: 20,
  inner_diameter: 8,
  inner_pitch: 1.25,
  inner_starts: 1,
  inner_left_handed: false,
  inner_thread_depth: 0,
  outer_diameter: 14,
  outer_pitch: 1.5,
  outer_starts: 1,
  outer_left_handed: false,
  outer_shape: 0,
  wrench_flat_depth: 1.2,
  knurl_count: 16,
  knurl_depth: 0.5,
  flange: true,
  flange_diameter: 20,
  flange_thickness: 3,
  flange_shape: 0,
  flange_holes: 0,
  flange_hole_diameter: 3.2,
  blind_bottom: 0,
  chamfer: 0.8,
  clearance: 0.2,
  cross_hole_diameter: 0,
  cross_hole_z: 0,
  split_slot_width: 0,
  segments: 36,
}

const schema = z.object({
  thread_mode: z.number().int().min(0).max(2).default(0),
  length: z.number().finite().min(2).max(200).default(20),
  inner_diameter: z.number().finite().min(1.6).max(150).default(8),
  inner_pitch: z.number().finite().min(0.25).max(10).default(1.25),
  inner_starts: z.number().int().min(1).max(4).default(1),
  inner_left_handed: z.boolean().default(false),
  inner_thread_depth: z.number().finite().min(0).max(200).default(0),
  outer_diameter: z.number().finite().min(3).max(200).default(14),
  outer_pitch: z.number().finite().min(0.25).max(10).default(1.5),
  outer_starts: z.number().int().min(1).max(4).default(1),
  outer_left_handed: z.boolean().default(false),
  outer_shape: z.number().int().min(0).max(4).default(0),
  wrench_flat_depth: z.number().finite().min(0.2).max(30).default(1.2),
  knurl_count: z.number().int().min(4).max(64).default(16),
  knurl_depth: z.number().finite().min(0.1).max(5).default(0.5),
  flange: z.boolean().default(true),
  flange_diameter: z.number().finite().min(4).max(260).default(20),
  flange_thickness: z.number().finite().min(0.8).max(80).default(3),
  flange_shape: z.number().int().min(0).max(2).default(0),
  flange_holes: z.number().int().min(0).max(12).default(0),
  flange_hole_diameter: z.number().finite().min(1).max(30).default(3.2),
  blind_bottom: z.number().finite().min(0).max(100).default(0),
  chamfer: z.number().finite().min(0).max(10).default(0.8),
  clearance: z.number().finite().min(0).max(1.5).default(0.2),
  cross_hole_diameter: z.number().finite().min(0).max(80).default(0),
  cross_hole_z: z.number().finite().min(0).max(200).default(0),
  split_slot_width: z.number().finite().min(0).max(40).default(0),
  segments: z.number().int().min(16).max(72).default(36),
}).strict()

export type BushingOptions = z.infer<typeof schema>

export interface BushingParameterSuggestion {
  title: string
  description: string
  changes: Record<string, number>
}

const ISO_PROFILE_FACTOR = (5 * Math.sqrt(3)) / 16 // ~0.541265877

function standardMetricPitch(diameter: number): number {
  if (diameter <= 2) return 0.4
  if (diameter <= 2.6) return 0.45
  if (diameter <= 3.2) return 0.5
  if (diameter <= 4.2) return 0.7
  if (diameter <= 5.2) return 0.8
  if (diameter <= 6.5) return 1.0
  if (diameter <= 9) return 1.25
  if (diameter <= 11) return 1.5
  if (diameter <= 13) return 1.75
  if (diameter <= 17) return 2.0
  if (diameter <= 23) return 2.5
  if (diameter <= 28) return 3.0
  return 3.5
}

function round2(v: number): number {
  return Math.round(v * 100) / 100
}

export function computeBushingParameterSuggestions(
  changedKey: string,
  rawValues: Record<string, number | boolean | string>,
): BushingParameterSuggestion[] {
  const innerD = Number(rawValues.inner_diameter)
  const outerD = Number(rawValues.outer_diameter)
  const mode = Number(rawValues.thread_mode ?? 0)
  const innerPitch = Number(rawValues.inner_pitch ?? 1.25)
  const outerPitch = Number(rawValues.outer_pitch ?? 1.5)
  const clearance = Number(rawValues.clearance ?? 0.2)
  const flange = Boolean(rawValues.flange)
  const flangeD = Number(rawValues.flange_diameter ?? 20)
  if (!Number.isFinite(innerD) || !Number.isFinite(outerD)) return []

  const suggestions: BushingParameterSuggestion[] = []
  const hasInnerThread = mode === 0 || mode === 2
  const hasOuterThread = mode === 1 || mode === 2

  const rInnerMax = innerD / 2 + clearance / 2
  const outerThreadDepth = hasOuterThread ? ISO_PROFILE_FACTOR * outerPitch + clearance / 2 : 0
  const minSafeOuterD = round2(2 * (rInnerMax + outerThreadDepth + 2.0))

  if (changedKey === 'inner_diameter' || outerD < minSafeOuterD) {
    const recPitch = standardMetricPitch(innerD)
    const targetOuterD = Math.max(outerD, minSafeOuterD)
    const targetFlangeD = flange ? Math.max(flangeD, round2(targetOuterD + 6)) : flangeD
    const changes: Record<string, number> = {}
    if (targetOuterD !== outerD) changes.outer_diameter = targetOuterD
    if (hasInnerThread && changedKey === 'inner_diameter' && Math.abs(innerPitch - recPitch) > 0.01) {
      changes.inner_pitch = recPitch
    }
    if (flange && targetFlangeD !== flangeD) changes.flange_diameter = targetFlangeD
    if (Object.keys(changes).length > 0) {
      const summary = Object.entries(changes).map(([k, v]) => `${k} = ${v} мм`).join(', ')
      suggestions.push({
        title: `Подобрать сопряжённые размеры (${summary})`,
        description: `Для внутреннего диаметра ${innerD} мм можно автоматически обновить наружный диаметр, шаг резьбы и буртик для сохранения прочной стенки:`,
        changes,
      })
    }
  }

  if (changedKey === 'outer_diameter' && flange && flangeD <= outerD + 1) {
    const nextFlange = round2(outerD + 6)
    suggestions.push({
      title: `Увеличить диаметр буртика (flange_diameter = ${nextFlange} мм)`,
      description: `Буртик должен выступать за наружный диаметр втулки (${outerD} мм):`,
      changes: { flange_diameter: nextFlange },
    })
  }

  return suggestions
}

const ANGLES_24 = Array.from({ length: 24 }, (_, i) => i * 15)
const ANGLES_48 = Array.from({ length: 48 }, (_, i) => i * 7.5)

/**
 * Generates a parametric OpenSCAD (.scad) threaded bushing / sleeve / adapter
 * supporting internal threads, external threads, dual inner+outer reducer threads,
 * outer body shapes (round, hex, wrench flats, square, knurled), flange/collar with
 * mounting holes, blind bottom, cross-hole, and split collet slot.
 */
export function generateBushing(input: unknown = BUSHING_DEFAULTS): string {
  const p = schema.parse(input)

  const hasInnerThread = p.thread_mode === 0 || p.thread_mode === 2
  const hasOuterThread = p.thread_mode === 1 || p.thread_mode === 2

  if (hasInnerThread && p.inner_diameter < 2 * p.inner_pitch) {
    throw Error('Внутренний диаметр должен быть минимум вдвое больше шага внутренней резьбы / inner_diameter must be at least 2 * inner_pitch')
  }
  if (hasOuterThread && p.outer_diameter < 2 * p.outer_pitch) {
    throw Error('Наружный диаметр должен быть минимум вдвое больше шага наружной резьбы / outer_diameter must be at least 2 * outer_pitch')
  }

  const rInnerMax = p.inner_diameter / 2 + p.clearance / 2
  let rOuterMin = p.outer_diameter / 2
  if (hasOuterThread) {
    rOuterMin -= p.clearance / 2 + ISO_PROFILE_FACTOR * p.outer_pitch
  } else if (p.outer_shape === 2) {
    rOuterMin -= p.wrench_flat_depth
  } else if (p.outer_shape === 4) {
    rOuterMin -= p.knurl_depth
  }

  const minWall = rOuterMin - rInnerMax
  if (minWall < 0.8) {
    throw Error(
      `Недостаточная остаточная толщина стенки втулки (${minWall.toFixed(2)} мм < 0.80 мм): увеличьте наружный диаметр или уменьшите внутренний диаметр / Insufficient minimum wall thickness (${minWall.toFixed(2)} mm < 0.80 mm): increase outer_diameter or reduce inner_diameter`,
    )
  }

  if (p.blind_bottom > 0 && p.blind_bottom >= p.length - 1) {
    throw Error('Толщина глухого дна должна оставлять минимум 1 мм глубины канала / blind_bottom must leave at least 1 mm of cavity depth')
  }
  if (p.inner_thread_depth > 0 && p.inner_thread_depth > p.length - p.blind_bottom) {
    throw Error('Глубина внутренней резьбы превышает доступную длину канала / inner_thread_depth exceeds available cavity length')
  }

  if (p.flange) {
    if (p.flange_thickness >= p.length - 0.5) {
      throw Error('Толщина буртика должна быть меньше общей длины втулки / flange_thickness must be less than total bushing length')
    }
    if (p.flange_diameter <= p.outer_diameter + 0.4) {
      throw Error('Диаметр буртика должен быть больше наружного диаметра втулки / flange_diameter must be greater than outer_diameter')
    }
    if (p.flange_holes > 0) {
      const rimSpan = (p.flange_diameter - p.outer_diameter) / 2
      if (p.flange_hole_diameter > rimSpan - 0.8) {
        throw Error('Крепёжные отверстия не помещаются на поле буртика: увеличьте диаметр буртика или уменьшите диаметр отверстий / Flange holes do not fit on the flange rim: increase flange_diameter or reduce flange_hole_diameter')
      }
    }
  }

  if (p.cross_hole_diameter > 0) {
    if (p.cross_hole_diameter >= p.length - 1 || p.cross_hole_diameter >= p.outer_diameter * 0.75) {
      throw Error('Диаметр поперечного отверстия слишком велик для габаритов втулки / cross_hole_diameter is too large for the bushing dimensions')
    }
    const zCenter = p.cross_hole_z > 0 ? p.cross_hole_z : p.flange ? (p.length + p.flange_thickness) / 2 : p.length / 2
    if (zCenter - p.cross_hole_diameter / 2 < 0.4 || zCenter + p.cross_hole_diameter / 2 > p.length - 0.4) {
      throw Error('Поперечное отверстие выходит за торцы втулки: скорректируйте высоту или диаметр отверстия / Cross-hole extends beyond bushing ends: adjust cross_hole_z or cross_hole_diameter')
    }
  }

  if (p.split_slot_width > 0 && p.split_slot_width >= p.inner_diameter * 0.85) {
    throw Error('Ширина продольного разреза слишком велика относительно внутреннего отверстия / split_slot_width is too wide relative to inner_diameter')
  }

  const thExpr = (a: number) => {
    const u = `((${a} * starts / 360) - floor(${a} * starts / 360))`
    const w = `min(1, max(0, (1 - abs(2 * ${u} - 1) - 0.125) / 0.75))`
    const r = `(r_min + (r_max - r_min) * ${w})`
    return `[${r} * cos(${a}), ${r} * sin(${a})]`
  }
  const knExpr = (a: number) => {
    const u = `((${a} * knurl_count / 360) - floor(${a} * knurl_count / 360))`
    const w = `(1 - abs(2 * ${u} - 1))`
    const r = `(d / 2 - k_depth + k_depth * ${w})`
    return `[${r} * cos(${a}), ${r} * sin(${a})]`
  }
  const threadPolygonPoints = ANGLES_24.map(thExpr).join(', ')
  const knurlPolygonPoints = ANGLES_48.map(knExpr).join(', ')

  return `// Parametric threaded bushing / sleeve / reducer adapter (mm).
// thread_mode: 0 = internal thread, 1 = external thread + smooth bore, 2 = internal + external threads.
// outer_shape: 0 = round, 1 = hex across flats, 2 = two wrench flats, 3 = square, 4 = knurled splines.
// flange_shape: 0 = round, 1 = hex across flats, 2 = square.
thread_mode = ${p.thread_mode}; // [0, 1, 2]
length = ${p.length}; // [2:0.5:200]
inner_diameter = ${p.inner_diameter}; // [1.6:0.2:150]
inner_pitch = ${p.inner_pitch}; // [0.25:0.05:10]
inner_starts = ${p.inner_starts}; // [1:1:4]
inner_left_handed = ${p.inner_left_handed};
inner_thread_depth = ${p.inner_thread_depth}; // [0:0.5:200]
outer_diameter = ${p.outer_diameter}; // [3:0.2:200]
outer_pitch = ${p.outer_pitch}; // [0.25:0.05:10]
outer_starts = ${p.outer_starts}; // [1:1:4]
outer_left_handed = ${p.outer_left_handed};
outer_shape = ${p.outer_shape}; // [0, 1, 2, 3, 4]
wrench_flat_depth = ${p.wrench_flat_depth}; // [0.2:0.1:30]
knurl_count = ${p.knurl_count}; // [4:1:64]
knurl_depth = ${p.knurl_depth}; // [0.1:0.05:5]
flange = ${p.flange};
flange_diameter = ${p.flange_diameter}; // [4:0.5:260]
flange_thickness = ${p.flange_thickness}; // [0.8:0.2:80]
flange_shape = ${p.flange_shape}; // [0, 1, 2]
flange_holes = ${p.flange_holes}; // [0:1:12]
flange_hole_diameter = ${p.flange_hole_diameter}; // [1:0.2:30]
blind_bottom = ${p.blind_bottom}; // [0:0.5:100]
chamfer = ${p.chamfer}; // [0:0.1:10]
clearance = ${p.clearance}; // [0:0.02:1.5]
cross_hole_diameter = ${p.cross_hole_diameter}; // [0:0.2:80]
cross_hole_z = ${p.cross_hole_z}; // [0:0.5:200]
split_slot_width = ${p.split_slot_width}; // [0:0.2:40]
segments = ${p.segments}; // [16:4:72]

$fn = segments;
iso_depth_factor = 5 * sqrt(3) / 16;
has_inner_thread = thread_mode == 0 || thread_mode == 2;
has_outer_thread = thread_mode == 1 || thread_mode == 2;

r_in_major = inner_diameter / 2 + clearance / 2;
r_in_minor = has_inner_thread ? max(0.3, r_in_major - iso_depth_factor * inner_pitch) : r_in_major;

r_out_major = max(r_in_major + 0.8, outer_diameter / 2 - (has_outer_thread ? clearance / 2 : 0));
r_out_minor = has_outer_thread ? max(r_in_major + 0.6, r_out_major - iso_depth_factor * outer_pitch) : r_out_major;

eff_chamfer = min(chamfer, length * 0.22, (r_out_major - r_in_minor) * 0.45);
flange_h = flange ? min(flange_thickness, length - 0.5) : 0;
flange_d = flange ? max(flange_diameter, outer_diameter + 0.4) : outer_diameter;

// Single simple closed 2D polygon without central pinch vertex.
module helical_profile_2d(r_min, r_max, starts) {
  polygon([${threadPolygonPoints}]);
}

module unthreaded_outer_2d(d, shape) {
  if (shape == 1) {
    circle(r = d / sqrt(3), $fn = 6);
  } else if (shape == 2) {
    flat_span = max(2 * r_in_major + 1.2, d - 2 * wrench_flat_depth);
    intersection() {
      circle(d = d, $fn = segments);
      square([flat_span, d + 2], center = true);
    }
  } else if (shape == 3) {
    square([d, d], center = true);
  } else if (shape == 4) {
    k_depth = min(knurl_depth, max(0.1, (d / 2 - r_in_major - 0.8)));
    polygon([${knurlPolygonPoints}]);
  } else {
    circle(d = d, $fn = segments);
  }
}

module flange_2d(d, shape) {
  if (shape == 1) {
    circle(r = d / sqrt(3), $fn = 6);
  } else if (shape == 2) {
    square([d, d], center = true);
  } else {
    circle(d = d, $fn = segments);
  }
}

module body_chamfer_envelope(h, r_bound, ch_bot, ch_top) {
  hull() {
    if (ch_bot > 0) {
      cylinder(h = 0.02, r = max(0.5, r_bound - ch_bot), $fn = segments);
      translate([0, 0, ch_bot]) cylinder(h = 0.02, r = r_bound, $fn = segments);
    } else {
      cylinder(h = 0.02, r = r_bound, $fn = segments);
    }
    if (ch_top > 0) {
      translate([0, 0, max(0.04, h - ch_top)]) cylinder(h = 0.02, r = r_bound, $fn = segments);
      translate([0, 0, max(0.06, h - 0.02)]) cylinder(h = 0.02, r = max(0.5, r_bound - ch_top), $fn = segments);
    } else {
      translate([0, 0, max(0.04, h - 0.02)]) cylinder(h = 0.02, r = r_bound, $fn = segments);
    }
  }
}

module outer_shaft() {
  if (has_outer_thread) {
    out_lead = outer_starts * outer_pitch;
    out_dir = outer_left_handed ? 1 : -1;
    out_slices = min(32, max(10, ceil(length / out_lead * 6)));
    linear_extrude(height = length, twist = out_dir * 360 * length / out_lead, slices = out_slices)
      helical_profile_2d(r_out_minor, r_out_major, outer_starts);
  } else {
    r_bound = outer_shape == 1 ? outer_diameter / sqrt(3) : (outer_shape == 3 ? outer_diameter / sqrt(2) : outer_diameter / 2);
    if (eff_chamfer > 0 && outer_shape != 4) {
      intersection() {
        linear_extrude(height = length)
          unthreaded_outer_2d(outer_diameter, outer_shape);
        body_chamfer_envelope(length, r_bound, flange ? 0 : eff_chamfer, eff_chamfer);
      }
    } else {
      linear_extrude(height = length)
        unthreaded_outer_2d(outer_diameter, outer_shape);
    }
  }
}

module flange_collar() {
  if (flange && flange_h > 0) {
    linear_extrude(height = flange_h)
      difference() {
        flange_2d(flange_d, flange_shape);
        if (flange_holes > 0) {
          bolt_circle_r = (flange_d + outer_diameter) / 4;
          for (k = [0 : flange_holes - 1]) {
            ang = k * 360 / flange_holes;
            translate([bolt_circle_r * cos(ang), bolt_circle_r * sin(ang)])
              circle(d = flange_hole_diameter, $fn = max(12, floor(segments / 2)));
          }
        }
      }
  }
}

module inner_cavity() {
  z_bot = blind_bottom > 0 ? min(length - 1, blind_bottom) : -0.2;
  cav_h = length + 0.4 - z_bot;
  if (has_inner_thread) {
    th_depth = inner_thread_depth > 0 ? min(length - max(0, z_bot), inner_thread_depth) : 0;
    z_th_start = th_depth > 0 ? max(z_bot, length - th_depth) : z_bot;
    th_h = length + 0.2 - z_th_start;
    in_lead = inner_starts * inner_pitch;
    in_dir = inner_left_handed ? 1 : -1;
    in_slices = min(32, max(10, ceil(th_h / in_lead * 6)));
    if (z_th_start > z_bot) {
      translate([0, 0, z_bot])
        cylinder(h = z_th_start - z_bot + 0.2, r = r_in_minor, $fn = max(16, floor(segments / 2)));
    }
    translate([0, 0, z_th_start])
      linear_extrude(height = th_h, twist = in_dir * 360 * th_h / in_lead, slices = in_slices)
        helical_profile_2d(r_in_minor, r_in_major, inner_starts);
  } else {
    translate([0, 0, z_bot])
      cylinder(h = cav_h, r = r_in_minor, $fn = segments);
  }

  // Top lead-in countersink chamfer
  if (eff_chamfer > 0 && !has_inner_thread) {
    translate([0, 0, length - eff_chamfer])
      cylinder(h = eff_chamfer + 0.1, r1 = r_in_minor, r2 = r_in_major + eff_chamfer + 0.1, $fn = segments);
  }
}

module secondary_cuts() {
  span = max(outer_diameter, flange_d) * 1.6;
  if (cross_hole_diameter > 0) {
    z_c = cross_hole_z > 0 ? cross_hole_z : (flange ? (length + flange_h) / 2 : length / 2);
    translate([0, 0, z_c])
      rotate([0, 90, 0])
        cylinder(h = span, d = cross_hole_diameter, center = true, $fn = max(12, floor(segments / 2)));
  }
  if (split_slot_width > 0) {
    translate([0, -split_slot_width / 2, -0.2])
      cube([span, split_slot_width, length + 0.4]);
  }
}

color([0.28, 0.56, 0.92])
difference() {
  union() {
    outer_shaft();
    flange_collar();
  }
  inner_cavity();
  secondary_cuts();
}
`
}
