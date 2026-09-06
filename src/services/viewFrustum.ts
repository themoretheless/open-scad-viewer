import type { Mat4 } from './math3d'

/** Conservative world-space sphere culling for WebGPU's -w..w XY, 0..w Z. */
export class ViewFrustum {
  private readonly planes = new Float64Array(24)

  update(matrix: Mat4) {
    for (let plane = 0; plane < 6; plane++) {
      const row = plane < 4 ? (plane >> 1) * 4 : 8
      const sign = plane % 2 === 0 ? 1 : -1
      const offset = plane * 4
      for (let column = 0; column < 4; column++) {
        this.planes[offset + column] = plane === 4 ? matrix[8 + column]
          : matrix[12 + column] + sign * matrix[row + column]
      }
      const length = Math.hypot(this.planes[offset], this.planes[offset + 1], this.planes[offset + 2])
      if (length > 0 && Number.isFinite(length)) {
        for (let column = 0; column < 4; column++) this.planes[offset + column] /= length
      } else this.planes.fill(0, offset, offset + 4)
    }
  }

  intersects(center: readonly number[], radius: number) {
    // Keep boundary objects despite Float32 transform/projection rounding.
    const tolerance = 1e-5 * (1 + Math.abs(center[0]) + Math.abs(center[1]) + Math.abs(center[2]) + radius)
    for (let offset = 0; offset < 24; offset += 4) {
      const distance = this.planes[offset] * center[0] + this.planes[offset + 1] * center[1]
        + this.planes[offset + 2] * center[2] + this.planes[offset + 3]
      if (distance < -radius - tolerance) return false
    }
    return true
  }
}
