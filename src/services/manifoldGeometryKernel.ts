import Module, { type ManifoldToplevel } from 'manifold-3d/manifold'
import {
  cleanup as cleanupManifold,
  garbageCollectManifold,
} from 'manifold-3d/lib/garbage-collector.js'
import type { GeometryKernel, GeometryKernelSession } from './geometryKernel'

export interface ManifoldKernelDependencies {
  load(): Promise<ManifoldToplevel>
  instrument(module: ManifoldToplevel): ManifoldToplevel
  cleanup(): void
}

const DEFAULT_DEPENDENCIES: ManifoldKernelDependencies = {
  async load() {
    const module = await Module()
    module.setup()
    return module
  },
  instrument: garbageCollectManifold,
  cleanup: cleanupManifold,
}

/**
 * Serialized Manifold runtime adapter. Its GC registry is process-global, so
 * callers must retain the facade's queue until Manifold offers isolated
 * per-session ownership.
 */
export class ManifoldGeometryKernel implements GeometryKernel<ManifoldToplevel> {
  readonly implementationKey = 'manifold-wasm-v1'
  private modulePromise: Promise<ManifoldToplevel> | undefined

  constructor(private readonly dependencies: ManifoldKernelDependencies = DEFAULT_DEPENDENCIES) {}

  warm(): Promise<ManifoldToplevel> {
    if (!this.modulePromise) {
      const attempt = this.dependencies.load().then(module => this.dependencies.instrument(module))
      this.modulePromise = attempt
      attempt.catch(() => {
        if (this.modulePromise === attempt) this.modulePromise = undefined
      })
    }
    return this.modulePromise
  }

  async openSession(): Promise<GeometryKernelSession<ManifoldToplevel>> {
    const module = await this.warm()
    let disposed = false
    return {
      module,
      dispose: () => {
        if (disposed) return
        disposed = true
        this.dependencies.cleanup()
      },
    }
  }
}

export const defaultGeometryKernel = new ManifoldGeometryKernel()
