import { computed, nextTick, ref, type ComputedRef, type Ref } from 'vue'
import { replaceSharedRecoveryHash, STORAGE_NOTICE_THROTTLE_MS } from '../appHelpers'
import type { Language } from '../i18n/appStrings'
import {
  updateWorkspaceDocument,
  workspaceDocumentsEqual,
  type WorkspaceDocumentSnapshot,
} from '../services/workspaceDocument'
import type { BrowserWorkspacePersistence, WorkspaceRetryResult } from '../services/workspacePersistence'
import type { WebGPURenderer } from '../services/webgpuRenderer'

export interface UseWorkspacePersistenceLifecycleOptions {
  initialWorkspaceDurable: boolean
  initialSharedImportPending: boolean
  workspacePersistence: BrowserWorkspacePersistence
  workspaceDocument: Ref<WorkspaceDocumentSnapshot>
  code: Ref<string>
  fileName: Ref<string>
  lang: Ref<Language>
  exampleGalleryOpen: Ref<boolean>
  getRenderer: () => WebGPURenderer | null
  isComponentActive: () => boolean
  markFitNextRender: () => void
  showNotice: (message: string) => void
  translate: (key: string) => string
}

export interface UseWorkspacePersistenceLifecycleReturn {
  workspacePersistenceStatus: Ref<'saved' | 'saving' | 'error'>
  workspaceConflict: Ref<boolean>
  savingBrowser: Ref<boolean>
  storageFailureDetail: Ref<string>
  persistenceLabel: ComputedRef<string>
  bumpWorkspaceEditGeneration: () => void
  isRestoringWorkspace: () => boolean
  isSharedImportPending: () => boolean
  applySourceChange: (value: string) => boolean
  applyFileNameChange: (value: string) => boolean
  markDocumentReplaced: () => void
  scheduleWorkspacePersistence: () => void
  persistWorkspaceNow: (retryIndexedDb?: boolean) => Promise<boolean>
  flushWorkspacePersistence: () => void
  handleVisibilityChange: () => void
  retryWorkspacePersistence: () => void
  saveBrowserDraft: () => Promise<void>
  resolveWorkspaceConflict: (preferCurrentDraft: boolean) => Promise<void>
  reportStorageFailure: () => void
  updateBeforeUnloadGuard: () => void
  detachBeforeUnloadGuard: () => void
  restoreWorkspaceDocument: (document: WorkspaceDocumentSnapshot) => Promise<void>
  disposeWorkspacePersistenceTimer: () => void
}

export function useWorkspacePersistenceLifecycle(
  options: UseWorkspacePersistenceLifecycleOptions,
): UseWorkspacePersistenceLifecycleReturn {
  const {
    initialWorkspaceDurable,
    initialSharedImportPending,
    workspacePersistence,
    workspaceDocument,
    code,
    fileName,
    lang,
    exampleGalleryOpen,
    getRenderer,
    isComponentActive,
    markFitNextRender,
    showNotice,
    translate,
  } = options

  const workspacePersistenceStatus = ref<'saved' | 'saving' | 'error'>(
    initialWorkspaceDurable ? 'saved' : 'error',
  )
  const workspaceConflict = ref(workspacePersistence.hasConflict)
  const savingBrowser = ref(false)
  const storageFailureDetail = ref('')

  const persistenceLabel = computed(() =>
    workspaceConflict.value
      ? translate('storageConflict')
      : workspacePersistenceStatus.value === 'saving'
        ? translate('savingDraft')
        : workspacePersistenceStatus.value === 'error'
          ? translate('unsavedDraft')
          : translate('savedDraftStatus'),
  )

  let lastStorageNotice = 0
  let storageDebounce: ReturnType<typeof setTimeout> | null = null
  let workspaceEditGeneration = 0
  let durableWorkspaceGeneration = initialWorkspaceDurable ? 0 : -1
  let workspaceSourceValid = true
  let workspaceFileNameValid = true
  let beforeUnloadAttached = false
  let sharedImportPending = initialSharedImportPending
  let restoringWorkspace = false

  function bumpWorkspaceEditGeneration() {
    workspaceEditGeneration++
  }

  function isRestoringWorkspace() {
    return restoringWorkspace
  }

  function isSharedImportPending() {
    return sharedImportPending
  }

  function applySourceChange(value: string): boolean {
    try {
      workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { source: value })
      workspaceSourceValid = true
      workspaceEditGeneration++
      scheduleWorkspacePersistence()
      return true
    } catch {
      workspaceSourceValid = false
      workspaceEditGeneration++
      workspacePersistenceStatus.value = 'error'
      updateBeforeUnloadGuard()
      reportStorageFailure()
      return false
    }
  }

  function applyFileNameChange(value: string): boolean {
    try {
      workspaceDocument.value = updateWorkspaceDocument(workspaceDocument.value, { fileName: value })
      workspaceFileNameValid = true
      workspaceEditGeneration++
      scheduleWorkspacePersistence()
      return true
    } catch {
      workspaceFileNameValid = false
      workspaceEditGeneration++
      workspacePersistenceStatus.value = 'error'
      updateBeforeUnloadGuard()
      reportStorageFailure()
      return false
    }
  }

  function markDocumentReplaced() {
    workspaceSourceValid = true
    workspaceFileNameValid = true
    workspaceEditGeneration++
    scheduleWorkspacePersistence()
  }

  function scheduleWorkspacePersistence() {
    if (storageDebounce) clearTimeout(storageDebounce)
    workspacePersistenceStatus.value = 'saving'
    updateBeforeUnloadGuard()
    storageDebounce = setTimeout(() => {
      void persistWorkspaceNow()
    }, 300)
  }

  async function persistWorkspaceNow(retryIndexedDb = false): Promise<boolean> {
    storageDebounce = null
    if (!workspaceSourceValid || !workspaceFileNameValid) {
      workspacePersistenceStatus.value = 'error'
      updateBeforeUnloadGuard()
      return false
    }
    const generation = workspaceEditGeneration
    const snapshot = workspaceDocument.value
    workspacePersistenceStatus.value = 'saving'
    updateBeforeUnloadGuard()
    let saved = false
    try {
      if (retryIndexedDb) {
        const result = await workspacePersistence.retry(snapshot)
        saved = result.saved
        if (result.restoredDocument) {
          const editorUnchanged =
            generation === workspaceEditGeneration &&
            workspaceDocumentsEqual(workspaceDocument.value, snapshot) &&
            code.value === snapshot.source &&
            fileName.value === snapshot.fileName
          if (editorUnchanged) await restoreWorkspaceDocument(result.restoredDocument)
          else saved = false
        }
      } else {
        saved = await workspacePersistence.save(snapshot)
      }
    } catch {
      saved = false
    }
    workspaceConflict.value = workspacePersistence.hasConflict
    if (!isComponentActive()) return saved
    if (saved) {
      durableWorkspaceGeneration = Math.max(durableWorkspaceGeneration, generation)
      if (sharedImportPending && location.hash.startsWith('#code=')) {
        if (workspacePersistence.backend === 'indexeddb') {
          history.replaceState(null, '', location.pathname + location.search)
          sharedImportPending = false
        } else {
          replaceSharedRecoveryHash(code.value)
        }
      }
      workspacePersistenceStatus.value =
        durableWorkspaceGeneration >= workspaceEditGeneration ? 'saved' : 'saving'
    } else {
      workspacePersistenceStatus.value = 'error'
      reportStorageFailure()
    }
    updateBeforeUnloadGuard()
    return saved
  }

  function flushWorkspacePersistence() {
    if (storageDebounce) clearTimeout(storageDebounce)
    if (!workspaceSourceValid || !workspaceFileNameValid) return
    if (sharedImportPending && location.hash.startsWith('#code=')) replaceSharedRecoveryHash(code.value)
    // localStorage is synchronous and therefore remains reliable during
    // pagehide/freeze where the browser may terminate an unfinished IDB task.
    if (!workspacePersistence.stageRecovery(workspaceDocument.value)) reportStorageFailure()
    void persistWorkspaceNow()
  }

  function handleVisibilityChange() {
    if (document.visibilityState === 'hidden' && storageDebounce) flushWorkspacePersistence()
  }

  function retryWorkspacePersistence() {
    if (storageDebounce) {
      clearTimeout(storageDebounce)
      storageDebounce = null
    }
    void persistWorkspaceNow(true)
  }

  async function saveBrowserDraft() {
    if (savingBrowser.value) return
    savingBrowser.value = true
    if (storageDebounce) {
      clearTimeout(storageDebounce)
      storageDebounce = null
    }
    try {
      if (!workspaceSourceValid || !workspaceFileNameValid) {
        showNotice(translate('failedBrowser'))
        return
      }
      await resolveWorkspaceConflict(true)
      showNotice(
        translate(
          workspacePersistenceStatus.value === 'saved' && workspacePersistence.backend === 'indexeddb'
            ? 'savedBrowser'
            : 'failedBrowser',
        ),
      )
    } finally {
      savingBrowser.value = false
    }
  }

  async function resolveWorkspaceConflict(preferCurrentDraft: boolean) {
    const generation = workspaceEditGeneration
    const snapshot = workspaceDocument.value
    workspacePersistenceStatus.value = 'saving'
    let result: WorkspaceRetryResult
    try {
      result = preferCurrentDraft
        ? await workspacePersistence.saveCurrentDraft(snapshot)
        : await workspacePersistence.resolveConflict(false, snapshot)
    } catch {
      result = { saved: false }
    }
    workspaceConflict.value = workspacePersistence.hasConflict
    const editorUnchanged =
      generation === workspaceEditGeneration &&
      workspaceDocumentsEqual(workspaceDocument.value, snapshot) &&
      code.value === snapshot.source &&
      fileName.value === snapshot.fileName
    if (result.restoredDocument && editorUnchanged) {
      if (!workspaceDocumentsEqual(result.restoredDocument, snapshot)) {
        await restoreWorkspaceDocument(result.restoredDocument)
      }
    } else if (!editorUnchanged) {
      scheduleWorkspacePersistence()
      return
    }
    if (result.saved) {
      durableWorkspaceGeneration = workspaceEditGeneration
      workspacePersistenceStatus.value = 'saved'
      if (sharedImportPending && workspacePersistence.backend === 'indexeddb') {
        history.replaceState(null, '', location.pathname + location.search)
        sharedImportPending = false
      }
    } else {
      workspacePersistenceStatus.value = 'error'
      reportStorageFailure()
    }
    updateBeforeUnloadGuard()
  }

  function reportStorageFailure() {
    storageFailureDetail.value =
      workspacePersistence.failureReason ||
      (lang.value === 'ru'
        ? 'Не удалось записать черновик. Нажмите «Сохранить в браузере» для повторной записи текущего кода.'
        : 'Could not write draft. Use Save in browser to retry the current code.')
    const now = Date.now()
    if (now - lastStorageNotice < STORAGE_NOTICE_THROTTLE_MS) return
    lastStorageNotice = now
    showNotice(translate('storageFailed'))
  }

  function handleBeforeUnload(event: BeforeUnloadEvent) {
    if (
      durableWorkspaceGeneration >= workspaceEditGeneration &&
      workspacePersistenceStatus.value !== 'error'
    )
      return
    event.preventDefault()
    event.returnValue = ''
  }

  function updateBeforeUnloadGuard() {
    const needed =
      durableWorkspaceGeneration < workspaceEditGeneration ||
      workspacePersistenceStatus.value === 'error'
    if (needed && !beforeUnloadAttached) {
      window.addEventListener('beforeunload', handleBeforeUnload)
      beforeUnloadAttached = true
    } else if (!needed && beforeUnloadAttached) {
      detachBeforeUnloadGuard()
    }
  }

  function detachBeforeUnloadGuard() {
    if (!beforeUnloadAttached) return
    window.removeEventListener('beforeunload', handleBeforeUnload)
    beforeUnloadAttached = false
  }

  async function restoreWorkspaceDocument(documentSnapshot: WorkspaceDocumentSnapshot) {
    restoringWorkspace = true
    workspaceSourceValid = true
    workspaceFileNameValid = true
    workspaceDocument.value = documentSnapshot
    code.value = documentSnapshot.source
    fileName.value = documentSnapshot.fileName
    exampleGalleryOpen.value = false
    getRenderer()?.setSourceHighlight(null)
    await nextTick()
    restoringWorkspace = false
    workspacePersistence.stageRecovery(documentSnapshot)
    workspaceEditGeneration++
    durableWorkspaceGeneration = workspaceEditGeneration
    markFitNextRender()
  }

  function disposeWorkspacePersistenceTimer() {
    if (storageDebounce) {
      clearTimeout(storageDebounce)
      void persistWorkspaceNow()
    }
  }

  return {
    workspacePersistenceStatus,
    workspaceConflict,
    savingBrowser,
    storageFailureDetail,
    persistenceLabel,
    bumpWorkspaceEditGeneration,
    isRestoringWorkspace,
    isSharedImportPending,
    applySourceChange,
    applyFileNameChange,
    markDocumentReplaced,
    scheduleWorkspacePersistence,
    persistWorkspaceNow,
    flushWorkspacePersistence,
    handleVisibilityChange,
    retryWorkspacePersistence,
    saveBrowserDraft,
    resolveWorkspaceConflict,
    reportStorageFailure,
    updateBeforeUnloadGuard,
    detachBeforeUnloadGuard,
    restoreWorkspaceDocument,
    disposeWorkspacePersistenceTimer,
  }
}
