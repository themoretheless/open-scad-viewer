import { z } from 'zod/v4'

export interface BushingParameterSuggestion {
  title: string
  description: string
  changes: Record<string, number>
}

export const BUSHING_DEFAULTS = {
  thread_mode: 3,
  auto_thread_std: true,
  fdm_thread_angle: true,
  length: 40,
  thread_section_length: 10,
  inner_diameter: 12,
  inner_pitch: 1.5,
  inner_starts: 1,
  inner_left_handed: false,
  inner_thread_depth: 0,
  outer_diameter: 22,
  outer_pitch: 1.5,
  outer_starts: 1,
  outer_left_handed: false,
  outer_shape: 0,
  wrench_flat_depth: 1.2,
  knurl_count: 16,
  knurl_depth: 0.5,
  flange: false,
  flange_diameter: 30,
  flange_thickness: 3,
  flange_shape: 0,
  flange_holes: 0,
  flange_hole_diameter: 3.2,
  blind_bottom: 0,
  chamfer: 0.8,
  clearance: 0.22,
  cross_hole_diameter: 0,
  cross_hole_z: 0,
  split_slot_width: 0,
  segments: 36,
}

export const bushingSchema = z.object({
  thread_mode: z.number().int().min(0).max(3).default(3),
  auto_thread_std: z.boolean().default(true),
  fdm_thread_angle: z.boolean().default(true),
  length: z.number().finite().min(4).max(300).default(40),
  thread_section_length: z.number().finite().min(2).max(120).default(10),
  inner_diameter: z.number().finite().min(1.6).max(200).default(12),
  inner_pitch: z.number().finite().min(0.25).max(10).default(1.5),
  inner_starts: z.number().int().min(1).max(4).default(1),
  inner_left_handed: z.boolean().default(false),
  inner_thread_depth: z.number().finite().min(0).max(200).default(0),
  outer_diameter: z.number().finite().min(3).max(250).default(22),
  outer_pitch: z.number().finite().min(0.25).max(10).default(1.5),
  outer_starts: z.number().int().min(1).max(4).default(1),
  outer_left_handed: z.boolean().default(false),
  outer_shape: z.number().int().min(0).max(4).default(0),
  wrench_flat_depth: z.number().finite().min(0.2).max(30).default(1.2),
  knurl_count: z.number().int().min(4).max(64).default(16),
  knurl_depth: z.number().finite().min(0.1).max(5).default(0.5),
  flange: z.boolean().default(false),
  flange_diameter: z.number().finite().min(4).max(300).default(30),
  flange_thickness: z.number().finite().min(0.8).max(80).default(3),
  flange_shape: z.number().int().min(0).max(2).default(0),
  flange_holes: z.number().int().min(0).max(12).default(0),
  flange_hole_diameter: z.number().finite().min(1).max(30).default(3.2),
  blind_bottom: z.number().finite().min(0).max(100).default(0),
  chamfer: z.number().finite().min(0).max(10).default(0.8),
  clearance: z.number().finite().min(0).max(1.5).default(0.22),
  cross_hole_diameter: z.number().finite().min(0).max(80).default(0),
  cross_hole_z: z.number().finite().min(0).max(200).default(0),
  split_slot_width: z.number().finite().min(0).max(40).default(0),
  segments: z.number().int().min(16).max(72).default(36),
}).strict()

export type BushingOptions = z.infer<typeof bushingSchema>

export const ISO_PROFILE_FACTOR = (5 * Math.sqrt(3)) / 16 // ~0.541265877
export const FDM_45_PROFILE_FACTOR = 0.375 // 45-degree overhang flank with ISO root/crest truncation

export function validateBushingParameters(p: BushingOptions): void {
  const isTubeMaleFemale = p.thread_mode === 3
  const hasInnerThread = p.thread_mode === 0 || p.thread_mode === 2
  const hasOuterThread = p.thread_mode === 1 || p.thread_mode === 2
  const profileFactor = p.fdm_thread_angle ? FDM_45_PROFILE_FACTOR : ISO_PROFILE_FACTOR

  const dMid = (p.inner_diameter + p.outer_diameter) / 2
  const wallTotal = (p.outer_diameter - p.inner_diameter) / 2
  const effClearance = p.auto_thread_std ? fdmRadialClearance(dMid) : p.clearance
  const effInPitch = p.auto_thread_std
    ? Math.min(standardMetricPitch(isTubeMaleFemale ? dMid : p.inner_diameter), Math.max(0.5, (wallTotal - 1.5) / 0.6))
    : p.inner_pitch
  const effOutPitch = p.auto_thread_std
    ? Math.min(standardMetricPitch(p.outer_diameter), Math.max(0.5, (wallTotal - 1.5) / 0.6))
    : p.outer_pitch

  if (isTubeMaleFemale) {
    if (p.length < 2 * p.thread_section_length + 2) {
      throw Error(
        `Общая длина трубки (${p.length} мм) должна быть минимум на 2 мм больше удвоенной длины резьбового участка (${2 * p.thread_section_length + 2} мм) / Tube length must be at least 2 * thread_section_length + 2 mm`,
      )
    }
    const hJoint = profileFactor * effInPitch
    const eachWall = (wallTotal - hJoint - effClearance) / 2
    if (eachWall < 0.75) {
      throw Error(
        `Недостаточная толщина стенки трубки для внутренней («мама») и наружной («папа») резьбы (${eachWall.toFixed(2)} мм < 0.75 мм): увеличьте наружный диаметр или уменьшите внутренний диаметр / Insufficient wall thickness for female+male tube threads (${eachWall.toFixed(2)} mm < 0.75 mm): increase outer_diameter or reduce inner_diameter`,
      )
    }
  } else {
    if (hasInnerThread && p.inner_diameter < 2 * effInPitch) {
      throw Error('Внутренний диаметр должен быть минимум вдвое больше шага внутренней резьбы / inner_diameter must be at least 2 * inner_pitch')
    }
    if (hasOuterThread && p.outer_diameter < 2 * effOutPitch) {
      throw Error('Наружный диаметр должен быть минимум вдвое больше шага наружной резьбы / outer_diameter must be at least 2 * outer_pitch')
    }

    const rInnerMax = p.inner_diameter / 2 + effClearance / 2
    let rOuterMin = p.outer_diameter / 2
    if (hasOuterThread) {
      rOuterMin -= effClearance / 2 + profileFactor * effOutPitch
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
  }

  if (p.blind_bottom > 0 && p.blind_bottom >= p.length - 1) {
    throw Error('Толщина глухого дна должна оставлять минимум 1 мм глубины канала / blind_bottom must leave at least 1 mm of cavity depth')
  }
  if (!isTubeMaleFemale && p.inner_thread_depth > 0 && p.inner_thread_depth > p.length - p.blind_bottom) {
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
}

export function standardMetricPitch(diameter: number): number {
  if (diameter <= 2) return 0.4
  if (diameter <= 2.6) return 0.45
  if (diameter <= 3.2) return 0.5
  if (diameter <= 4.2) return 0.7
  if (diameter <= 5.2) return 0.8
  if (diameter <= 6.5) return 1.0
  if (diameter <= 9) return 1.25
  if (diameter <= 12) return 1.5
  if (diameter <= 16) return 1.75
  if (diameter <= 22) return 2.0
  if (diameter <= 30) return 2.5
  if (diameter <= 42) return 3.0
  return 3.5
}

export function fdmRadialClearance(diameter: number): number {
  return round2(Math.min(0.45, Math.max(0.16, 0.14 + 0.005 * diameter)))
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
  const mode = Number(rawValues.thread_mode ?? 3)
  const innerPitch = Number(rawValues.inner_pitch ?? 1.5)
  const outerPitch = Number(rawValues.outer_pitch ?? 1.5)
  const clearance = Number(rawValues.clearance ?? 0.22)
  const flange = Boolean(rawValues.flange)
  const flangeD = Number(rawValues.flange_diameter ?? 28)
  const fdmAngle = Boolean(rawValues.fdm_thread_angle ?? true)
  if (!Number.isFinite(innerD) || !Number.isFinite(outerD)) return []

  const suggestions: BushingParameterSuggestion[] = []
  const profileFactor = fdmAngle ? FDM_45_PROFILE_FACTOR : ISO_PROFILE_FACTOR

  if (mode === 3) {
    // Tube mode (female bottom + male top sharing the wall thickness)
    const recPitch = standardMetricPitch(innerD + 6)
    const recClearance = fdmRadialClearance(innerD + 6)
    const minWallTotal = 2 * 1.2 + profileFactor * recPitch + recClearance
    const minSafeOuterD = round2(innerD + 2 * Math.max(3.5, minWallTotal))
    if (changedKey === 'inner_diameter' || changedKey === 'thread_mode' || outerD < minSafeOuterD) {
      const targetOuterD = Math.max(outerD, minSafeOuterD)
      const changes: Record<string, number> = {}
      if (targetOuterD !== outerD) changes.outer_diameter = targetOuterD
      if (Math.abs(innerPitch - recPitch) > 0.01) changes.inner_pitch = recPitch
      if (Math.abs(clearance - recClearance) > 0.02) changes.clearance = recClearance
      if (flange && flangeD <= targetOuterD + 2) changes.flange_diameter = round2(targetOuterD + 6)
      if (Object.keys(changes).length > 0) {
        const summary = Object.entries(changes).map(([k, v]) => `${k} = ${v} мм`).join(', ')
        suggestions.push({
          title: `Подобрать стенку и стандарт резьбы под 3D-печать (${summary})`,
          description: `Для трубки «мама-папа» с внутренним каналом ${innerD} мм рекомендуется согласовать наружный диаметр, шаг и зазор FDM:`,
          changes,
        })
      }
    }
    return suggestions
  }

  const hasInnerThread = mode === 0 || mode === 2
  const hasOuterThread = mode === 1 || mode === 2

  const rInnerMax = innerD / 2 + clearance / 2
  const outerThreadDepth = hasOuterThread ? profileFactor * outerPitch + clearance / 2 : 0
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
