import type { DisplayMode } from './rendererContracts'

export type RenderingBackendTier = 'webgpu-interactive' | 'headless-geometry'
export type TransparencyTier = 'opaque-depth' | 'object-sorted-alpha' | 'unavailable'

export interface BackendQuality {
  readonly backend: RenderingBackendTier
  readonly geometryBuilds: 'full'
  readonly interactiveViewport: boolean
  readonly transparency: TransparencyTier
}

export function backendQuality(webGpuReady: boolean, displayMode: DisplayMode): BackendQuality {
  if (!webGpuReady) {
    return Object.freeze({
      backend: 'headless-geometry' as const,
      geometryBuilds: 'full' as const,
      interactiveViewport: false,
      transparency: 'unavailable' as const,
    })
  }
  return Object.freeze({
    backend: 'webgpu-interactive' as const,
    geometryBuilds: 'full' as const,
    interactiveViewport: true,
    transparency: displayMode === 'xray' ? 'object-sorted-alpha' as const : 'opaque-depth' as const,
  })
}

export function usesTransparentPass(alpha: number, displayMode: DisplayMode): boolean {
  return isTransparentAlpha(effectiveDisplayAlpha(alpha, displayMode))
}

export function effectiveDisplayAlpha(alpha: number, displayMode: DisplayMode): number {
  return displayMode === 'xray' ? Math.min(alpha, 0.24) : alpha
}

export function isTransparentAlpha(alpha: number): boolean {
  return Number.isFinite(alpha) && alpha < 1
}
