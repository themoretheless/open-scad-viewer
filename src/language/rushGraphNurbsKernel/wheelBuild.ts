import { callGeometryRust } from '../../kernel/geometry/kernel'
import type { NurbsBrep } from '../../kernel/geometry/brep'
import type { OwnNurbsBuildControl } from '../rushGraphNurbsKernel'

export interface WheelResult {
  model: NurbsBrep
  wallErrorUpperMm: number
  solidGeometryCertified: boolean
}
export type WheelResolver = (nodeId: string, spec: object) => WheelResult

export class PendingWheel {
  constructor(readonly nodeId: string, readonly spec: object) {}
}

export async function buildWheel(spec: object, control: OwnNurbsBuildControl): Promise<WheelResult> {
  const job = callGeometryRust<number>('za_wheel_start', spec)
  try {
    for (;;) {
      control.onYield?.()
      await new Promise<void>(resolve => setTimeout(resolve, 0))
      if (control.shouldAbort?.()) throw new DOMException('Build cancelled', 'AbortError')
      const result = callGeometryRust<WheelResult | null>('za_wheel_step', {job})
      if (result) return result
    }
  } finally {
    callGeometryRust('za_wheel_cancel', {job})
  }
}
