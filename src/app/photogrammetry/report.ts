import type {PhotoMeasuredInput} from './calibration'
import type {PhotoDensePreset, PhotoDenseDiagnostics, PhotoDiagnostics} from './kernel'
import type {PhotoTimings} from './workerProtocol'

export interface PhotoReportInput {
  image: number
  name: string
  bytes: number
  equivalent: number
  width: number
  height: number
  focalPixels: number
  calibration?: PhotoMeasuredInput
}
export interface PhotoRunReport {
  settings: {dense: boolean; resolution: number; maxImageSide: number; densePreset?: PhotoDensePreset}
  inputs: readonly PhotoReportInput[]
  sparse: PhotoDiagnostics | null
  dense: PhotoDenseDiagnostics | null
  timings: PhotoTimings | null
}

/** Metadata-only record: originals and pixel buffers are never included in this download. */
export function photoReportJson(report: PhotoRunReport): string {
  return JSON.stringify({format: 'open-scad-viewer/photo-report', version: 1, ...report}, null, 2) + '\n'
}

export function photoRegistrationReason(reason: string, russian: boolean): string {
  const labels: Record<string, [string, string]> = {
    registered: ['Связан с моделью', 'Registered'],
    cancelled: ['Обработка отменена', 'Processing cancelled'],
    invalid_options: ['Некорректные настройки', 'Invalid settings'],
    invalid_image: ['Некорректные данные снимка', 'Invalid image data'],
    insufficient_consistent_observations: ['Мало точек после проверки геометрии', 'Too few geometrically consistent observations'],
    insufficient_features: ['Мало различимых деталей', 'Too few distinctive features'],
    initialization_failed: ['Не удалось выбрать начальную пару', 'Initial pair could not be reconstructed'],
    insufficient_correspondences: ['Мало связей с точками 3D', 'Too few 2D–3D correspondences'],
    pose_rejected: ['Положение камеры не прошло проверку', 'Camera pose failed verification'],
    not_connected: ['Нет связи с восстановленными ракурсами', 'Not connected to registered views'],
    not_processed: ['Кадр не обработан', 'Frame not processed'],
  }
  return labels[reason]?.[russian ? 0 : 1] ?? reason
}
