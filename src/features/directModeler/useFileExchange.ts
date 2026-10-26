import { computed, onUnmounted, ref, watch, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import {
  MAX_DOCUMENT_CHARACTERS,
  parseDirectDocument,
  serializeDirectDocument,
  type DirectBody,
  type DirectDocument,
  type DirectHistory,
  type Point2,
} from '../../services/directModeling'
import { xyPlane, type SketchPlane } from '../../services/directSketchGeometry'
import { downloadBytes } from '../../services/downloadArtifact'
import { type MeshExportFormat } from '../../services/meshConvert'
import { stripMeshExtension } from '../../services/meshFormats'
import { manifoldPreferenceSnapshot } from '../../services/manifoldPreferences'
import { formatMeshImportNotice } from '../../services/meshImportReport'
import { stringifyMeshJson } from '../../services/meshJson'
import type { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { download, type Pane } from './helpers'

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
    getSaveGeneration,
    isRestoreDisposed,
    restoringDraft,
    error,
    setNotice,
    workspace,
    activeGroup,
    selection,
    extraSelection,
    mode,
    sketchPaneOpen,
    operation,
    advancedOp,
    subtract,
    boxSelect,
    activePlane,
    workplaneOutline,
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
  const svgInput = ref<HTMLInputElement>()
  const svgBusy = ref(false)
  let svgController: AbortController | undefined
  function cancelSvg() {
    svgController?.abort()
    svgController = undefined
    svgBusy.value = false
  }
  watch(
    () => props.open,
    open => {
      if (!open) cancelSvg()
    },
    { flush: 'sync' },
  )
  onUnmounted(cancelSvg)

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

  async function importSvg(event: Event) {
    const input = event.target as HTMLInputElement,
      file = input.files?.[0]
    if (!file || svgBusy.value) {
      input.value = ''
      return
    }
    cancelSvg()
    const controller = new AbortController()
    svgController = controller
    const before = document.value,
      generation = getSaveGeneration()
    svgBusy.value = true
    error.value = ''
    let worker: import('../../services/svgWorkerClient').SvgWorkerClient | undefined
    try {
      if (file.size > 4 * 1024 * 1024) throw Error('SVG exceeds 4 MiB.')
      const { createSvgWorkerClient } = await import('../../services/svgWorkerClient')
      const text = await file.text()
      if (controller.signal.aborted) return
      worker = createSvgWorkerClient()
      const result = await worker.run({ kind: 'contours', svg: text, options: {} }, { signal: controller.signal })
      if (controller.signal.aborted) return
      if (isRestoreDisposed() || !props.open || document.value !== before || getSaveGeneration() !== generation)
        throw Error(label('Сцена изменилась. Повторите импорт SVG.', 'The scene changed. Import SVG again.'))
      if (!result.contours?.length) throw Error('SVG contains no closed contours.')
      if (result.contours.some(r => r.length > 512))
        throw Error(
          label(
            'Контур SVG превышает 512 вершин. Упростите исходный профиль.',
            'SVG contour exceeds 512 vertices. Simplify the source profile.',
          ),
        )
      const next = history.document
      const ids = result.contours.map(points => {
        const id = crypto.randomUUID()
        next.sketches.push({
          id,
          name: `${file.name.slice(0, 70)} · ${next.sketches.length + 1}`,
          closed: true,
          points,
        })
        return id
      })
      commit(parseDirectDocument(serializeDirectDocument(next)))
      operation.value = null
      advancedOp.value = null
      subtract.value = null
      activePlane.value = xyPlane()
      workplaneOutline.value = []
      selection.value = ids[0]
      extraSelection.value = ids.slice(1)
      mode.value = '2d'
      sketchPaneOpen.value = true
      fit('2d')
      setNotice(
        label(
          'SVG импортирован. Для отверстий выдавливайте выбранные контуры вместе.',
          'SVG imported. Extrude the selected contours together to retain holes.',
        ) + (result.warnings.length ? ' ' + result.warnings.join(' ') : ''),
      )
    } catch (e) {
      if (!controller.signal.aborted) error.value = e instanceof Error ? e.message : String(e)
    } finally {
      worker?.dispose()
      if (svgController === controller) {
        svgController = undefined
        svgBusy.value = false
      }
      input.value = ''
    }
  }

  async function exportBodySvg() {
    if (restoringDraft.value) return
    const body = selectedBody.value
    if (!body) return
    cancelSvg()
    const controller = new AbortController()
    svgController = controller
    const mesh = structuredClone(body.mesh)
    error.value = ''
    let worker: import('../../services/svgWorkerClient').SvgWorkerClient | undefined
    try {
      const { createSvgWorkerClient } = await import('../../services/svgWorkerClient')
      if (controller.signal.aborted) return
      worker = createSvgWorkerClient()
      const vertices = new Float32Array(mesh.positions.length * 2)
      for (let i = 0; i < mesh.positions.length; i += 3) vertices.set(mesh.positions.slice(i, i + 3), i * 2)
      const result = await worker.run(
        {
          kind: 'project',
          axis: 'z',
          options: {},
          meshes: [
            {
              vertices,
              indices: new Uint32Array(mesh.indices),
              faceIds: new Uint32Array(mesh.indices.length / 3),
              transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
            },
          ],
        },
        { signal: controller.signal },
      )
      if (!controller.signal.aborted && !isRestoreDisposed() && props.open) download(result.svg, 'body-xy.svg')
    } catch (e) {
      if (!controller.signal.aborted) error.value = e instanceof Error ? e.message : String(e)
    } finally {
      worker?.dispose()
      if (svgController === controller) svgController = undefined
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
