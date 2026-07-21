/**
 * SafeStorage — the single access point for localStorage.
 *
 * Every read validates and falls back; every write catches quota errors and
 * reports them through an injectable callback (dependency inversion: the UI
 * layer registers a notice handler, this module stays framework-free).
 * Silently swallowing QuotaExceededError would lose user data without a word.
 */

export type StorageFailureHandler = (key: string, error: unknown) => void

let onWriteFailure: StorageFailureHandler | null = null

/** UI layer registers a handler (e.g. show a notice) for failed writes. */
export function setStorageFailureHandler(handler: StorageFailureHandler | null) {
  onWriteFailure = handler
}

/** Raw string read; never throws (private mode / disabled storage safe). */
export function storageGet(key: string): string | null {
  try { return localStorage.getItem(key) } catch { return null }
}

/**
 * Raw string write; never throws. Returns true on success.
 * On failure invokes the registered failure handler so the user is told
 * their data was NOT saved instead of silently losing it.
 */
export function storageSet(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value)
    return true
  } catch (error) {
    onWriteFailure?.(key, error)
    return false
  }
}

/** JSON read with fallback and optional shape validation. */
export function storageGetJSON<T>(key: string, fallback: T, validate?: (value: unknown) => boolean): T {
  const raw = storageGet(key)
  if (raw === null) return fallback
  try {
    const value = JSON.parse(raw) as unknown
    if (value === null || value === undefined) return fallback
    if (validate && !validate(value)) return fallback
    return value as T
  } catch {
    return fallback
  }
}

/** JSON write; returns true on success, reports failures like storageSet. */
export function storageSetJSON(key: string, value: unknown): boolean {
  return storageSet(key, JSON.stringify(value))
}

/** Enum read: only returns the stored value if it is one of `allowed`. */
export function storageGetEnum<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  const raw = storageGet(key)
  return (allowed as readonly string[]).includes(raw ?? '') ? (raw as T) : fallback
}
