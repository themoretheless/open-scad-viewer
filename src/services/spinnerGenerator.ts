import { z } from 'zod/v4'

export const SPINNER_DEFAULTS = {
  inner_radius: 30.845,
  outer_radius: 32,
  hole: 44,
  gap: 0.05,
  height: 10,
  helix: 35,
}

export const SPINNER_SAFE_DEFAULTS = {
  hole: 38,
  inner_rim: 0.6,
  outer_rim: 0.6,
  gap: 0.25,
  height: 6,
  helix: 35,
}

const schema = z.object({
  inner_radius: z.number().finite().min(10).max(80).default(30.845),
  outer_radius: z.number().finite().min(11).max(100).default(32),
  hole: z.number().finite().min(0).max(100).default(44),
  gap: z.number().finite().min(0).max(0.6).default(0.05),
  height: z.number().finite().min(2).max(30).default(10),
  helix: z.number().finite().min(0).max(45).default(35),
}).strict()

const safeSchema = z.object({
  hole: z.number().finite().min(10).max(100).default(38),
  inner_rim: z.number().finite().min(0.4).max(20).default(0.6),
  outer_rim: z.number().finite().min(0.4).max(20).default(0.6),
  gap: z.number().finite().min(0).max(0.6).default(0.25),
  height: z.number().finite().min(2).max(30).default(6),
  helix: z.number().finite().min(0).max(45).default(35),
}).strict()

/**
 * Generates a parametric Rush/1 herringbone planetary spinner (sun = 32 teeth,
 * ring = 40 teeth, 18 planets of 4 teeth, each gear a single NURBS body).
 */
export function generateSpinner(input: unknown = SPINNER_DEFAULTS): string {
  const p = schema.parse(input)
  const gearScale = p.inner_radius / 30.845
  const gearModule = 1.5 * gearScale
  const clearance = 0.1 * gearScale
  const ringRim = p.outer_radius - (gearModule * 40 / 2 + gearModule + clearance)
  const maxHole = 2 * (gearModule * 32 / 2 - gearModule - clearance) - gearModule / 2

  if (p.outer_radius <= p.inner_radius) {
    throw Error('Наружный радиус должен быть больше внутреннего / outer_radius must be greater than inner_radius')
  }
  if (p.gap > 0.6 * gearScale) {
    throw Error('Зазор слишком велик для выбранного размера зубьев / Gap exceeds tooth allowance')
  }
  if (ringRim < gearModule / 4) {
    throw Error('Недостаточная толщина наружного кольца: увеличьте наружный радиус или уменьшите внутренний / Ring rim is too thin: increase outer_radius or reduce inner_radius')
  }
  if (p.hole > maxHole) {
    throw Error('Центральное отверстие выходит за корень солнечной шестерни: уменьшите отверстие или увеличьте внутренний радиус / Center hole exceeds sun gear root diameter')
  }

  return `// @rush/1
// Шевронный спиннер: солнце 32, венец 40, 18 сателлитов по 4 зуба.
// Каждая шестерня — одно NURBS-тело; эвольвента и винтовые стенки подогнаны сплайнами.
param inner_radius = ${p.inner_radius}mm range 10mm..80mm
param outer_radius = ${p.outer_radius}mm range 11mm..100mm
param hole = ${p.hole}mm range 0mm..100mm
param gap = ${p.gap}mm range 0mm..0.6mm
param height = ${p.height}mm range 2mm..30mm
param helix = ${p.helix}deg range 0deg..45deg

sun_teeth = 32
planet_teeth = 4
ring_teeth = 40
planet_count = 18
gear_scale = inner_radius / 30.845mm
gear_module = 1.5mm * gear_scale
clearance = 0.1mm * gear_scale
orbit_radius = gear_module * (sun_teeth + planet_teeth) / 2
ring_rim = outer_radius - (gear_module * ring_teeth / 2 + gear_module + clearance)

validate gap.atMost(0.6mm * gear_scale).message("Зазор не подходит к размеру зубьев")
validate ring_rim.atLeast(gear_module / 4).message("Недостаточная толщина кольца: увеличьте outer_radius или уменьшите inner_radius")
validate outer_radius.greaterThan(inner_radius).message("outer_radius должен быть больше inner_radius")
validate hole.atMost(2 * (gear_module * sun_teeth / 2 - gear_module - clearance) - gear_module / 2).message("Отверстие выходит за корень центральной шестерни")

// Солнце и сателлиты - противоположного наклона, сателлиты и венец - одного.
sun = gear(teeth: sun_teeth, module: gear_module, thickness: height, helix_angle: -helix, herringbone: true,
  bore: hole, clearance: clearance, backlash: gap)
ring = gear(teeth: ring_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  internal: true, rim_width: ring_rim, clearance: clearance, backlash: gap)
planet = gear(teeth: planet_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  clearance: clearance, backlash: gap)

// Фаза зацепления несмещённого эвольвентного ряда при солнце в нуле.
planets = [for i in 0..<planet_count
  let orbit = i * 360deg / planet_count
  let phase = (1 + sun_teeth / planet_teeth) * orbit + 180deg - 180deg / planet_teeth
  => planet.rotate(z: phase - orbit).translate(x: orbit_radius).rotate(z: orbit)]

show [sun, ring, planets]
`
}

/**
 * Second variant: conflict-free spinner parameterized directly by center hole
 * diameter (`hole`), inner sun wall thickness (`inner_rim`), and outer ring
 * wall thickness (`outer_rim`), so changing any slider automatically scales
 * the gear train without wall-thickness or radius-order errors.
 */
export function generateSpinnerSafe(input: unknown = SPINNER_SAFE_DEFAULTS): string {
  const p = safeSchema.parse(input)

  return `// @rush/1
// Шевронный спиннер (вариант 2 — по отверстию и толщинам бортиков без конфликтов радиусов).
// Радиусы шестерён вычисляются автоматически из отверстия hole и толщин бортиков inner_rim / outer_rim.
param hole = ${p.hole}mm range 10mm..100mm
param inner_rim = ${p.inner_rim}mm range 0.4mm..20mm
param outer_rim = ${p.outer_rim}mm range 0.4mm..20mm
param gap = ${p.gap}mm range 0mm..0.6mm
param height = ${p.height}mm range 2mm..30mm
param helix = ${p.helix}deg range 0deg..45deg

sun_teeth = 32
planet_teeth = 4
ring_teeth = 40
planet_count = 18

// Корень зуба солнца = hole / 2 + inner_rim => модуль шестерён автоматически подстраивается под отверстие и внутренний бортик
sun_root_radius = hole / 2 + inner_rim
gear_scale = sun_root_radius / 22.4mm
gear_module = 1.5mm * gear_scale
// Радиальный зазор (между вершиной одного зуба и впадиной другого) растёт вместе с gap, чтобы верхушки зубьев не приваривались при FDM-печати
clearance = 0.12mm * gear_scale + gap
planet_backlash = gap * 0.45
orbit_radius = gear_module * (sun_teeth + planet_teeth) / 2
ring_rim = outer_rim

validate gap.atMost(0.65mm * gear_scale).message("Уменьшите gap: слишком большой боковой зазор для текущего размера зубьев")
validate inner_rim.atLeast(0.4mm * gear_scale).message("Увеличьте inner_rim: слишком тонкий внутренний бортик солнца")
validate outer_rim.atLeast(0.4mm * gear_scale).message("Увеличьте outer_rim: слишком тонкий наружный бортик кольца")

// Солнце и сателлиты - противоположного наклона, сателлиты и венец - одного.
sun = gear(teeth: sun_teeth, module: gear_module, thickness: height, helix_angle: -helix, herringbone: true,
  bore: hole, clearance: clearance, backlash: gap)
ring = gear(teeth: ring_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  internal: true, rim_width: ring_rim, clearance: clearance, backlash: gap)
planet = gear(teeth: planet_teeth, module: gear_module, thickness: height, helix_angle: helix, herringbone: true,
  clearance: clearance, backlash: planet_backlash)

// Фаза зацепления несмещённого эвольвентного ряда при солнце в нуле.
planets = [for i in 0..<planet_count
  let orbit = i * 360deg / planet_count
  let phase = (1 + sun_teeth / planet_teeth) * orbit + 180deg - 180deg / planet_teeth
  => planet.rotate(z: phase - orbit).translate(x: orbit_radius).rotate(z: orbit)]

show [sun, ring, planets]
`
}

export interface SpinnerParameterSuggestion {
  title: string
  description: string
  changes: Record<string, number>
}

function round2(value: number): number {
  return Math.round(value * 100) / 100
}

/**
 * Computes suggested dependent parameter updates when one spinner parameter
 * is edited in either Variant 1 (`inner_radius`, `outer_radius`, `hole`) or
 * Variant 2 (`hole`, `inner_rim`, `outer_rim`).
 */
export function computeSpinnerParameterSuggestions(
  changedName: string,
  params: Readonly<Record<string, number | boolean | string>>,
): SpinnerParameterSuggestion[] {
  const num = (key: string) => {
    const v = params[key]
    return typeof v === 'number' && Number.isFinite(v) ? v : undefined
  }
  const suggestions: SpinnerParameterSuggestion[] = []

  // Variant 1: inner_radius, outer_radius, hole
  const innerRadius = num('inner_radius')
  const outerRadius = num('outer_radius')
  const hole = num('hole')
  if (innerRadius !== undefined && outerRadius !== undefined && hole !== undefined) {
    const gearScale = innerRadius / 30.845
    const sunRootRadius = 22.4 * gearScale
    const ringRootRadius = 31.6 * gearScale
    const currentInnerRim = sunRootRadius - hole / 2
    const currentOuterRim = outerRadius - ringRootRadius
    const gap = num('gap')
    const maxSafeGap = round2(Math.max(0.02, 0.30 * gearScale))
    const recGap = round2(Math.min(maxSafeGap, Math.max(0.04, 0.12 * gearScale)))

    if (gap !== undefined && gap > maxSafeGap) {
      suggestions.push({
        title: `Уменьшить зазор до безопасного: gap → ${recGap} мм (макс. ${maxSafeGap} мм для 4-зубых сателлитов)`,
        description: `При текущем размере зубьев зазор gap = ${gap} мм срезает вершину 4-зубого сателлита («Gear tooth profile collapses»)`,
        changes: { gap: recGap },
      })
    }

    if (changedName === 'inner_radius') {
      const targetOuter = round2(ringRootRadius + 2.5)
      const targetHole = round2(Math.max(10, 2 * (sunRootRadius - 2.5)))
      const changes: Record<string, number> = { outer_radius: targetOuter, hole: targetHole }
      if (gap !== undefined && gap > maxSafeGap) changes.gap = recGap
      if (Math.abs(outerRadius - targetOuter) >= 0.1 || Math.abs(hole - targetHole) >= 0.1) {
        suggestions.push({
          title: `Подогнать под бортики 2.5 мм (${Object.entries(changes).map(([k, v]) => `${k} → ${v} мм`).join(', ')})`,
          description: `При inner_radius = ${innerRadius} мм корень венца = ${round2(ringRootRadius)} мм, корень солнца = ${round2(sunRootRadius)} мм`,
          changes,
        })
      }
      const keepHoleOuter = round2(ringRootRadius + Math.max(1.5, currentOuterRim))
      if (currentInnerRim < 0.8) {
        const safeHole = round2(Math.max(10, 2 * (sunRootRadius - 2.0)))
        suggestions.push({
          title: `Уменьшить отверстие до безопасного (hole → ${safeHole} мм, бортик 2 мм)`,
          description: `Сейчас внутренний бортик солнца слишком тонкий (${round2(currentInnerRim)} мм)`,
          changes: { hole: safeHole, ...(currentOuterRim < 0.8 ? { outer_radius: round2(ringRootRadius + 2.0) } : {}) },
        })
      } else if (currentOuterRim < 0.8 && Math.abs(outerRadius - keepHoleOuter) >= 0.1) {
        const safeOuter = round2(ringRootRadius + 2.0)
        suggestions.push({
          title: `Увеличить наружный радиус (outer_radius → ${safeOuter} мм, бортик 2 мм)`,
          description: `Сейчас наружный бортик кольца слишком тонкий (${round2(currentOuterRim)} мм)`,
          changes: { outer_radius: safeOuter },
        })
      }
    } else if (changedName === 'hole') {
      const targetInnerForHole = round2(((hole / 2 + 2.5) / 22.4) * 30.845)
      const targetRingRoot = 31.6 * (targetInnerForHole / 30.845)
      const targetOuterForHole = round2(targetRingRoot + 2.5)
      if (Math.abs(innerRadius - targetInnerForHole) >= 0.1 || Math.abs(outerRadius - targetOuterForHole) >= 0.1) {
        suggestions.push({
          title: `Пересчитать радиусы под hole = ${hole} мм и бортики 2.5 мм (inner_radius → ${targetInnerForHole}, outer_radius → ${targetOuterForHole})`,
          description: `Масштабирует шестерни так, чтобы вокруг отверстия ${hole} мм остался бортик 2.5 мм и снаружи 2.5 мм`,
          changes: { inner_radius: targetInnerForHole, outer_radius: targetOuterForHole },
        })
      }
    } else if (changedName === 'outer_radius') {
      const targetRingRoot = outerRadius - 2.5
      const targetInnerForOuter = round2((targetRingRoot / 31.6) * 30.845)
      const targetSunRoot = 22.4 * (targetInnerForOuter / 30.845)
      const targetHoleForOuter = round2(Math.max(10, 2 * (targetSunRoot - 2.5)))
      if (targetInnerForOuter >= 10 && (Math.abs(innerRadius - targetInnerForOuter) >= 0.1 || Math.abs(hole - targetHoleForOuter) >= 0.1)) {
        suggestions.push({
          title: `Пересчитать внутренний радиус и отверстие под outer_radius = ${outerRadius} мм (inner_radius → ${targetInnerForOuter}, hole → ${targetHoleForOuter})`,
          description: `Сохраняет бортики 2.5 мм при наружном диаметре ${round2(outerRadius * 2)} мм`,
          changes: { inner_radius: targetInnerForOuter, hole: targetHoleForOuter },
        })
      }
    }
  }

  // Variant 2: hole, inner_rim, outer_rim
  const innerRim = num('inner_rim')
  const outerRim = num('outer_rim')
  if (hole !== undefined && innerRim !== undefined && outerRim !== undefined) {
    const sunRootRadius = hole / 2 + innerRim
    const gearScale = sunRootRadius / 22.4
    const ringRootRadius = 31.6 * gearScale
    const outerDiameter = round2(2 * (ringRootRadius + outerRim))
    const maxSafeGap = round2(Math.max(0.05, 0.60 * gearScale))
    const gap = num('gap')

    if (gap !== undefined && gap > maxSafeGap) {
      suggestions.push({
        title: `Уменьшить зазор до максимально допустимого: gap → ${maxSafeGap} мм`,
        description: `При hole = ${hole} мм и inner_rim = ${innerRim} мм (модуль зуба ${round2(1.5 * gearScale)} мм) зазор больше ${maxSafeGap} мм выходит за допуск зуба`,
        changes: { gap: maxSafeGap },
      })
    }

    if (changedName === 'inner_rim' && Math.abs(outerRim - innerRim) >= 0.1) {
      const changes: Record<string, number> = { outer_rim: innerRim }
      if (gap !== undefined && gap > maxSafeGap) changes.gap = maxSafeGap
      suggestions.push({
        title: `Сделать наружный бортик таким же: ${Object.entries(changes).map(([k, v]) => `${k} → ${v} мм`).join(', ')}`,
        description: `Симметричные бортики по ${innerRim} мм (наружный диаметр станет ${round2(2 * (ringRootRadius + innerRim))} мм)`,
        changes,
      })
    } else if (changedName === 'outer_rim' && Math.abs(innerRim - outerRim) >= 0.1) {
      suggestions.push({
        title: `Сделать внутренний бортик таким же: inner_rim → ${outerRim} мм`,
        description: `Симметричные бортики по ${outerRim} мм`,
        changes: { inner_rim: outerRim },
      })
    } else if (changedName === 'hole') {
      const scaledRim = round2(Math.max(0.4, Math.min(8, (hole / 38) * 0.6)))
      const scaledGearScale = (hole / 2 + scaledRim) / 22.4
      const scaledMaxSafeGap = round2(Math.max(0.05, 0.60 * scaledGearScale))
      const changes: Record<string, number> = {}
      if (Math.abs(innerRim - scaledRim) >= 0.15) changes.inner_rim = scaledRim
      if (Math.abs(outerRim - scaledRim) >= 0.15) changes.outer_rim = scaledRim
      if (gap !== undefined && gap > scaledMaxSafeGap) changes.gap = scaledMaxSafeGap
      if (Object.keys(changes).length > 0 && !(Object.keys(changes).length === 1 && 'gap' in changes && gap !== undefined && gap > maxSafeGap)) {
        suggestions.push({
          title: `Подогнать бортики под hole = ${hole} мм (${Object.entries(changes).map(([k, v]) => `${k} → ${v} мм`).join(', ')})`,
          description: `Текущий наружный диаметр спиннера: ${outerDiameter} мм (модуль зуба ${round2(1.5 * gearScale)} мм, допустимый gap до ${maxSafeGap} мм)`,
          changes,
        })
      }
    }
  }

  return suggestions
}

