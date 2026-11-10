import Module, { type CadSolid, type CadToplevel } from './geometry/module'
import { warmGeometryKernel } from './geometry/kernel'
import { KernelHandleTable, type GeometryKernel, type GeometryKernelSession } from './geometryKernel'
import { createCadKernelOps, type CadKernelOps } from './cadKernelOps'

export interface CadKernelDependencies {
  load(): Promise<CadToplevel>
  instrument(module: CadToplevel): CadToplevel
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

let instrumentedRawCadSolid: typeof CadSolid | undefined

/**
 * setup() exposes factory constructors whose .prototype is a child of the
 * actual Embind instance prototype. Instrument the prototype of real objects:
 * wrapping only the exported factory prototype silently misses every member
 * result. Keep ownership private to this WASM module and preserve the receiver.
 */
function ownModuleGeometry(module: CadToplevel): () => void {
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
  const solid = module.CadSolid.cube([0, 0, 0])
  const section = module.CrossSection.square([0, 0])
  const solidPrototype: object = Object.getPrototypeOf(solid)
  const sectionPrototype: object = Object.getPrototypeOf(section)
  const RawCadSolid = solidPrototype.constructor as typeof module.CadSolid
  solid.delete()
  section.delete()
  wrapMethods(solidPrototype, MANIFOLD_METHODS)
  wrapMethods(sectionPrototype, CROSS_SECTION_METHODS)
  wrapMethods(module.CadSolid, MANIFOLD_FACTORIES)
  wrapMethods(module.CrossSection, CROSS_SECTION_FACTORIES)
  // ofMesh/ofPolygons call the public constructor; a Set deduplicates results
  // also observed by their surrounding factory/member wrappers.
  instrumentedRawCadSolid = RawCadSolid
  const constructSolid = (args: unknown[]): CadSolid => {
    // The convenience constructor throws on a failed status after
    // creating a native handle. Capture that handle before checking its status.
    const geometry = track(Reflect.construct(RawCadSolid, args)) as CadSolid & OwnedGeometry
    try {
      const status = geometry.status()
      if (status !== 'NoError') {
        throw new module.CadError(status)
      }
      return geometry
    } catch (error) {
      owned.delete(geometry)
      try { if (!geometry.isDeleted()) geometry.delete() }
      catch (cleanupError) { throw new AggregateError([error, cleanupError], 'Invalid solid cleanup failed') }
      throw error
    }
  }
  module.CadSolid = new Proxy(module.CadSolid, {
    apply: (_target, _receiver, args) => constructSolid(args),
    construct: (_target, args) => constructSolid(args),
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
    if (errors.length) throw new AggregateError(errors, 'CAD geometry cleanup failed')
  }
}

export interface GeometryEvalSession {
  readonly kernel: CadKernelOps
  dispose(): void
}

function defaultDependencies(): CadKernelDependencies {
  let cleanup = () => {}
  return {
    async load() {
      await warmGeometryKernel()
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
 * Serialized own-Rust CAD adapter. Each kernel has its own WASM module and
 * ownership registry; callers serialize leases within that kernel's module.
 */
export class CadGeometryKernel implements GeometryKernel<CadToplevel> {
  readonly implementationKey = 'own-rust-cad-v1'
  private modulePromise: Promise<CadToplevel> | undefined

  constructor(private readonly dependencies: CadKernelDependencies = defaultDependencies()) {}

  warm(): Promise<CadToplevel> {
    if (!this.modulePromise) {
      const attempt = this.dependencies.load().then(module => this.dependencies.instrument(module))
      this.modulePromise = attempt
      attempt.catch(() => {
        if (this.modulePromise === attempt) this.modulePromise = undefined
      })
    }
    return this.modulePromise
  }

  async openSession(): Promise<GeometryKernelSession<CadToplevel>> {
    const module = await this.warm()
    const handles = new KernelHandleTable()
    let disposed = false
    return {
      module,
      handles,
      dispose: () => {
        if (disposed) return
        disposed = true
        handles.clear()
        this.dependencies.cleanup()
      },
    }
  }

  async openEvalSession(): Promise<GeometryEvalSession> {
    const session = await this.openSession()
    const raw = instrumentedRawCadSolid ?? session.module.CadSolid
    return {
      kernel: createCadKernelOps(session.module, raw),
      dispose: () => session.dispose(),
    }
  }
}

export const defaultGeometryKernel = new CadGeometryKernel()
