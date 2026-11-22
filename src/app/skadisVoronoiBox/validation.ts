import {
  BOX_DEFAULTS,
  isSkadisMount,
  legacyBoxSchema,
  SKADIS_CONSTANTS,
  SKADIS_VORONOI_BOX_DEFAULTS,
  skadisVoronoiSchema,
  type LatticeMode,
  type LegacyBoxParams,
  type MountSystem,
  type SkadisLayoutInfo,
  type SkadisVoronoiBoxParams,
} from './schema'

export function resolveEffectiveHookCount(p: SkadisVoronoiBoxParams): number {
  if (p.mount_system === 'none') return 0
  if (p.hook_count > 0) return p.hook_count
  if (p.mount_system === 'skadis_dovetail') {
    const step = SKADIS_CONSTANTS.boardPitch * p.skadis_spacing_steps
    const margin = Math.max(16, p.corner_radius + 9)
    return Math.max(2, Math.min(8, Math.floor((p.width - 2 * margin) / step) + 1))
  }
  if (p.mount_system === 'skadis_integral') {
    const step = SKADIS_CONSTANTS.boardPitch * p.skadis_spacing_steps
    const margin = Math.max(10, p.corner_radius + 6)
    return Math.max(1, Math.min(8, Math.floor((p.width - 2 * margin) / step) + 1))
  }
  return Math.max(2, Math.min(8, Math.round(p.width / 90)))
}

export function computeSkadisLayout(p: SkadisVoronoiBoxParams): SkadisLayoutInfo {
  const effCount = resolveEffectiveHookCount(p)
  let hookStep = 0
  if (isSkadisMount(p.mount_system)) {
    hookStep = effCount > 1 ? SKADIS_CONSTANTS.boardPitch * p.skadis_spacing_steps : 0
  } else if (p.mount_system !== 'none') {
    const span = p.width - 2 * p.corner_radius
    const besselSpan = Math.max(
      0,
      p.width - 2 * Math.max(p.corner_radius + p.hook_width / 2, p.corner_radius + 0.2071 * span),
    )
    hookStep =
      p.hook_spacing ||
      (effCount === 1
        ? 0
        : p.dividers === effCount && effCount > 1
          ? (p.width - 2 * Math.max(p.wall, p.corner_radius)) / (p.dividers + 1)
          : besselSpan >= (effCount - 1) * p.hook_width * 1.15
            ? besselSpan / (effCount - 1)
            : span / (effCount + 1))
  }
  const hookSpan = hookStep * Math.max(0, effCount - 1)
  const first = (p.width - hookSpan) / 2
  const positions = p.mount_system === 'none' ? [] : Array.from({ length: effCount }, (_, i) => first + i * hookStep)
  const minWidth =
    p.mount_system === 'skadis_dovetail'
      ? hookSpan + 2 * Math.max(16, p.corner_radius + 9)
      : p.mount_system === 'skadis_integral'
        ? hookSpan + 2 * Math.max(10, p.corner_radius + 6)
        : p.mount_system === 'none'
          ? 4 * p.wall + 4
          : Math.max(4 * p.wall + 4, hookSpan + p.hook_width + 2 * p.corner_radius)
  const ribLines = Math.floor((p.rib + 1e-6) / p.line_width)
  const wallsPrintable = p.print_optimized || p.lattice_mode === 'solid'
  const mountPrintable =
    p.mount_system === 'none' ||
    p.mount_system === 'rail_dovetail' ||
    (p.mount_system === 'skadis_dovetail' && p.skadis_part > 0)
  return { hookStep, hookSpan, positions, minWidth, ribLines, supportFree: wallsPrintable && mountPrintable }
}

const LATTICE_LABELS: Record<'ru' | 'en', Record<string, string>> = {
  ru: {
    voronoi: 'Вороной (Rust)',
    honeycomb: 'соты',
    isogrid: 'изосетка',
    triangles: 'треугольники',
    grid: 'сетка',
    trabecular: 'органическая (как в коробке)',
    circles: 'отверстия',
    solid: 'сплошное',
  },
  en: {
    voronoi: 'Voronoi (Rust)',
    honeycomb: 'honeycomb',
    isogrid: 'isogrid',
    triangles: 'triangles',
    grid: 'grid',
    trabecular: 'organic (box style)',
    circles: 'circles',
    solid: 'solid',
  },
}

const RUST_PATTERNS = new Set(['voronoi', 'honeycomb', 'isogrid', 'triangles', 'grid'])

export function summarizeSkadisBox(p: SkadisVoronoiBoxParams, locale: 'ru' | 'en'): string {
  const layout = computeSkadisLayout(p)
  const posList = layout.positions.map(x => x.toFixed(1)).join(', ')
  const ru = locale === 'ru'
  const wallName = LATTICE_LABELS[locale][p.lattice_mode] ?? p.lattice_mode
  const floorName = LATTICE_LABELS[locale][p.floor_lattice_mode] ?? p.floor_lattice_mode
  const fillText = ru ? `стены: ${wallName} · дно: ${floorName}` : `walls: ${wallName} · floor: ${floorName}`
  if (isSkadisMount(p.mount_system)) {
    const effCount = layout.positions.length
    const autoSuffix = p.hook_count === 0 ? (ru ? ' авто' : ' auto') : ''
    const gridText =
      effCount > 1
        ? ru
          ? `SKÅDIS ${effCount} шт.${autoSuffix}, шаг ${layout.hookStep} мм (отверстия 5×15 мм, X = ${posList} мм)`
          : `SKÅDIS ${effCount}${autoSuffix} hooks, ${layout.hookStep} mm grid (5×15 mm slots, X = ${posList} мм)`
        : ru
          ? `SKÅDIS 1 крючок (X = ${posList} мм)`
          : `SKÅDIS 1 hook (X = ${posList} mm)`
    const ribText =
      p.lattice_mode === 'solid'
        ? ''
        : ru
          ? `перемычка ${p.rib} мм (≈ ${layout.ribLines} линий × ${p.line_width} мм)`
          : `rib ${p.rib} mm (≈ ${layout.ribLines} lines × ${p.line_width} mm)`
    const printText =
      p.mount_system === 'skadis_dovetail'
        ? p.skadis_part === 0
          ? ru
            ? 'режим сборки; для печати выберите деталь 1, 2 или 3'
            : 'assembly preview; switch part to 1, 2 or 3 for printing'
          : ru
            ? 'без поддержек (короткий 5 мм мост горловины паза)'
            : 'support-free (short ~5 mm throat bridge only)'
        : ru
          ? 'проверьте поддержки под зацепами крючков'
          : 'check slicer supports under integral hook lips'
    return `${gridText} · ${fillText}${ribText ? ` · ${ribText}` : ''} · ${printText}.`
  }
  return ru
    ? `Универсальная рейка · ${fillText} · перемычка ${p.rib} мм (≈ ${layout.ribLines} линий) · ${layout.supportFree ? 'ориентация под печать без поддержек' : 'проверьте поддержки в слайсере'}.`
    : `Generic rail · ${fillText} · rib ${p.rib} mm (≈ ${layout.ribLines} lines) · ${layout.supportFree ? 'support-free print orientation' : 'check supports in slicer'}.`
}

function validateCommonGeometry(p: SkadisVoronoiBoxParams): number {
  const { width: w, depth: d, height: h, wall: t, floor: f } = p
  if (w <= 4 * t || d <= 4 * t || h <= f + 2 * t)
    throw Error(
      'Размеры коробки слишком малы для выбранных стенок и дна / Box dimensions are too small for the wall and floor',
    )
  if ((w - 2 * Math.max(t, p.corner_radius) - p.dividers * t) / (p.dividers + 1) < 4)
    throw Error('Между перегородками должно оставаться минимум 4 мм / Compartments must be at least 4 mm wide')
  const topDepth = d + (h - f) * Math.tan((p.front_angle * Math.PI) / 180)
  if (topDepth <= 4 * t)
    throw Error(
      'Наклон закрывает коробку: увеличьте глубину или уменьшите угол / Tilt leaves insufficient opening depth',
    )
  if (2 * p.corner_radius >= Math.min(w, d, topDepth))
    throw Error('Радиус не помещается в габариты коробки / Corner radius does not fit the box dimensions')
  return topDepth
}

function validateMountGeometry(p: SkadisVoronoiBoxParams, layout: SkadisLayoutInfo, topDepth: number): void {
  if (p.mount_system === 'skadis_dovetail') {
    if (p.bend_radius > 0)
      throw Error('Крепление SKÅDIS требует плоской задней стенки / SKÅDIS mount requires bend_radius = 0')
    if (p.hook_count !== 0 && (p.hook_count < 2 || p.hook_count > 8))
      throw Error(
        'Для паза SKÅDIS выберите 0 (авто по ширине) или от 2 до 8 крючков / Choose 0 (auto) or 2 to 8 hooks for SKÅDIS dovetail',
      )
    if (p.skadis_back_thickness < 7)
      throw Error(
        'Задняя стенка под паз SKÅDIS должна быть не меньше 7 мм / SKÅDIS dovetail back thickness must be at least 7 mm',
      )
    if (p.depth < p.skadis_back_thickness + p.wall + 12)
      throw Error(
        'Глубина коробки мала для задней стенки SKÅDIS / Box depth is too shallow for SKÅDIS back wall and interior',
      )
    if (p.height < p.skadis_top_offset + 36)
      throw Error('Высота коробки мала для паза SKÅDIS и упора / Box height is too short for SKÅDIS dovetail channel')
    if (p.corner_radius > 0 && p.corner_radius < p.wall)
      throw Error(
        'Скругление углов должно быть 0 или не меньше толщины стенки / Corner radius must be 0 or at least wall thickness',
      )
    if (p.width < layout.minWidth)
      throw Error(
        `Коробка слишком узка для шага крючков SKÅDIS (нужно минимум ${Math.ceil(layout.minWidth)} мм) / Box too narrow for SKÅDIS hook span (need at least ${Math.ceil(layout.minWidth)} mm)`,
      )
  } else if (p.mount_system === 'skadis_integral') {
    if (p.bend_radius > 0)
      throw Error('Крепление SKÅDIS требует плоской задней стенки / SKÅDIS mount requires bend_radius = 0')
    if (p.hook_count > 8) throw Error('Максимум 8 крючков SKÅDIS / Maximum 8 SKÅDIS hooks')
    if (p.height < p.skadis_top_offset + 28)
      throw Error('Высота коробки мала для крючка SKÅDIS / Box height is too short for SKÅDIS hook')
    if (p.depth < p.wall * 2 + 10) throw Error('Глубина коробки мала / Box depth is too shallow')
    if (p.width < layout.minWidth)
      throw Error(
        `Коробка слишком узка для шага крючков SKÅDIS (нужно минимум ${Math.ceil(layout.minWidth)} мм) / Box too narrow for SKÅDIS hook span (need at least ${Math.ceil(layout.minWidth)} mm)`,
      )
  } else if (p.mount_system !== 'none') {
    const left = (p.width - layout.hookSpan - p.hook_width) / 2
    if (
      p.hook_drop + p.hook_thickness > p.height ||
      left < p.corner_radius ||
      (layout.positions.length > 1 && p.hook_width >= layout.hookStep)
    )
      throw Error(
        'Крючки не помещаются или перекрываются: уменьшите размеры, количество или шаг / Hooks do not fit or overlap; adjust their dimensions, count or spacing',
      )
  }
  if (p.bend_radius > 0) {
    const attachedOut = p.mount_system === 'rail_monolithic' ? p.hook_gap + p.hook_thickness : 0
    const minRadius = Math.ceil(
      Math.max(p.width / (2 * Math.PI) + 1, (p.bend_reverse ? Math.max(p.depth, topDepth) : attachedOut) + 1),
    )
    if (p.width / p.bend_radius >= 2 * Math.PI)
      throw Error(
        `Дуга замыкается: для ширины ${p.width} мм укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Increase bend radius to at least ${minRadius} mm`,
      )
    if (p.bend_radius <= (p.bend_reverse ? Math.max(p.depth, topDepth) : attachedOut))
      throw Error(
        `Изгиб пересекает ось: укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Bend reaches its axis; increase radius to at least ${minRadius} mm`,
      )
  }
}

function estimateSites(mode: string, spanX: number, spanY: number, effCell: number): number {
  const base = Math.ceil(spanX / effCell) * Math.ceil(spanY / (effCell * 0.866))
  if (mode === 'isogrid') return (Math.ceil(spanX / effCell) + 2) * (Math.ceil(spanY / (effCell * 0.866)) + 2) * 2
  if (mode === 'triangles') return base * 2
  return base
}

function validateLatticeGeometry(p: SkadisVoronoiBoxParams, enforceNozzle: boolean): void {
  const usesCircles = p.lattice_mode === 'circles' || p.floor_lattice_mode === 'circles' || (p.drain && !p.floor_mesh)
  if (usesCircles && p.hole_pitch < p.hole_diameter + 1.2)
    throw Error('Между отверстиями нужно минимум 1.2 мм материала / Leave at least 1.2 mm between holes')
  if (p.lattice_mode === 'trabecular') {
    const effCell = p.cell_size / p.mesh_density
    const minFactor = enforceNozzle ? 1.4 : 0.4
    if (effCell < p.rib * minFactor)
      throw Error(
        'Размер ячейки с учётом плотности должен быть не меньше перемычки / Effective cell size must be at least the rib width',
      )
  } else if (RUST_PATTERNS.has(p.lattice_mode)) {
    const effCell = p.cell_size / p.mesh_density
    if (effCell < p.rib * 1.8)
      throw Error(
        'Размер ячейки Вороного с учётом плотности должен быть не меньше 1.8× перемычки / Effective Voronoi cell size must be at least 1.8× rib width',
      )
    if (estimateSites(p.lattice_mode, Math.max(p.width, p.depth), Math.max(p.height - p.floor, p.depth), effCell) > 600)
      throw Error(
        'Слишком плотная сетка Вороного (>600 ячеек на грань): увеличьте размер ячейки или уменьшите плотность / Voronoi lattice exceeds 600 sites per face',
      )
  }
  if (enforceNozzle && p.lattice_mode !== 'solid' && p.lattice_mode !== 'circles') {
    if (p.rib + 1e-6 < p.line_width * p.min_rib_lines)
      throw Error(
        `Перемычка (${p.rib} мм) тоньше ${p.min_rib_lines} линий сопла (${(p.line_width * p.min_rib_lines).toFixed(2)} мм) / Rib width is narrower than requested extrusion lines`,
      )
  }
  if (!enforceNozzle && p.lattice_mode === 'trabecular' && p.floor_mesh) {
    const floorEff = p.floor_cell_size / p.mesh_density
    if (floorEff < p.floor_rib * 0.4)
      throw Error(
        'Размер ячейки дна с учётом плотности должен быть не меньше перемычки / Effective floor cell size must be at least the rib width',
      )
    if (floorEff > Math.min(p.width, p.depth) - 2 * Math.max(p.wall, p.floor_rib * 1.5))
      throw Error(
        'Ячейка дна не помещается в коробку: уменьшите её размер или увеличьте плотность / Floor cell does not fit; reduce its size or increase density',
      )
  } else if (p.floor_lattice_mode === 'trabecular' || RUST_PATTERNS.has(p.floor_lattice_mode)) {
    const floorEff = p.floor_cell_size / p.mesh_density
    const minFloorFactor = p.floor_lattice_mode === 'trabecular' ? 1.4 : 1.8
    if (floorEff < p.floor_rib * minFloorFactor)
      throw Error(
        `Размер ячейки дна с учётом плотности должен быть не меньше ${minFloorFactor}× перемычки дна / Effective floor cell size must be at least ${minFloorFactor}× floor rib width`,
      )
    const sitePattern = p.floor_lattice_mode === 'trabecular' ? 'voronoi' : p.floor_lattice_mode
    if (estimateSites(sitePattern, p.width, p.depth, floorEff) > 600)
      throw Error(
        'Слишком плотная сетка дна (>600 ячеек): увеличьте размер ячейки дна / Floor lattice exceeds 600 sites',
      )
    if (enforceNozzle && p.floor_rib + 1e-6 < p.line_width * p.min_rib_lines)
      throw Error(
        `Перемычка дна (${p.floor_rib} мм) тоньше ${p.min_rib_lines} линий сопла (${(p.line_width * p.min_rib_lines).toFixed(2)} мм) / Floor rib is narrower than requested extrusion lines`,
      )
  }
  if (p.print_optimized && p.lattice_mode !== 'solid' && Math.abs(p.front_angle) >= p.overhang_angle)
    throw Error(
      'Наклон передней стенки превышает угол свеса для печати / Front tilt exceeds the selected print overhang angle',
    )
}

function normalizeFloorFill(input: unknown): unknown {
  if (!input || typeof input !== 'object') return input
  const obj = { ...(input as Record<string, unknown>) }
  const mode = obj.floor_lattice_mode
  const floorMesh = obj.floor_mesh
  const drain = obj.drain
  if (mode === 'solid' || mode === 'circles') {
    obj.floor_mesh = false
    obj.drain = mode === 'circles'
  } else if (floorMesh === false) {
    obj.floor_lattice_mode = drain === true ? 'circles' : 'solid'
  } else if (typeof mode === 'string') {
    obj.floor_mesh = true
    obj.drain = false
  }
  return obj
}

function normalizeBendSegments(input: unknown): unknown {
  return input &&
    typeof input === 'object' &&
    'bend_radius' in input &&
    Number((input as Record<string, unknown>).bend_radius) <= 0
    ? {
        ...(input as Record<string, unknown>),
        bend_segments: Math.max(4, Math.trunc(Number((input as Record<string, unknown>).bend_segments) || 24)),
      }
    : input
}

export function parseSkadisVoronoiBox(input: unknown = SKADIS_VORONOI_BOX_DEFAULTS): SkadisVoronoiBoxParams {
  const p = skadisVoronoiSchema.parse(normalizeFloorFill(normalizeBendSegments(input)))
  const topDepth = validateCommonGeometry(p)
  const layout = computeSkadisLayout(p)
  validateMountGeometry(p, layout, topDepth)
  validateLatticeGeometry(p, true)
  return p
}

export function parseLegacyBox(input: unknown = BOX_DEFAULTS): {
  legacy: LegacyBoxParams
  unified: SkadisVoronoiBoxParams
} {
  const l = legacyBoxSchema.parse(normalizeBendSegments(input))
  const mount_system: MountSystem = !l.hooks
    ? 'none'
    : l.hook_mount === 1
      ? 'rail_t_slot'
      : l.hook_mount === 2
        ? 'rail_dovetail'
        : 'rail_monolithic'
  const lattice_mode: LatticeMode = !l.porous ? 'solid' : l.organic ? 'trabecular' : 'circles'
  const unified: SkadisVoronoiBoxParams = {
    ...SKADIS_VORONOI_BOX_DEFAULTS,
    width: l.width,
    depth: l.depth,
    height: l.height,
    wall: l.wall,
    floor: l.floor,
    corner_radius: l.corner_radius,
    front_angle: l.front_angle,
    dividers: l.dividers,
    bend_radius: l.bend_radius,
    bend_reverse: l.bend_reverse,
    bend_segments: l.bend_segments,
    mount_system,
    hook_count: l.hook_count,
    hook_width: l.hook_width,
    hook_thickness: l.hook_thickness,
    hook_gap: l.hook_gap,
    hook_drop: l.hook_drop,
    hook_spacing: l.hook_spacing,
    hook_chamfer: l.hook_chamfer,
    hook_tolerance: l.hook_tolerance,
    lattice_mode,
    print_optimized: l.print_optimized,
    overhang_angle: l.overhang_angle,
    cell_size: l.cell_size,
    cell_radius: l.cell_radius,
    rib: l.rib,
    stress_gradient: l.stress_gradient,
    mesh_density: l.mesh_density,
    pattern_seed: l.pattern_seed,
    hole_diameter: l.hole_diameter,
    hole_pitch: l.hole_pitch,
    floor_lattice_mode: l.floor_mesh ? 'voronoi' : l.drain ? 'circles' : 'solid',
    floor_mesh: l.floor_mesh,
    floor_cell_size: l.floor_cell_size,
    floor_rib: l.floor_rib,
    drain: l.drain,
  }
  const topDepth = validateCommonGeometry(unified)
  const layout = computeSkadisLayout(unified)
  validateMountGeometry(unified, layout, topDepth)
  validateLatticeGeometry(unified, false)
  return { legacy: l, unified }
}
