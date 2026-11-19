/** Present native bounds without rounding a small unresolved interval to zero. */
export function directionalTravelSummary(operation: string, report: string, russian: boolean): string[] {
  if (!['curve_measure_directional_travel', 'curve_measure_directional_travel_batch'].includes(operation)) return []
  let parsed
  try {
    parsed = JSON.parse(report)
  } catch {
    return []
  }
  const rows = operation.endsWith('_batch') ? parsed?.intervals : [parsed]
  if (!Array.isArray(rows)) return []
  const unit = russian ? 'мм' : 'mm'
  const format = (value: number) => value.toLocaleString(russian ? 'ru' : 'en', { maximumSignificantDigits: 17 })
  const bounds = (pair: number[]) => pair[0] === pair[1] ? format(pair[0]!) : pair.map(format).join(' … ')
  const pair = (value: unknown): value is number[] => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite)
  return rows.flatMap(row => {
    if (!pair(row?.parameterInterval) || !pair(row.forwardBoundsMm) || !pair(row.reverseBoundsMm)) return []
    const status = row.reverseStatus === 'within' ? (russian ? 'в пределах лимита' : 'within limit')
      : row.reverseStatus === 'exceeds' ? (russian ? 'лимит превышен' : 'limit exceeded')
      : row.reverseStatus === 'undetermined' ? (russian ? 'сравнение с лимитом неопределённо' : 'limit comparison inconclusive') : ''
    return [`[${row.parameterInterval.map(format).join(' … ')}] · ${russian ? 'Вперёд' : 'Forward'} ${bounds(row.forwardBoundsMm)} ${unit} · ${russian ? 'Назад' : 'Reverse'} ${bounds(row.reverseBoundsMm)} ${unit}${status ? ' · ' + status : ''}`]
  })
}
