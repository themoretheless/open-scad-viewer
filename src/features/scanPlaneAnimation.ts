export type ScanDirection = 1 | -1

/** Advance a scan while preserving overshoot when it reflects at either endpoint. */
export function advanceScanPlane(
  offset: number,
  direction: ScanDirection,
  min: number,
  max: number,
  elapsedMs: number,
  oneWayMs = 8_000,
): { offset: number; direction: ScanDirection } {
  if (![offset, min, max, elapsedMs, oneWayMs].every(Number.isFinite) || max < min || oneWayMs <= 0) {
    throw new RangeError('Scan range and timing must be finite and ordered')
  }
  const range = max - min
  if (range === 0) return { offset: min, direction: 1 }
  const position = Math.min(range, Math.max(0, offset - min))
  const phase = direction === 1 ? position : 2 * range - position
  const next = (phase + range * Math.max(0, elapsedMs) / oneWayMs) % (2 * range)
  return {
    offset: min + (next <= range ? next : 2 * range - next),
    direction: next < range ? 1 : -1,
  }
}
