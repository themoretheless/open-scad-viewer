export function curveMeasurementLabel(operation: string, russian: boolean): string {
  const motionLabels: Record<string, [string, string]> = {
    curve_axis_motion_limits: ['Ограничения движения', 'Motion limits'],
    curve_axis_motion_slowdown: ['Замедление по оси', 'Axis slowdown'],
    curve_axis_motion_multi_slowdown: ['Замедление по нескольким осям', 'Multi-axis slowdown'],
    curve_axis_motion_duration: ['Длительность прохода', 'Traversal duration'],
    curve_axis_motion_deadline: ['Проверка срока прохода', 'Traversal deadline'],
  }
  if (Object.hasOwn(motionLabels, operation)) return motionLabels[operation]![russian ? 0 : 1]
  if (operation === 'curve_axial_derivative_peak') return russian ? 'Пик осевой производной' : 'Peak axial derivative'
  return russian ? 'Ход вдоль оси' : 'Travel along axis'
}

/** Present native bounds without rounding a small unresolved interval to zero. */
export function directionalTravelSummary(operation: string, report: string, russian: boolean): string[] {
  let parsed
  try {
    parsed = JSON.parse(report)
  } catch {
    return []
  }
  const unit = russian ? 'мм' : 'mm'
  const format = (value: number) => value.toLocaleString(russian ? 'ru' : 'en', { maximumSignificantDigits: 17 })
  const bounds = (pair: number[]) => pair[0] === pair[1] ? format(pair[0]!) : pair.map(format).join(' … ')
  const pair = (value: unknown): value is number[] => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite)
  if (operation.startsWith('curve_axis_motion_') && parsed) {
    const status = (key: unknown) => key === 'within' ? (russian ? 'В пределах лимита' : 'Within limit')
      : key === 'exceeds' ? (russian ? 'Лимит превышен' : 'Limit exceeded')
      : (russian ? 'Результат неопределён' : 'Undetermined')
    const lines: string[] = []
    if (parsed.status) lines.push(status(parsed.status))
    for (const [key, label] of [
      ['speedBounds', russian ? 'Скорость' : 'Speed'], ['accelerationBounds', russian ? 'Ускорение' : 'Acceleration'],
      ['jerkBounds', russian ? 'Рывок' : 'Jerk'], ['slowdownBounds', russian ? 'Коэффициент замедления' : 'Slowdown factor'],
      ['durationBounds', russian ? 'Длительность в единицах параметра' : 'Duration in parameter units'],
    ] as const) {
      const endpoints = parsed[key]
      if (Array.isArray(endpoints) && endpoints.length === 2) {
        lines.push(label + ' · ' + endpoints.map(x => x === 'unbounded' ? '∞' : format(x)).join(' … '))
      }
    }
    return lines
  }
  if (operation === 'curve_axial_derivative_peak') {
    if (!pair(parsed?.peakBounds) || ![1, 2, 3].includes(parsed.order)) return []
    const label = (russian ? ['Скорость', 'Ускорение', 'Рывок'] : ['Speed', 'Acceleration', 'Jerk'])[parsed.order - 1]
    const lines = [`${label} · ${bounds(parsed.peakBounds)} ${unit}/Δu^${parsed.order}`]
    if (Array.isArray(parsed.parameterIntervals)) {
      for (const interval of parsed.parameterIntervals) {
        if (pair(interval)) lines.push(`u ∈ [${interval.map(format).join(' … ')}]`)
      }
    }
    return lines
  }
  if (!['curve_measure_directional_travel', 'curve_measure_directional_travel_batch'].includes(operation)) return []
  const rows = operation.endsWith('_batch') ? parsed?.intervals : [parsed]
  if (!Array.isArray(rows)) return []
  return rows.flatMap(row => {
    if (!pair(row?.parameterInterval) || !pair(row.forwardBoundsMm) || !pair(row.reverseBoundsMm)) return []
    const status = row.reverseStatus === 'within' ? (russian ? 'в пределах лимита' : 'within limit')
      : row.reverseStatus === 'exceeds' ? (russian ? 'лимит превышен' : 'limit exceeded')
      : row.reverseStatus === 'undetermined' ? (russian ? 'сравнение с лимитом неопределённо' : 'limit comparison inconclusive') : ''
    return [`[${row.parameterInterval.map(format).join(' … ')}] · ${russian ? 'Вперёд' : 'Forward'} ${bounds(row.forwardBoundsMm)} ${unit} · ${russian ? 'Назад' : 'Reverse'} ${bounds(row.reverseBoundsMm)} ${unit}${status ? ' · ' + status : ''}`]
  })
}
