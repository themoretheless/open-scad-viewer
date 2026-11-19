export function curveMeasurementLabel(operation: string, russian: boolean): string {
  if (operation.includes('periodic_surface_region') || operation.includes('periodic_surface_contour_forest') || operation.includes('periodic_surface_paths')) return russian ? 'Область на периодической поверхности' : 'Periodic chart region'
  if (operation.includes('periodic_surface_path') || operation.includes('wrapped_surface_point')) return russian ? 'Измерение в периодических координатах' : 'Periodic chart measurement'
  if (operation === 'surface_trim_range_length') return russian ? 'Длина участка контура' : 'Contour range length'
  if (operation.startsWith('surface_trim_contour_')) return russian ? 'Измерение контура' : 'Contour measurement'
  if (operation === 'surface_trim_patch_area_vector') return russian ? 'Вектор площади' : 'Area vector'
  if (operation === 'surface_trim_patch_projected_area') return russian ? 'Площадь проекции' : 'Projected area'
  if (operation.startsWith('surface_trim_')) return russian ? 'Свойства обрезанной поверхности' : 'Trimmed surface properties'
  if (operation === 'surface_measure_trimmed_mass') return russian ? 'Масса обрезанных участков' : 'Trimmed surface mass'
  const motionLabels: Record<string, [string, string]> = {
    curve_axis_motion_partition_speed: ['Участки по лимиту скорости', 'Speed limit intervals'],
    curve_axis_motion_partition_motion: ['Участки по ограничениям движения', 'Motion limit intervals'],
    curve_axis_motion_partition_axes: ['Участки по нескольким осям', 'Multi-axis limit intervals'],
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
  if (operation.includes('periodic_surface_region') || operation.includes('periodic_surface_contour_forest') || operation.includes('periodic_surface_path')) {
    if (parsed?.location) return [String(parsed.location)]
    if (Array.isArray(parsed?.locations)) return [parsed.locations.join(', ')]
    if (typeof parsed?.simple === 'boolean') return [russian ? (parsed.simple ? 'Простой контур' : 'Контур с самоконтактом') : (parsed.simple ? 'Simple contour' : 'Self-contacting contour')]
    if (typeof parsed?.disjoint === 'boolean') return [russian ? (parsed.disjoint ? 'Границы раздельны' : 'Границы соприкасаются') : (parsed.disjoint ? 'Disjoint boundaries' : 'Boundary contact')]
    if (parsed?.relation?.kind) return [parsed.relation.kind]
    if (Array.isArray(parsed?.nodes)) return [(russian ? 'Контуры · ' : 'Contours · ') + parsed.nodes.length]
    if (parsed?.area) return [(russian ? 'Площадь UV · ' : 'UV area · ') + parsed.area.numerator + '/' + parsed.area.denominator]
    if (Array.isArray(parsed?.components)) return [(russian ? 'Компоненты · ' : 'Components · ') + parsed.components.length]
    if (parsed?.refusal) return [String(parsed.refusal)]
  }
  if (operation.includes('periodic_surface_path') || operation.includes('wrapped_surface_point')) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    if (parsed?.signedArea) return [(russian ? 'Ориентированная площадь UV' : 'Signed UV area') + ' · ' + parsed.signedArea.numerator + '/' + parsed.signedArea.denominator]
    if (pair(parsed?.winding)) return [(russian ? 'Обороты U, V' : 'U, V winding') + ' · ' + parsed.winding.join(', ')]
    if (Array.isArray(parsed?.point)) return ['XYZ · ' + parsed.point.map(format).join(', ') + ' ' + unit]
    const entries = parsed?.samples ?? parsed?.parameters
    return Array.isArray(entries) ? [(russian ? 'Число точек · ' : 'Points · ') + entries.length] : []
  }
  if (operation === 'surface_trim_range_length') return pair(parsed?.length?.boundsMm)
    ? [(russian ? 'Длина · ' : 'Length · ') + bounds(parsed.length.boundsMm) + ' ' + unit]
    : [russian ? 'Длина неопределённа' : 'Length unresolved']
  if (operation.startsWith('surface_trim_contour_')) {
    if (Array.isArray(parsed?.joins)) return [(russian ? 'Число стыков · ' : 'Joins · ') + parsed.joins.length]
    if (Array.isArray(parsed?.stations)) return [(russian ? 'Число точек деления · ' : 'Division points · ') + parsed.stations.length]
    if (pair(parsed?.parameterBounds)) return ['u ∈ [' + parsed.parameterBounds.map(format).join(' … ') + ']']
    return [String(parsed?.refusalStage ?? parsed?.status ?? '')]
  }
  if (operation === 'surface_trim_patch_area_vector' && Array.isArray(parsed?.boundsMm2) && parsed.boundsMm2.every(pair)) {
    return parsed.boundsMm2.map((v:number[],i:number) => ['X','Y','Z'][i] + ' · ' + bounds(v) + ' ' + unit + '²')
  }
  if (operation.startsWith('surface_trim_')) {
    return pair(parsed?.boundsMm2) ? [(russian ? 'Площадь · ' : 'Area · ') + bounds(parsed.boundsMm2) + ' ' + unit + '²']
      : [russian ? 'Площадь неопределённа' : 'Area unresolved']
  }
  if (operation === 'surface_measure_trimmed_mass') {
    if (!pair(parsed?.massBoundsKg)) return [russian ? 'Масса не определена при заданных допусках и бюджете' : 'Mass unresolved under the requested tolerances and budget']
    const status = parsed.twoPlaneBalance?.status ?? parsed.axisBalance?.status
    const verdict = status === 'within' ? (russian ? 'Балансировка в допуске' : 'Balance within tolerance')
      : status === 'outside' ? (russian ? 'Дисбаланс превышает допуск' : 'Unbalance exceeds tolerance')
      : status === 'undetermined' ? (russian ? 'Балансировка неопределённа' : 'Balance undetermined') : ''
    const axisLines: string[] = []
    if (Array.isArray(parsed.principalAxes)) {
      parsed.principalAxes.forEach((axis: {direction:number[];distanceBoundsMm?:number[];angleSineBounds?:number[]}|null,index:number) => {
        if (!axis || !Array.isArray(axis.direction) || axis.direction.length !== 3 || !axis.direction.every(Number.isFinite)) return
        const label = (russian ? 'Главная ось ' : 'Principal axis ') + (index + 1)
        axisLines.push(label + ' · [' + axis.direction.map(format).join(', ') + ']')
        if (pair(axis.distanceBoundsMm)) axisLines.push((russian ? 'Расстояние · ' : 'Distance · ') + bounds(axis.distanceBoundsMm) + ' ' + unit)
        if (pair(axis.angleSineBounds)) axisLines.push('sin θ · ' + bounds(axis.angleSineBounds))
      })
    }
    return [...axisLines,...(verdict ? [verdict] : []), (russian ? 'Общая масса · ' : 'Total mass · ') + bounds(parsed.massBoundsKg) + (russian ? ' кг' : ' kg')]
  }
  if (operation.startsWith('curve_axis_motion_') && parsed) {
    const status = (key: unknown) => key === 'within' ? (russian ? 'В пределах лимита' : 'Within limit')
      : key === 'exceeds' ? (russian ? 'Лимит превышен' : 'Limit exceeded')
      : (russian ? 'Результат неопределён' : 'Undetermined')
    const lines: string[] = []
    if (Array.isArray(parsed.cells)) {
      const counts = { within: 0, exceeds: 0, undetermined: 0 }
      for (const cell of parsed.cells) {
        const states = Array.isArray(cell?.axisStatuses) ? cell.axisStatuses.flat()
          : Array.isArray(cell?.statuses) ? cell.statuses : [cell?.status]
        if (states.includes('exceeds')) counts.exceeds++
        else if (states.length && states.every((state: string) => state === 'within')) counts.within++
        else counts.undetermined++
      }
      lines.push(russian
        ? `Участки в пределах лимита: ${counts.within}, с превышением: ${counts.exceeds}, неопределённые: ${counts.undetermined}`
        : `Intervals within limit: ${counts.within}, exceeding: ${counts.exceeds}, unresolved: ${counts.undetermined}`)
    }
    if (parsed.status) lines.push(status(parsed.status))
    for (const [key, label] of [
      ['speedBounds', russian ? 'Скорость' : 'Speed'], ['accelerationBounds', russian ? 'Ускорение' : 'Acceleration'],
      ['jerkBounds', russian ? 'Рывок' : 'Jerk'], ['slowdownBounds', russian ? 'Коэффициент замедления' : 'Slowdown factor'],
      ['durationBounds', russian ? 'Длительность в единицах параметра' : 'Duration in parameter units'],
    ] as const) {
      const endpoints = parsed[key]
      if (Array.isArray(endpoints) && endpoints.length === 2 && endpoints.every(x => Number.isFinite(x) || x === 'unbounded')) {
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
  if (operation === 'curve_directional_travel_full' && pair(parsed?.totalBoundsMm)) return [(russian ? 'Суммарный ход' : 'Total travel') + ' · ' + bounds(parsed.totalBoundsMm) + ' ' + unit]
  if (!['curve_check_backtracking','curve_directional_travel_signed','curve_measure_directional_travel', 'curve_measure_directional_travel_batch'].includes(operation)) return []
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
