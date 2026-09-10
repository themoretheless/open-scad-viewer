import Module, { type Manifold, type ManifoldToplevel } from './geometry/module'
import type { GeometryKernel, GeometryKernelSession } from './geometryKernel'

export interface ManifoldKernelDependencies {
  load(): Promise<ManifoldToplevel>
  instrument(module: ManifoldToplevel): ManifoldToplevel
  cleanup(): void
}

interface OwnedGeometry {
  delete(): void
  isDeleted(): boolean
}

function isOwnedGeometry(value: unknown): value is OwnedGeometry {
  return value !== null && typeof value === 'object'
    && typeof (value as Partial<OwnedGeometry>).delete === 'function'
    && typeof (value as Partial<OwnedGeometry>).isDeleted === 'function'
}

const MANIFOLD_FACTORIES = [
  'cube', 'cylinder', 'sphere', 'tetrahedron', 'extrude', 'revolve', 'compose',
  'union', 'difference', 'intersection', 'levelSet', 'smooth', 'ofMesh', 'hull',
]
const MANIFOLD_METHODS = [
  'add', 'subtract', 'intersect', 'decompose', 'warp', 'warpBatch', 'transform',
  'translate', 'rotate', 'scale', 'mirror', 'calculateCurvature', 'calculateNormals',
  'smoothByNormals', 'smoothOut', 'refine', 'refineToLength', 'refineToTolerance',
  'setProperties', 'setTolerance', 'simplify', 'asOriginal', 'trimByPlane', 'split',
  'splitByPlane', 'slice', 'project', 'hull', 'minkowskiSum', 'minkowskiDifference',
  'withContext',
]
const CROSS_SECTION_FACTORIES = ['square', 'circle', 'union', 'difference', 'intersection', 'compose', 'ofPolygons', 'hull']
const CROSS_SECTION_METHODS = [
  'add', 'subtract', 'intersect', 'rectClip', 'decompose', 'transform', 'translate',
  'rotate', 'scale', 'mirror', 'simplify', 'offset', 'hull', 'warp', 'extrude', 'revolve',
]

/**
 * setup() exposes factory constructors whose .prototype is a child of the
 * actual Embind instance prototype. Instrument the prototype of real objects:
 * wrapping only the exported factory prototype silently misses every member
 * result. Keep ownership private to this WASM module and preserve the receiver.
 */
function ownModuleGeometry(module: ManifoldToplevel): () => void {
  const owned = new Set<OwnedGeometry>()
  const track = <T>(value: T): T => {
    if (Array.isArray(value)) {
      for (const item of value) track(item)
    } else if (isOwnedGeometry(value)) {
      owned.add(value)
    }
    return value
  }
  const wrapMethods = (target: object, names: readonly string[]) => {
    const methods = target as Record<string, unknown>
    for (const name of names) {
      const original = methods[name]
      if (typeof original !== 'function') continue
      methods[name] = function (this: unknown, ...args: unknown[]) {
        // Some convenience methods create a raw intermediate and then call a
        // public member on it (e.g. centered extrusion -> translate).
        track(this)
        return track(Reflect.apply(original, this, args))
      }
    }
  }
  const solid = module.Manifold.cube([0, 0, 0])
  const section = module.CrossSection.square([0, 0])
  const solidPrototype: object = Object.getPrototypeOf(solid)
  const sectionPrototype: object = Object.getPrototypeOf(section)
  const RawManifold = solidPrototype.constructor as typeof module.Manifold
  solid.delete()
  section.delete()
  wrapMethods(solidPrototype, MANIFOLD_METHODS)
  wrapMethods(sectionPrototype, CROSS_SECTION_METHODS)
  wrapMethods(module.Manifold, MANIFOLD_FACTORIES)
  wrapMethods(module.CrossSection, CROSS_SECTION_FACTORIES)
  // ofMesh/ofPolygons call the public constructor; a Set deduplicates results
  // also observed by their surrounding factory/member wrappers.
  const constructManifold = (args: unknown[]): Manifold => {
    // The upstream convenience constructor throws on a failed status after
    // creating a native handle. Capture that handle before checking its status.
    const geometry = track(Reflect.construct(RawManifold, args)) as Manifold & OwnedGeometry
    try {
      const status = geometry.status()
      if (status !== 'NoError') {
        const ManifoldError = (module as unknown as { ManifoldError: new (status: string) => Error }).ManifoldError
        throw new ManifoldError(status)
      }
      return geometry
    } catch (error) {
      owned.delete(geometry)
      try { if (!geometry.isDeleted()) geometry.delete() }
      catch (cleanupError) { throw new AggregateError([error, cleanupError], 'Invalid Manifold cleanup failed') }
      throw error
    }
  }
  module.Manifold = new Proxy(module.Manifold, {
    apply: (_target, _receiver, args) => constructManifold(args),
    construct: (_target, args) => constructManifold(args),
  })
  module.CrossSection = new Proxy(module.CrossSection, {
    apply: (target, receiver, args) => track(Reflect.apply(target, receiver, args)),
    construct: (target, args, newTarget) => track(Reflect.construct(target, args, newTarget)),
  })
  return () => {
    const pending = [...owned]
    owned.clear()
    const errors: unknown[] = []
    for (let index = pending.length - 1; index >= 0; index--) {
      try {
        if (!pending[index].isDeleted()) pending[index].delete()
      } catch (error) { errors.push(error) }
    }
    if (errors.length) throw new AggregateError(errors, 'Manifold geometry cleanup failed')
  }
}

function defaultDependencies(): ManifoldKernelDependencies {
  let cleanup = () => {}
  return {
    async load() {
      const module = await Module()
      module.setup()
      return module
    },
    instrument(module) {
      cleanup = ownModuleGeometry(module)
      return module
    },
    cleanup: () => cleanup(),
  }
}

/**
 * Serialized Manifold runtime adapter. Each kernel has its own WASM module and
 * ownership registry; callers serialize leases within that kernel's module.
 */
export class ManifoldGeometryKernel implements GeometryKernel<ManifoldToplevel> {
  readonly implementationKey = 'own-rust-cad-v1'
  private modulePromise: Promise<ManifoldToplevel> | undefined

  constructor(private readonly dependencies: ManifoldKernelDependencies = defaultDependencies()) {}

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
