import { nextTick, onUnmounted, ref, type Ref, type ShallowRef } from 'vue'
import {
  emptyDirectDocument,
  MAX_DRAFT_CHARACTERS,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectDocument,
  type DirectHistory,
} from '../../services/directModeling'
import { isGeometryKernelReady, warmGeometryKernel } from '../../services/geometry/kernel'
import { storageGet, storageSet } from '../../services/safeStorage'
import { readSolidDraftHead, writeSolidDraftHead } from '../../services/solidDraftHeadStore'
import {
  collectSolidDraftSnapshots,
  readSolidDraftSnapshot,
  removeSolidDraftSnapshot,
  solidDraftSnapshotId,
  withSolidDraftLock,
  writeSolidDraftSnapshot,
} from '../../services/solidDraftStore'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import type { Pane } from './helpers'

export interface UseDraftPersistenceOptions {
  key: string
  legacyKey: string
  props: { open: boolean; initialDocument?: DirectDocument }
  label: (ruText: string, enText: string) => string
  error: Ref<string>
  workspace: Ref<HTMLElement | undefined>
  historyWorker: ReturnType<typeof createSolidPreviewWorker>
  historyPending: Ref<boolean>
  historyMode: Ref<'restore' | 'import' | 'export' | 'shared'>
  bumpHistoryGeneration: () => number
  getHistoryGeneration: () => number
  getHistory: () => DirectHistory
  getDocument: () => ShallowRef<DirectDocument>
  getSnapDocument: () => ShallowRef<DirectDocument>
  getCancelCommand: () => () => void
  getSync: () => (next?: DirectDocument) => void
  getFit: () => (pane: Pane, selectedOnly?: boolean) => void
}

export function useDraftPersistence(options: UseDraftPersistenceOptions) {
  const {
    key,
    legacyKey,
    props,
    label,
    error,
    workspace,
    historyWorker,
    historyPending,
    historyMode,
    bumpHistoryGeneration,
    getHistoryGeneration,
    getHistory,
    getDocument,
    getSnapDocument,
    getCancelCommand,
    getSync,
    getFit,
  } = options

  const saveError = ref(false)
  const savePending = ref(false)
  const draftConflict = ref(false)
  const loadingLatestDraft = ref(false)
  let persistedRevision: string | null = null
  const durableReady = readSolidDraftHead(key).then(head => {
    persistedRevision = head?.revision ?? null
    return head
  })
  void durableReady.catch(() => {})
  let saveGeneration = 0
  let persistedReference = storageGet(key)
  let stored = persistedReference ?? storageGet(legacyKey)
  const restoringDraft = ref(
    !props.initialDocument &&
      (typeof indexedDB !== 'undefined' ||
        (!!stored && (!isGeometryKernelReady() || !!solidDraftSnapshotId(stored)))),
  )
  let restoreDisposed = false
  onUnmounted(() => {
    restoreDisposed = true
  })

  let initial = emptyDirectDocument()
  try {
    if (
      !restoringDraft.value &&
      stored &&
      isGeometryKernelReady() &&
      !solidDraftSnapshotId(stored) &&
      !props.initialDocument
    ) {
      initial = parseDirectDocument(stored)
    }
  } catch {
    saveError.value = true
    error.value = label(
      'Сохранённый JSON повреждён. Импортируйте резервный JSON через меню «Файл».',
      'Saved JSON is corrupt. Import a JSON backup from the File menu.',
    )
  }
  if (props.initialDocument) initial = props.initialDocument

  function draftConflictMessage() {
    return label(
      'Документ изменён в другой вкладке. Скачайте JSON своих правок или загрузите сохранённую версию; Undo вернёт локальную модель.',
      'Document changed in another tab. Download your edits as JSON or load the saved version; Undo restores your local model.',
    )
  }

  function noteExternalDraft() {
    if (storageGet(key) !== persistedReference) {
      draftConflict.value = true
      saveError.value = true
      error.value = draftConflictMessage()
    }
  }

  function onDraftStorage(event: StorageEvent) {
    if (event.key === key || event.key === null) noteExternalDraft()
  }
  window.addEventListener?.('storage', onDraftStorage)
  onUnmounted(() => window.removeEventListener?.('storage', onDraftStorage))
  if (typeof indexedDB !== 'undefined') void collectSolidDraftSnapshots().catch(() => {})

  async function loadLatestDraft() {
    if (loadingLatestDraft.value) return
    getCancelCommand()()
    loadingLatestDraft.value = true
    const document = getDocument()
    const history = getHistory()
    const generation = bumpHistoryGeneration(),
      saveRevision = ++saveGeneration,
      before = document.value,
      target = history
    const current = () =>
      !restoreDisposed &&
      props.open &&
      generation === getHistoryGeneration() &&
      saveRevision === saveGeneration &&
      document.value === before &&
      target === getHistory()
    savePending.value = false
    historyMode.value = 'shared'
    historyPending.value = true
    workspace.value?.focus()
    try {
      await withSolidDraftLock(key, async () => {
        await durableReady
        if (!current()) return
        const head = await readSolidDraftHead(key),
          reference = storageGet(key)
        if (!current()) return
        if (!head && !reference)
          throw Error(
            label(
              'Сохранённая версия отсутствует. Скачайте JSON текущей модели.',
              'No saved version exists. Download the current model as JSON.',
            ),
          )
        const text =
          head?.text ??
          (solidDraftSnapshotId(reference) ? await readSolidDraftSnapshot(reference!) : reference!)
        if (!current()) return
        // The shared draft lock keeps the canonical head stable until the validated
        // replacement is published. Local edits remain the preceding history entry.
        const applied = await target.commitAsync(async () => {
          const next = await historyWorker.run({ kind: 'restoreDocument', text })
          if (!current() || storageGet(key) !== reference)
            throw Error(
              label(
                'Документ изменился во время загрузки. Повторите загрузку.',
                'Document changed while loading. Load it again.',
              ),
            )
          return next
        })
        if (!applied || !current()) return
        persistedRevision = head?.revision ?? null
        persistedReference = stored = reference
        draftConflict.value = false
        error.value = ''
        saveError.value = false
        getSync()()
      })
    } catch (e) {
      if (current()) {
        saveError.value = true
        error.value = e instanceof Error ? e.message : String(e)
      }
    } finally {
      if (generation === getHistoryGeneration()) {
        loadingLatestDraft.value = false
        historyPending.value = false
      }
    }
  }

  function persist() {
    const document = getDocument()
    const generation = ++saveGeneration,
      previous = persistedReference
    savePending.value = true
    // Serialization finishes before the asynchronous write. Only top-level empty
    // collections are omitted; cloning materialized instance geometry is unnecessary.
    const persisted = { ...document.value }
    if (!persisted.curves?.length) delete persisted.curves
    if (!persisted.surfaces?.length) delete persisted.surfaces
    const text = serializeDirectDocument(persisted)
    void withSolidDraftLock(key, async () => {
      await durableReady
      if (generation !== saveGeneration || restoreDisposed) return
      if (storageGet(key) !== previous) {
        draftConflict.value = true
        throw Error('DRAFT_CONFLICT')
      }
      let marker: string | undefined
      try {
        if (text.length > MAX_DRAFT_CHARACTERS) marker = await writeSolidDraftSnapshot(text)
        if (generation !== saveGeneration || restoreDisposed) {
          if (marker) await removeSolidDraftSnapshot(marker)
          return
        }
        const head = await writeSolidDraftHead(
          key,
          persistedRevision,
          text,
          () => generation === saveGeneration && !restoreDisposed,
        )
        persistedRevision = head.revision
        if (generation !== saveGeneration || restoreDisposed) {
          if (marker) await removeSolidDraftSnapshot(marker)
          return
        }
        const reference = marker ?? text
        if (!storageSet(key, reference)) throw Error('DRAFT_REFERENCE_FAILED')
        stored = persistedReference = reference
        saveError.value = false
        draftConflict.value = false
        await removeSolidDraftSnapshot(previous).catch(() => {})
      } catch (e) {
        if (marker && stored !== marker) void removeSolidDraftSnapshot(marker).catch(() => {})
        throw e
      }
    })
      .catch(e => {
        if (generation === saveGeneration && !restoreDisposed) {
          saveError.value = true
          const detail = e instanceof Error ? e.message : String(e)
          if (detail === 'DRAFT_CONFLICT') {
            draftConflict.value = true
            error.value = draftConflictMessage()
          } else if (e instanceof Error && e.name === 'QuotaExceededError')
            error.value = label(
              'Хранилище заполнено. Скачайте JSON, освободите место и повторите сохранение.',
              'Storage is full. Download JSON, free space and retry saving.',
            )
          else if (detail === 'DRAFT_REFERENCE_FAILED')
            error.value = label(
              'Документ записан в IndexedDB, но не удалось обновить указатель и уведомить другие вкладки. Скачайте JSON и повторите сохранение.',
              'Document saved to IndexedDB, but its reference and other tabs could not be updated. Download JSON and retry saving.',
            )
          else if (detail === 'DRAFT_CORRUPT')
            error.value = label(
              'Сохранённая версия повреждена. Скачайте локальный JSON через меню «Файл».',
              'Saved version is corrupt. Download local JSON from File.',
            )
          else error.value = label('Не удалось сохранить черновик: ', 'Could not save draft: ') + detail
        }
      })
      .finally(() => {
        if (generation === saveGeneration) savePending.value = false
      })
  }

  const kernelReady = ref(isGeometryKernelReady())
  const kernelLoadError = ref('')
  const kernelLoading = ref(false)
  const initialRecoveryWanted = restoringDraft.value
  function retryKernelLoad() {
    if (kernelLoading.value || restoreDisposed) return
    restoringDraft.value = initialRecoveryWanted
    void restoreInitialDraft()
  }
  const draftRecoveryWorker = createSolidPreviewWorker()
  let draftRecoveryGeneration = 0
  function stopDraftRecovery() {
    draftRecoveryGeneration++
    kernelLoading.value = false
    draftRecoveryWorker.cancel()
    getHistory().cancelRestore()
    restoringDraft.value = false
  }
  function cancelDraftRecovery() {
    stopDraftRecovery()
    saveError.value = true
    error.value = label(
      'Восстановление отменено. Сохранённый черновик не изменён; перезагрузите страницу, чтобы восстановить его.',
      'Recovery cancelled. The saved draft is unchanged; reload the page to recover it.',
    )
  }
  onUnmounted(() => {
    stopDraftRecovery()
    draftRecoveryWorker.dispose()
  })

  async function restoreInitialDraft() {
    const history = getHistory()
    const generation = ++draftRecoveryGeneration,
      target = history
    const reference = stored,
      expectedReference = persistedReference,
      expectedGeneration = saveGeneration
    const current = () =>
      !restoreDisposed &&
      generation === draftRecoveryGeneration &&
      target === getHistory() &&
      saveGeneration === expectedGeneration &&
      !history.canUndo &&
      storageGet(key) === expectedReference
    kernelLoading.value = true
    kernelLoadError.value = ''
    try {
      try {
        await warmGeometryKernel()
      } catch (e) {
        if (!restoreDisposed && generation === draftRecoveryGeneration) {
          kernelLoadError.value = e instanceof Error ? e.message : String(e)
        }
        return
      }
      if (restoreDisposed) return
      kernelReady.value = true
      if (!restoringDraft.value || !current()) return
      const head = await durableReady
      const text =
        head?.text ??
        (reference
          ? solidDraftSnapshotId(reference)
            ? await readSolidDraftSnapshot(reference)
            : reference
          : null)
      if (!current()) return
      if (text) {
        const applied = await target.resetAsync(async () => {
          const recovered = await draftRecoveryWorker.run({ kind: 'restoreDocument', text })
          if (!current()) throw Error('Draft recovery cancelled.')
          if (!head)
            await withSolidDraftLock(key, async () => {
              if (!current()) throw Error('Draft recovery cancelled.')
              const existing = await readSolidDraftHead(key)
              if (existing && existing.text !== text) {
                draftConflict.value = true
                throw Error(draftConflictMessage())
              }
              const migrated = existing ?? (await writeSolidDraftHead(key, null, text, current))
              if (current()) persistedRevision = migrated.revision
            })
          if (!current()) throw Error('Draft recovery cancelled.')
          return recovered
        })
        if (!applied || !current()) return
        const snapDocument = getSnapDocument()
        const document = getDocument()
        snapDocument.value = history.document
        document.value = snapDocument.value
        error.value = ''
        saveError.value = false
      }
      await nextTick()
      if (current()) {
        const fit = getFit()
        fit('2d')
        fit('3d')
      }
    } catch (e) {
      if (!current()) return
      saveError.value = true
      const detail = e instanceof Error ? e.message : String(e)
      error.value =
        e instanceof SyntaxError || (e instanceof Error && e.name === 'SyntaxError')
          ? label(
              'Сохранённый JSON повреждён. Импортируйте резервный JSON через меню «Файл».',
              'Saved JSON is corrupt. Import a JSON backup from the File menu.',
            )
          : detail === 'Saved draft snapshot is missing or invalid. Import a JSON backup.'
            ? label(
                'Сохранённый снимок отсутствует или повреждён. Импортируйте резервный JSON через меню «Файл».',
                detail,
              )
            : label('Не удалось восстановить сохранённый документ: ', 'Could not restore the saved document: ') +
              detail
    } finally {
      if (generation === draftRecoveryGeneration) {
        restoringDraft.value = false
        kernelLoading.value = false
      }
    }
  }

  return {
    initial,
    saveError,
    savePending,
    draftConflict,
    loadingLatestDraft,
    restoringDraft,
    isRestoreDisposed: () => restoreDisposed,
    getSaveGeneration: () => saveGeneration,
    loadLatestDraft,
    persist,
    kernelReady,
    kernelLoadError,
    kernelLoading,
    retryKernelLoad,
    stopDraftRecovery,
    cancelDraftRecovery,
    restoreInitialDraft,
  }
}
