import type { SketchCurveConstraint } from './sketchCurveConstraints'

const CURVE_CONSTRAINT_NAMES: Record<SketchCurveConstraint['kind'], [string, string]> = {
  concentric: ['концентричность', 'concentric'],
  equal_radius: ['равный радиус', 'equal radius'],
  tangent_circle: ['касание окружностей', 'tangent circles'],
  tangent_line: ['касание отрезка', 'tangent to segment'],
  coincident: ['совпадение точек', 'coincident'],
}

/** Localized constraint name for panels and notices. */
export function sketchCurveConstraintName(c: SketchCurveConstraint, locale: string): string {
  const [ru, en] = CURVE_CONSTRAINT_NAMES[c.kind]
  const internal = c.kind === 'tangent_circle' && c.internal
  return locale === 'ru' ? `${ru}${internal ? ' (внутреннее)' : ''}` : `${en}${internal ? ' (internal)' : ''}`
}

export function droppedCurveConstraintsNotice(dropped: readonly SketchCurveConstraint[], locale: string): string {
  if (!dropped.length) return ''
  const names = dropped.map(c => sketchCurveConstraintName(c, locale)).join(', ')
  return locale === 'ru'
    ? `Обрезка сняла ограничения окружностей и дуг: ${names}.`
    : `Trim removed circle and arc constraints: ${names}.`
}

export const CURVE_DRAG_REFUSED_MESSAGE =
  'This circle or arc cannot be dragged there: its curve constraints cannot be satisfied. The drag was refused.'

/** Localized notice for a refused circle or arc handle drag. */
export function curveDragRefusedNotice(locale: string): string {
  return locale === 'ru'
    ? 'Окружность или дугу нельзя перетащить сюда: её ограничения не выполнимы. Перетаскивание отменено.'
    : CURVE_DRAG_REFUSED_MESSAGE
}

export const COINCIDENT_EDIT_REFUSED_MESSAGE =
  'This polyline cannot be edited this way: its coincident constraints cannot be satisfied. The edit was refused.'

/** Localized notice for a refused edit of a polyline with coincident constraints. */
export function coincidentEditRefusedNotice(locale: string): string {
  return locale === 'ru'
    ? 'Ломаную нельзя так изменить: её ограничения совпадения не выполнимы. Изменение отменено.'
    : COINCIDENT_EDIT_REFUSED_MESSAGE
}
