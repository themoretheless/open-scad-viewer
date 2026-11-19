export function curveMeasurementLabel(operation: string, russian: boolean): string {
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
