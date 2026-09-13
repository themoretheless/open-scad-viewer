import { SVG_MAX_BYTES, SVG_MAX_FONTS, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES } from './svgLimits'

export interface SvgDraft {
  version: 1
  text: string
  filename: string
  height: number | string
  axis: 'x' | 'y' | 'z'
  dpi: number | string
  tolerance: number | string
  geometryMode: 'vector' | 'silhouette'
  rasterSize: number | string
  alphaThreshold: number | string
  fonts: { name: string; bytes: Uint8Array }[]
  opened: boolean
}
export const SVG_DRAFT_DATABASE = 'open-scad-viewer-svg'
export const SVG_DRAFT_STORE = 'drafts'
export class SvgDraftStorageError extends Error {
  constructor(readonly code: 'unavailable' | 'conflict' | 'invalid', message: string) { super(message); this.name = 'SvgDraftStorageError' }
}
// A draft preserves unfinished controls too; geometry validates their values when an operation runs.
const numericDraft = (value: unknown) => typeof value === 'number' && Number.isFinite(value) || typeof value === 'string' && value.length <= 64
export function validSvgDraft(value: unknown): value is SvgDraft {
  if (!value || typeof value !== 'object') return false
  const draft = value as Partial<SvgDraft>
  return draft.version === 1 && typeof draft.text === 'string' && draft.text.length <= SVG_MAX_BYTES && new TextEncoder().encode(draft.text).length <= SVG_MAX_BYTES
    && typeof draft.filename === 'string' && draft.filename.length <= 512 && typeof draft.opened === 'boolean'
    && numericDraft(draft.height) && ['x', 'y', 'z'].includes(draft.axis ?? '')
    && numericDraft(draft.dpi) && numericDraft(draft.tolerance)
    && ['vector', 'silhouette'].includes(draft.geometryMode ?? '') && numericDraft(draft.rasterSize)
    && numericDraft(draft.alphaThreshold) && Array.isArray(draft.fonts) && draft.fonts.length <= SVG_MAX_FONTS
    && draft.fonts.every(font => font && typeof font.name === 'string' && font.name.length <= 512 && font.bytes instanceof Uint8Array && font.bytes.byteLength > 0 && font.bytes.byteLength <= SVG_MAX_FONT_BYTES)
    && draft.fonts.reduce((sum, font) => sum + font.bytes.byteLength, 0) <= SVG_MAX_TOTAL_FONT_BYTES
}
function completed(transaction: IDBTransaction) {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onabort = transaction.onerror = () => reject(transaction.error ?? new Error('SVG draft could not be saved.'))
  })
}
function result<T>(request: IDBRequest<T>) {
  return new Promise<T>((resolve, reject) => { request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error) })
}

/** IndexedDB keeps fonts and large SVGs out of localStorage; RAM preserves edits across component remounts when storage fails. */
export class SvgDraftStore {
  private cached: SvgDraft | null = null
  private token: string | null = null
  private loading: Promise<SvgDraft | null> | null = null
  private queue: Promise<void> = Promise.resolve()
  constructor(private readonly factory: IDBFactory | undefined = globalThis.indexedDB) {}
  async load(): Promise<SvgDraft | null> {
    if (this.cached) return this.cached
    return this.loading ??= this.read()
  }
  private async read(): Promise<SvgDraft | null> {
    const db = await this.open()
    try {
      const transaction = db.transaction(SVG_DRAFT_STORE, 'readonly')
      const [row] = await Promise.all([result(transaction.objectStore(SVG_DRAFT_STORE).get('active')), completed(transaction)])
      if (row === undefined) { this.token = null; return this.cached }
      if (!row || typeof row.token !== 'string' || !validSvgDraft(row.draft)) throw new SvgDraftStorageError('invalid', 'The saved SVG draft is invalid or from an unsupported version.')
      this.token = row.token
      // A user edit saved while this read was pending takes precedence.
      this.cached ??= row.draft
      return this.cached
    } finally { db.close() }
  }
  save(draft: SvgDraft): Promise<void> {
    if (!validSvgDraft(draft)) return Promise.reject(new SvgDraftStorageError('invalid', 'SVG draft settings or size are outside the supported limits.'))
    this.cached = draft
    const save = this.queue.catch(() => {}).then(async () => { await this.loading?.catch(() => {}); await this.write(draft) })
    this.queue = save
    return save
  }
  private async write(draft: SvgDraft) {
    const db = await this.open()
    try {
      const transaction = db.transaction(SVG_DRAFT_STORE, 'readwrite'), done = completed(transaction), store = transaction.objectStore(SVG_DRAFT_STORE)
      void done.catch(() => {})
      const row = await result(store.get('active'))
      if ((row?.token ?? null) !== this.token) {
        await done
        throw new SvgDraftStorageError('conflict', 'The SVG draft changed in another tab. Download your SVG before reloading.')
      }
      const token = `${Date.now()}-${Math.random().toString(36).slice(2)}`
      store.put({ token, draft }, 'active')
      await done
      this.token = token
    } finally { db.close() }
  }
  private open(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      if (!this.factory) { reject(new SvgDraftStorageError('unavailable', 'SVG draft storage is unavailable. Changes are kept for this session only.')); return }
      let settled = false
      const fail = (error: unknown) => { if (settled) return; settled = true; clearTimeout(timer); reject(error) }
      const timer = setTimeout(() => fail(new SvgDraftStorageError('unavailable', 'SVG draft storage did not respond. Changes are kept for this session only.')), 3000)
      let request: IDBOpenDBRequest
      try { request = this.factory.open(SVG_DRAFT_DATABASE, 1) }
      catch (error) { fail(error); return }
      request.onupgradeneeded = () => { if (!request.result.objectStoreNames.contains(SVG_DRAFT_STORE)) request.result.createObjectStore(SVG_DRAFT_STORE) }
      request.onerror = () => fail(request.error)
      request.onblocked = () => fail(new SvgDraftStorageError('unavailable', 'SVG draft storage is blocked by another tab.'))
      request.onsuccess = () => {
        if (settled) { request.result.close(); return }
        settled = true; clearTimeout(timer)
        request.result.onversionchange = () => request.result.close()
        resolve(request.result)
      }
    })
  }
}
let shared: SvgDraftStore | undefined
export function svgDraftStore() { return shared ??= new SvgDraftStore() }
