import { computed, ref, watch, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import {
  MAX_DOCUMENT_CHARACTERS,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectBody,
  type DirectDocument,
  type DirectHistory,
  type Point2,
} from '../../services/directModeling'
import type { SketchPlane } from '../../services/directSketchGeometry'
import { downloadBytes } from '../../services/downloadArtifact'
import { type MeshExportFormat } from '../../services/meshConvert'
import { stripMeshExtension } from '../../services/meshFormats'
import { manifoldPreferenceSnapshot } from '../../services/manifoldPreferences'
import { formatMeshImportNotice } from '../../services/meshImportReport'
import { stringifyMeshJson } from '../../services/meshJson'
import type { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { download, type Pane } from './helpers'
import { useSvgExchange } from './useSvgExchange'

export interface UseFileExchangeOptions {
  props: { open: boolean }
  ru: ComputedRef<boolean>
  label: (ruText: string, enText: string) => string
  document: ShallowRef<DirectDocument>
  history: DirectHistory
  historyWorker: ReturnType<typeof createSolidPreviewWorker>
  historyPending: Ref<boolean>
  historyMode: Ref<'restore' | 'import' | 'export' | 'shared'>
  bumpHistoryGeneration: () => number
  getHistoryGeneration: () => number
  getSaveGeneration: () => number
  isRestoreDisposed: () => boolean
  restoringDraft: Ref<boolean>
  error: Ref<string>
  setNotice: (message: string) => void
  workspace: Ref<HTMLElement | undefined>
  activeGroup: Ref<string>
  selection: Ref<string>
  extraSelection: Ref<string[]>
  mode: Ref<'2d' | '3d'>
  sketchPaneOpen: Ref<boolean>
  operation: Ref<'extrude' | 'revolve' | 'fillet' | 'dogear' | 'array' | null>
  advancedOp: Ref<any>
  subtract: Ref<{ a: string[]; b: string[]; active: 'a' | 'b' } | null>
  boxSelect: Ref<boolean>
  activePlane: Ref<SketchPlane>
  workplaneOutline: Ref<Point2[][]>
  selectedBody: ComputedRef<DirectBody | undefined>
  sourceWallPending: Ref<boolean>
  sourceSeamPending: Ref<boolean>
  sourceStepPending: Ref<boolean>
  cancelSourceWall: () => void
  cancelSourceSeam: () => void
  cancelSourceStep: () => void
  cancelCommand: () => void
  prepareCommit: (
    next: DirectDocument,
    origin?: 'edit' | 'file',
  ) => { validateLocked: ((doc: DirectDocument) => void) | undefined; added: string[] }
  finishCommit: (added: string[], next?: DirectDocument) => void
  commit: (next: DirectDocument) => void
  sync: (next?: DirectDocument) => void
  run: (action: () => void) => void
  fit: (pane: Pane, selectedOnly?: boolean) => void
  pickObject: (id: string, pane: Pane, additive?: boolean) => void
}

export function useFileExchange(options: UseFileExchangeOptions) {
  const {
    props,
    ru,
    label,
    document,
    history,
    historyWorker,
    historyPending,
    historyMode,
    bumpHistoryGeneration,
    getHistoryGeneration,
    isRestoreDisposed,
    restoringDraft,
    error,
    setNotice,
    workspace,
    activeGroup,
    selection,
    extraSelection,
    mode,
    operation,
    advancedOp,
    subtract,
    boxSelect,
    selectedBody,
    sourceWallPending,
    sourceSeamPending,
    sourceStepPending,
    cancelSourceWall,
    cancelSourceSeam,
    cancelSourceStep,
    cancelCommand,
    prepareCommit,
    finishCommit,
    commit,
    sync,
    run,
    fit,
    pickObject,
  } = options

  const stlInput = ref<HTMLInputElement>()
  const meshImportProgress = ref<{ phase: string; completed: number; total: number }>()
  const meshImportStatus = computed(() => {
    const progress = meshImportProgress.value
    if (!progress) return ''
    const phase =
      progress.phase === 'decode'
        ? label('Разбор файла', 'Decoding file')
        : progress.phase === 'finalize'
          ? label('Сборка индексированного меша', 'Building indexed mesh')
          : progress.phase === 'check'
            ? label('Проверка manifold', 'Checking manifold topology')
            : progress.phase === 'repair'
              ? label('Исправление топологии', 'Repairing topology')
              : progress.phase === 'inspect'
                ? label('Анализ меша', 'Inspecting mesh')
                : progress.phase === 'export'
                  ? label('Экспорт файла', 'Exporting file')
                  : label('Повторная проверка', 'Verifying the result')
    return `${phase} · ${progress.completed}/${progress.total}`
  })
  watch(historyPending, pending => {
    if (!pending) meshImportProgress.value = undefined
  }, { flush: 'sync' })
  const { svgInput, svgBusy, importSvg, exportBodySvg } = useSvgExchange(options)

  const stepInput = ref<HTMLInputElement>()
  const stepBusy = ref(false)
  const bodyExportFormat = ref<MeshExportFormat>('stl_binary')

  function closeFileMenu(e: KeyboardEvent) {
    const menu = e.currentTarget as HTMLDetailsElement
    if (!menu.open) return
    e.preventDefault()
    e.stopPropagation()
    if (sourceWallPending.value) cancelSourceWall()
    if (sourceSeamPending.value) cancelSourceSeam()
    if (sourceStepPending.value) cancelSourceStep()
    menu.open = false
    menu.querySelector('summary')?.focus()
  }

  function downloadProject() {
    if (!restoringDraft.value) download(serializeDirectDocument(document.value), 'solid-model.json')
  }

  async function importFile(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0]
    if (!file) return
    cancelCommand()
    error.value = ''
    const generation = bumpHistoryGeneration(),
      target = history,
      before = document.value
    const current = () =>
      generation === getHistoryGeneration() &&
      target === history &&
      !isRestoreDisposed() &&
      props.open &&
      document.value === before
    historyMode.value = 'import'
    historyPending.value = true
    workspace.value?.focus()
    try {
      if (file.size > MAX_DOCUMENT_CHARACTERS) throw Error('Document exceeds 64 MB.')
      const text = await file.text()
      if (!current()) return
      const parsed = JSON.parse(text)
      if (parsed?.language === 'rush/nurbs-1') {
        const next = await historyWorker.run({
          kind: 'rushGraphImport',
          document: history.document,
          text,
          group: activeGroup.value || undefined,
        })
        if (!current()) return
        const { validateLocked, added } = prepareCommit(next)
        const applied = await target.commitAsync(async () => next, validateLocked)
        if (!current() || !applied) return
        historyPending.value = false
        finishCommit(added)
        const addedIds = new Set(added)
        selection.value =
          next.surfaces?.find(item => addedIds.has(item.id))?.id ??
          next.curves?.find(item => addedIds.has(item.id))?.id ??
          ''
      } else {
        // Inspect only the collections needed for metadata. Geometry is validated
        // once in the worker, before the history transaction can be published.
        if (
          !parsed ||
          parsed.version !== 1 ||
          !Array.isArray(parsed.bodies) ||
          !Array.isArray(parsed.sketches) ||
          [parsed.curves, parsed.surfaces].some(items => items != null && !Array.isArray(items)) ||
          [...parsed.bodies, ...parsed.sketches, ...(parsed.curves ?? []), ...(parsed.surfaces ?? [])].some(
            item => !item || typeof item !== 'object',
          )
        )
          throw Error('Invalid direct modeling document.')
        const imported = parsed as DirectDocument
        if (imported.blenderProjectId === undefined) imported.blenderProjectId = crypto.randomUUID()
        const { validateLocked, added } = prepareCommit(imported, 'file')
        const payload = stringifyMeshJson(imported)
        const applied = await target.commitAsync(
          () => historyWorker.run({ kind: 'restoreDocument', text: payload }),
          validateLocked,
        )
        if (!current() || !applied) return
        historyPending.value = false
        finishCommit(added)
        selection.value = ''
        extraSelection.value = []
      }
      fit('2d')
      fit('3d')
      const menu = input.closest?.('details') as HTMLDetailsElement | null
      if (menu) menu.open = false
    } catch (e) {
      const { solidImportErrorMessage } = await import('../../services/solidImportErrorMessage')
      if (generation === getHistoryGeneration()) error.value = solidImportErrorMessage(e, ru.value ? 'ru' : 'en')
    } finally {
      if (generation === getHistoryGeneration()) historyPending.value = false
      input.value = ''
    }
  }

  async function downloadBody() {
    if (restoringDraft.value || historyPending.value) return
    cancelCommand()
    error.value = ''
    const body = selectedBody.value
    if (!body) { error.value = label('Выберите тело.', 'Select a body.'); return }
    const generation = bumpHistoryGeneration(), before = document.value, id = body.id
    const name = body.name || 'body', format = bodyExportFormat.value
    const current = () => generation === getHistoryGeneration() && !isRestoreDisposed()
      && props.open && document.value === before && selectedBody.value?.id === id
    historyMode.value = 'export'
    historyPending.value = true
    workspace.value?.focus()
    try {
      const artifact = await historyWorker.run(
        { kind: 'meshExport', mesh: body.mesh, target: format, options: {} },
        { onProgress: progress => { if (current()) meshImportProgress.value = progress } },
      )
      if (!current()) return
      historyPending.value = false
      downloadBytes(artifact.data, artifact.mimeType, `${name}.${artifact.extension}`)
    } catch (e) {
      if (generation === getHistoryGeneration()) error.value = e instanceof Error ? e.message : String(e)
    } finally {
      if (generation === getHistoryGeneration()) historyPending.value = false
    }
  }

  async function importStl(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0]
    if (!file) return
    cancelCommand()
    error.value = ''
    setNotice('')
    const generation = bumpHistoryGeneration(),
      target = history,
      before = document.value
    const current = () =>
      generation === getHistoryGeneration() &&
      target === history &&
      !isRestoreDisposed() &&
      props.open &&
      document.value === before
    historyMode.value = 'import'
    historyPending.value = true
    workspace.value?.focus()
    try {
      const bytes = new Uint8Array(await file.arrayBuffer())
      if (!current()) return
      const preferences = manifoldPreferenceSnapshot()
      const imported = await historyWorker.run(
        {
          kind: 'meshImport',
          fileName: file.name,
          input: bytes,
          options: { weld: preferences.importWeldEpsilonMm, repair: preferences.importRepair },
        },
        { onProgress: progress => { if (current()) meshImportProgress.value = progress } },
      )
      if (!current()) return
      // Publishing the document must not make its watcher cancel this completed import.
      historyPending.value = false
      const d = history.document
      const id = crypto.randomUUID()
      d.bodies.push({
        id,
        name: stripMeshExtension(file.name) || imported.format.toUpperCase(),
        mesh: { positions: imported.positions, indices: imported.indices },
      })
      commit(d)
      selection.value = id
      mode.value = '3d'
      setNotice(formatMeshImportNotice(imported, ru.value ? 'ru' : 'en'))
    } catch (e) {
      if (generation === getHistoryGeneration()) error.value = e instanceof Error ? e.message : String(e)
    } finally {
      if (generation === getHistoryGeneration()) historyPending.value = false
      input.value = ''
    }
  }

  async function importStep(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0]
    if (stepBusy.value) {
      input.value = ''
      return
    }
    const before = document.value
    stepBusy.value = true
    error.value = ''
    setNotice('')
    try {
      if (!file) return
      if (file.size > 16 * 1024 * 1024) throw Error('STEP file exceeds 16 MiB.')
      const text = await file.text()
      const { prepareSolidStepImport } = await import('../../services/solidStepExchange')
      if (isRestoreDisposed() || !props.open || document.value !== before)
        throw Error('The scene changed during STEP import. Import again to add its bodies.')
      cancelCommand()
      const imported = await prepareSolidStepImport(text, before)
      if (isRestoreDisposed() || !props.open || document.value !== before) {
        throw Error(
          imported.report.retained
            ? 'AP242 original saved, but the scene changed during import. Import again to add its bodies.'
            : 'The scene changed during STEP import. Import again to add its bodies.',
        )
      }
      commit(imported.document)
      operation.value = null
      advancedOp.value = null
      subtract.value = null
      boxSelect.value = false
      pickObject(imported.selected, '3d')
      fit('3d')
      setNotice(
        imported.report.retained
          ? label(
              `AP242: ${imported.report.occurrenceIdentities.length} экземпляров; оригинал сохранён.`,
              `AP242: ${imported.report.occurrenceIdentities.length} occurrence(s); original saved.`,
            )
          : label('STEP: сетка импортирована.', 'STEP: mesh imported.'),
      )
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      stepBusy.value = false
      input.value = ''
    }
  }

  async function exportBlender() {
    if (restoringDraft.value) return
    error.value = ''
    const before = document.value
    try {
      const { exportSolidBlenderSnapshot } = await import('../../services/solidBlenderExchange')
      if (document.value !== before || isRestoreDisposed())
        throw Error(label('Сцена изменилась. Повторите экспорт.', 'The scene changed. Export again.'))
      // Validate support before assigning identity or persisting metadata.
      exportSolidBlenderSnapshot(before, before.blenderProjectId ?? 'validate')
      const id = history.ensureBlenderProjectId()
      sync()
      download(exportSolidBlenderSnapshot(document.value, id), 'solid-model.osv-blender.json')
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  return {
    stlInput,
    meshImportProgress,
    meshImportStatus,
    svgInput,
    svgBusy,
    stepInput,
    stepBusy,
    bodyExportFormat,
    closeFileMenu,
    downloadProject,
    importFile,
    downloadBody,
    importStl,
    importStep,
    exportBlender,
    importSvg,
    exportBodySvg,
  }
}
