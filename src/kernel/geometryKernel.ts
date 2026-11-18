/** A disposable lease over one geometry-kernel runtime. */
export interface GeometryKernelSession<TModule> {
  readonly module: TModule
  readonly handles: KernelHandleTable
  dispose(): void
}

/** Kernel lifecycle boundary; language semantics remain compiler-owned. */
export interface GeometryKernel<TModule> {
  readonly implementationKey: string
  warm(): Promise<TModule>
  openSession(): Promise<GeometryKernelSession<TModule>>
}

/** Opaque solid identity. The payload lives only in a session handle table. */
export interface KernelSolidHandle {
  readonly type: 'solid'
  readonly id: number
}

/** Opaque planar-section identity. The payload lives only in a session handle table. */
export interface KernelSectionHandle {
  readonly type: 'section'
  readonly id: number
}

export type KernelGeometryHandle = KernelSolidHandle | KernelSectionHandle

const SOLID_TYPE = 'solid' as const
const SECTION_TYPE = 'section' as const

export function isKernelSolidHandle(value: unknown): value is KernelSolidHandle {
  return isHandle(value, SOLID_TYPE)
}

export function isKernelSectionHandle(value: unknown): value is KernelSectionHandle {
  return isHandle(value, SECTION_TYPE)
}

function isHandle(value: unknown, type: 'solid' | 'section'): boolean {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false
  const keys = Reflect.ownKeys(value)
  if (keys.length !== 2 || !keys.includes('type') || !keys.includes('id')) return false
  const candidate = value as { type: unknown; id: unknown }
  return candidate.type === type && Number.isInteger(candidate.id) && (candidate.id as number) > 0
}

/**
 * Session-local owner of kernel geometry. Callers receive frozen handles and
 * must not retain the underlying solid/section objects across dispose.
 */
export class KernelHandleTable<TSolid = unknown, TSection = unknown> {
  private nextId = 1
  private readonly solids = new Map<number, TSolid>()
  private readonly sections = new Map<number, TSection>()

  adoptSolid(value: TSolid): KernelSolidHandle {
    const id = this.nextId++
    this.solids.set(id, value)
    return Object.freeze({ type: SOLID_TYPE, id })
  }

  adoptSection(value: TSection): KernelSectionHandle {
    const id = this.nextId++
    this.sections.set(id, value)
    return Object.freeze({ type: SECTION_TYPE, id })
  }

  requireSolid(handle: KernelSolidHandle): TSolid {
    const value = this.solids.get(handle.id)
    if (value === undefined || handle.type !== SOLID_TYPE) {
      throw new Error(`Unknown kernel solid handle ${handle.id}`)
    }
    return value
  }

  requireSection(handle: KernelSectionHandle): TSection {
    const value = this.sections.get(handle.id)
    if (value === undefined || handle.type !== SECTION_TYPE) {
      throw new Error(`Unknown kernel section handle ${handle.id}`)
    }
    return value
  }

  get size(): number {
    return this.solids.size + this.sections.size
  }

  clear(): void {
    this.solids.clear()
    this.sections.clear()
  }
}
