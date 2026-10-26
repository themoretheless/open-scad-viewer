import type { MeshBooleanReport } from './meshEditing'

/** Localized kernel outcome, including implicit repairs that affect semantics. */
export function formatMeshBooleanNotice(report: MeshBooleanReport, locale: 'ru' | 'en'): string {
  const ru = locale === 'ru'
  const repaired = report.operandRepaired
    .map((needed, index) => needed ? ru ? `операнд ${index + 1}` : `operand ${index + 1}` : '')
    .filter(Boolean)
  return [
    ru
      ? `Булева операция ${report.operation} выполнена: результат manifold, треугольников: ${report.outputTriangles}.`
      : `Boolean ${report.operation} completed: manifold result, ${report.outputTriangles} triangles.`,
    repaired.length
      ? `${ru ? 'Full pre-repair' : 'Full pre-repair'}: ${repaired.join(', ')}.`
      : ru ? 'Предварительный ремонт операндов не потребовался.' : 'Operand pre-repair was not required.',
    ru
      ? `Ядро: work ${report.work}, fragments ${report.fragments}, допуск ${report.toleranceMm} мм.`
      : `Kernel: work ${report.work}, fragments ${report.fragments}, tolerance ${report.toleranceMm} mm.`,
  ].join(' ')
}
