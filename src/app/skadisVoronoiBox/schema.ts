import { z } from 'zod/v4'

export const SKADIS_CONSTANTS = Object.freeze({
  boardPitch: 40,
  slotWidth: 5,
  slotHeight: 15,
  boardThickness: 5,
  hookWidth: 4.4,
  minBackThickness: 7,
  ribWidth: 16,
  ribDepth: 10,
})

export type MountSystem =
  'skadis_dovetail' | 'skadis_integral' | 'rail_monolithic' | 'rail_t_slot' | 'rail_dovetail' | 'none'

export type LatticeMode =
  'voronoi' | 'honeycomb' | 'isogrid' | 'triangles' | 'grid' | 'trabecular' | 'circles' | 'solid'

export type FloorLatticeMode =
  'trabecular' | 'voronoi' | 'honeycomb' | 'isogrid' | 'triangles' | 'grid' | 'circles' | 'solid'

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

export const SKADIS_VORONOI_BOX_DEFAULTS = {
  width: 120,
  depth: 75,
  height: 80,
  wall: 2,
  floor: 2,
  corner_radius: 5,
  front_angle: 0,
  dividers: 0,
  bottom_fillet: true,
  edge_frame: false,
  bend_radius: 0,
  bend_reverse: false,
  bend_segments: 24,
  mount_system: 'skadis_dovetail' as MountSystem,
  hook_count: 0,
  skadis_spacing_steps: 2,
  skadis_top_offset: 14,
  skadis_back_thickness: 7,
  skadis_fit: 0.25,
  skadis_friction_ridge: true,
  skadis_part: 0,
  hook_width: 10,
  hook_thickness: 3,
  hook_gap: 6,
  hook_drop: 16,
  hook_spacing: 0,
  hook_chamfer: false,
  hook_tolerance: 0.2,
  lattice_mode: 'trabecular' as LatticeMode,
  print_optimized: false,
  overhang_angle: 45,
  cell_size: 12,
  cell_radius: 2,
  rib: 2.4,
  voronoi_jitter: 0.7,
  stress_gradient: 1,
  mesh_density: 1,
  pattern_seed: 1,
  hole_diameter: 6,
  hole_pitch: 16,
  floor_lattice_mode: 'trabecular' as FloorLatticeMode,
  floor_mesh: true,
  floor_frame: false,
  floor_cell_size: 12,
  floor_cell_radius: 2,
  floor_rib: 2.4,
  floor_jitter: 0.7,
  drain: false,
  line_width: 0.4,
  min_rib_lines: 2,
}

const dimension = z.number().finite().min(0.8).max(500)

export const legacyBoxSchema = z
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

export const skadisVoronoiSchema = z
  .object({
    width: dimension,
    depth: dimension,
    height: dimension,
    wall: dimension,
    floor: dimension,
    corner_radius: z.number().finite().min(0).max(250).default(5),
    front_angle: z.number().finite().min(-60).max(60).default(0),
    dividers: z.number().int().min(0).max(8).default(0),
    bottom_fillet: z.boolean().default(true),
    edge_frame: z.boolean().default(false),
    bend_radius: z.number().finite().min(0).max(100000).default(0),
    bend_reverse: z.boolean().default(false),
    bend_segments: z.number().int().min(4).max(128).default(24),
    mount_system: z
      .enum(['skadis_dovetail', 'skadis_integral', 'rail_monolithic', 'rail_t_slot', 'rail_dovetail', 'none'])
      .default('skadis_dovetail'),
    hook_count: z.number().int().min(0).max(8).default(0),
    skadis_spacing_steps: z.number().int().min(1).max(6).default(2),
    skadis_top_offset: z.number().finite().min(8).max(40).default(14),
    skadis_back_thickness: z.number().finite().min(3).max(16).default(7),
    skadis_fit: z.number().finite().min(0.1).max(0.5).default(0.25),
    skadis_friction_ridge: z.boolean().default(true),
    skadis_part: z.number().int().min(0).max(3).default(0),
    hook_width: dimension.default(10),
    hook_thickness: dimension.default(3),
    hook_gap: dimension.default(6),
    hook_drop: dimension.default(16),
    hook_spacing: z.number().finite().min(0).max(500).default(0),
    hook_chamfer: z.boolean().default(false),
    hook_tolerance: z.number().finite().min(0.05).max(0.8).default(0.2),
    lattice_mode: z
      .enum(['voronoi', 'honeycomb', 'isogrid', 'triangles', 'grid', 'trabecular', 'circles', 'solid'])
      .default('trabecular'),
    print_optimized: z.boolean().default(false),
    overhang_angle: z.number().finite().min(10).max(80).default(45),
    cell_size: dimension.min(1).default(12),
    cell_radius: z.number().finite().min(0).max(50).default(2),
    rib: dimension.default(2.4),
    voronoi_jitter: z.number().finite().min(0).max(1).default(0.7),
    stress_gradient: z.number().finite().min(0).max(2).default(1),
    mesh_density: z.number().finite().min(0.5).max(2).default(1),
    pattern_seed: z.number().int().min(0).max(100000).default(1),
    hole_diameter: dimension.default(6),
    hole_pitch: dimension.default(16),
    floor_lattice_mode: z
      .enum(['trabecular', 'voronoi', 'honeycomb', 'isogrid', 'triangles', 'grid', 'circles', 'solid'])
      .default('trabecular'),
    floor_mesh: z.boolean().default(true),
    floor_frame: z.boolean().default(false),
    floor_cell_size: dimension.min(1).default(12),
    floor_cell_radius: z.number().finite().min(0).max(50).default(2),
    floor_rib: dimension.default(2.4),
    floor_jitter: z.number().finite().min(0).max(1).default(0.7),
    drain: z.boolean().default(false),
    line_width: z.number().finite().min(0.1).max(2).default(0.4),
    min_rib_lines: z.number().int().min(1).max(8).default(2),
  })
  .strict()

export type LegacyBoxParams = z.infer<typeof legacyBoxSchema>
export type SkadisVoronoiBoxParams = z.infer<typeof skadisVoronoiSchema>

export interface SkadisLayoutInfo {
  hookStep: number
  hookSpan: number
  positions: number[]
  minWidth: number
  ribLines: number
  supportFree: boolean
}

export function isSkadisMount(mount: MountSystem): boolean {
  return mount === 'skadis_dovetail' || mount === 'skadis_integral'
}
