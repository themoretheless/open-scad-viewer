import { boxGeometrySource } from './boxGeometrySource'
import { z } from 'zod/v4'

export const BOX_DEFAULTS = {
  width: 120,
  depth: 75,
  height: 80,
  wall: 3,
  floor: 4,
  corner_radius: 0,
  front_angle: 0,
  dividers: 0,
  hooks: true,
  bend_radius: 0,
  bend_reverse: false,
  bend_segments: 24,
  porous: true,
  organic: true,
  print_optimized: true,
  overhang_angle: 45,
  cell_size: 12,
  cell_radius: 2,
  rib: 2.4,
  stress_gradient: 1,
  mesh_density: 1,
  pattern_seed: 1,
  hole_diameter: 6,
  hole_pitch: 16,
  floor_mesh: true,
  floor_cell_size: 12,
  floor_rib: 2.4,
  drain: false,
  hook_count: 2,
  hook_width: 10,
  hook_thickness: 3,
  hook_gap: 6,
  hook_drop: 16,
  hook_spacing: 0,
  hook_chamfer: false,
  hook_mount: 0,
  hook_tolerance: 0.2,
}
const dimension = z.number().finite().min(0.8).max(500)
const schema = z
  .object({
    width: dimension,
    depth: dimension,
    height: dimension,
    wall: dimension,
    floor: dimension,
    porous: z.boolean(),
    corner_radius: z.number().finite().min(0).max(250).default(0),
    front_angle: z.number().finite().min(-60).max(60).default(0),
    bend_radius: z.number().finite().min(0).max(100000).default(0),
    bend_reverse: z.boolean().default(false),
    bend_segments: z.number().int().min(4).max(128).default(24),
    organic: z.boolean().default(true),
    cell_size: dimension.min(1).default(12),
    cell_radius: z.number().finite().min(0).max(50).default(2),
    rib: dimension.default(2.4),
    stress_gradient: z.number().finite().min(0).max(2).default(1),
    mesh_density: z.number().finite().min(0.5).max(2).default(1),
    print_optimized: z.boolean().default(true),
    overhang_angle: z.number().finite().min(10).max(80).default(45),
    pattern_seed: z.number().int().min(0).max(100000).default(1),
    hole_diameter: dimension,
    hole_pitch: dimension,
    floor_mesh: z.boolean().default(false),
    floor_cell_size: dimension.min(1).default(12),
    floor_rib: dimension.default(2.4),
    dividers: z.number().int().min(0).max(8),
    drain: z.boolean(),
    hooks: z.boolean(),
    hook_count: z.number().int().min(1).max(6),
    hook_width: dimension,
    hook_thickness: dimension.default(3),
    hook_gap: dimension,
    hook_drop: dimension,
    hook_spacing: z.number().finite().min(0).max(500).default(0),
    hook_chamfer: z.boolean().default(false),
    hook_mount: z.number().int().min(0).max(2).default(0),
    hook_tolerance: z.number().finite().min(0.05).max(0.8).default(0.2),
  })
  .strict()

/** Dimensions are external; organic walls retain a solid border and hook pads. */
export function generateBox(input: unknown = BOX_DEFAULTS): string {
  const normalized =
    input &&
    typeof input === 'object' &&
    'bend_radius' in input &&
    Number((input as Record<string, unknown>).bend_radius) <= 0
      ? {
          ...(input as Record<string, unknown>),
          bend_segments: Math.max(4, Math.trunc(Number((input as Record<string, unknown>).bend_segments) || 24)),
        }
      : input
  const p = schema.parse(normalized)
  const { width: w, depth: d, height: h, wall: t, floor: f } = p
  if (w <= 4 * t || d <= 4 * t || h <= f + 2 * t)
    throw Error(
      'Размеры коробки слишком малы для выбранных стенок и дна / Box dimensions are too small for the wall and floor',
    )
  if ((w - 2 * Math.max(t, p.corner_radius) - p.dividers * t) / (p.dividers + 1) < 4)
    throw Error('Между перегородками должно оставаться минимум 4 мм / Compartments must be at least 4 mm wide')
  if (((p.porous && !p.organic) || (p.drain && !p.floor_mesh)) && p.hole_pitch < p.hole_diameter + 1.2)
    throw Error('Между отверстиями нужно минимум 1.2 мм материала / Leave at least 1.2 mm between holes')
  if (p.hooks) {
    const span = w - 2 * p.corner_radius
    const besselSpan = Math.max(
      0,
      w - 2 * Math.max(p.corner_radius + p.hook_width / 2, p.corner_radius + 0.2071 * span),
    )
    const step =
      p.hook_spacing ||
      (p.hook_count === 1
        ? 0
        : p.dividers === p.hook_count && p.hook_count > 1
          ? (w - 2 * Math.max(t, p.corner_radius)) / (p.dividers + 1)
          : besselSpan >= (p.hook_count - 1) * p.hook_width * 1.15
            ? besselSpan / (p.hook_count - 1)
            : span / (p.hook_count + 1))
    const left = (w - step * (p.hook_count - 1) - p.hook_width) / 2
    if (p.hook_drop + p.hook_thickness > h || left < p.corner_radius || (p.hook_count > 1 && p.hook_width >= step))
      throw Error(
        'Крючки не помещаются или перекрываются: уменьшите размеры, количество или шаг / Hooks do not fit or overlap; adjust their dimensions, count or spacing',
      )
  }
  if (p.organic && p.porous && p.cell_size / p.mesh_density < p.rib * 0.4)
    throw Error(
      'Размер ячейки с учётом плотности должен быть не меньше перемычки / Effective cell size must be at least the rib width',
    )
  if (p.floor_mesh && p.floor_cell_size / p.mesh_density < p.floor_rib * 0.4)
    throw Error(
      'Размер ячейки дна с учётом плотности должен быть не меньше перемычки / Effective floor cell size must be at least the rib width',
    )
  if (p.floor_mesh && p.floor_cell_size / p.mesh_density > Math.min(w, d) - 2 * Math.max(t, p.floor_rib * 1.5))
    throw Error(
      'Ячейка дна не помещается в коробку: уменьшите её размер или увеличьте плотность / Floor cell does not fit; reduce its size or increase density',
    )
  const topDepth = d + (h - f) * Math.tan((p.front_angle * Math.PI) / 180)
  if (topDepth <= 4 * t)
    throw Error(
      'Наклон закрывает коробку: увеличьте глубину или уменьшите угол / Tilt leaves insufficient opening depth',
    )
  if (2 * p.corner_radius >= Math.min(w, d, topDepth))
    throw Error('Радиус не помещается в габариты коробки / Corner radius does not fit the box dimensions')
  if (p.bend_radius > 0) {
    const attachedOut = p.hooks && p.hook_mount === 0 ? p.hook_gap + p.hook_thickness : 0
    const minRadius = Math.ceil(
      Math.max(w / (2 * Math.PI) + 1, (p.bend_reverse ? Math.max(d, topDepth) : attachedOut) + 1),
    )
    if (w / p.bend_radius >= 2 * Math.PI)
      throw Error(
        `Дуга замыкается: для ширины ${w} мм укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Increase bend radius to at least ${minRadius} мм`,
      )
    if (p.bend_radius <= (p.bend_reverse ? Math.max(d, topDepth) : attachedOut))
      throw Error(
        `Изгиб пересекает ось: укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Bend reaches its axis; increase radius to at least ${minRadius} mm`,
      )
  }
  if (p.print_optimized && p.porous && Math.abs(p.front_angle) >= p.overhang_angle)
    throw Error(
      'Наклон передней стенки превышает угол свеса для печати / Front tilt exceeds the selected print overhang angle',
    )
  return `// Organizer box. External dimensions in mm; organic skeletal walls.
// Generic rail hooks: measure your rail. No load rating or physical fit test.
${Object.entries(p)
  .map(([key, value]) => `${key} = ${value};`)
  .join('\n')}
${boxGeometrySource}`
}
