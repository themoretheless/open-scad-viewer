import { nextTick, onUnmounted, ref, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import { useMainSolidTask } from '../features/useMainSolidTask'
import { createSolidPreviewWorker } from '../services/solidPreviewWorker'
import { downloadBlob, sanitizeFileName } from '../appHelpers'
import type { MeshData } from '../core/mesh'
import type { BuildPerformanceSample } from '../services/buildPerformance'
import { detectMeshImportFormat, type MeshExportFormat } from '../services/meshFormats'
import { SOURCE_FILE_EXTENSION, withSourceExtension } from '../services/rushFrontendDetect'
import { MAX_WORKSPACE_FILE_NAME_LENGTH, MAX_WORKSPACE_SOURCE_LENGTH } from '../services/workspaceDocument'
import { encodeWorkspaceShare } from '../services/workspaceShare'

export interface UseWorkspaceFileActionsOptions {
  code: Ref<string>
  fileName: Ref<string>
  error: Ref<string>
  canExport: ComputedRef<boolean>
  sceneMeshes: ComputedRef<MeshData[]>
  buildCounters: ShallowRef<{
    builds: number
    superseded: number
    workerStarts: number
    hardRestarts: number
  }>
  performanceSamples: ShallowRef<BuildPerformanceSample[]>
  markFitNextRender: () => void
  showNotice: (message: string) => void
  translate: (key: string) => string
}

export function useWorkspaceFileActions(options: UseWorkspaceFileActionsOptions) {
  const {
    code,
    fileName,
    error,
    canExport,
    sceneMeshes,
    buildCounters,
    performanceSamples,
    markFitNextRender,
    showNotice,
    translate,
  } = options

  const fileInputRef = ref<HTMLInputElement | null>(null)
  const convertInputRef = ref<HTMLInputElement | null>(null)
  const exportDialogOpen = ref(false)
  const exportDialogRef = ref<HTMLElement | null>(null)
  const additionalExportFormat = ref<MeshExportFormat>('stl_binary')
  const conversionTask = useMainSolidTask(createSolidPreviewWorker())
  const conversionOperation = ref<'convert' | 'export'>('convert')
  const { pending: conversionPending, progress: conversionProgress } = conversionTask
  let disposed = false
  onUnmounted(() => {
    disposed = true
    conversionTask.dispose()
  })

  function openExportDialog() {
    exportDialogOpen.value = true
    void nextTick(() => {
      const dialog = exportDialogRef.value
      ;(dialog?.querySelector<HTMLElement>('input:checked') ?? dialog?.querySelector<HTMLElement>('button'))?.focus()
    })
  }

  async function confirmExport() {
    exportDialogOpen.value = false
    await exportAdditionalMesh()
  }

  function triggerOpen() {
    fileInputRef.value?.click()
  }

  async function openSelectedFile(event: Event) {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (file) await openFile(file)
  }

  async function handleDrop(event: DragEvent) {
    const file = event.dataTransfer?.files?.[0]
    if (!file) return
    if (detectMeshImportFormat(file.name)) await convertFile(file)
    else await openFile(file)
  }

  function triggerConvert() {
    if (!conversionPending.value) convertInputRef.value?.click()
  }

  async function convertSelectedFile(event: Event) {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (file) await convertFile(file)
  }

  /** The whole conversion runs off the UI thread; only the current task may download. */
  async function convertFile(file: File) {
    if (disposed || conversionPending.value) return
    const target = additionalExportFormat.value
    try {
      const input = new Uint8Array(await file.arrayBuffer())
      if (disposed) return
      conversionOperation.value = 'convert'
      await conversionTask.run(
        { kind: 'meshConvert', fileName: file.name, input, target, options: {} },
        result => {
          downloadBlob(new Blob([result.data.slice().buffer as ArrayBuffer], { type: result.mimeType }), result.fileName)
          showNotice(`${translate('converted')}: ${result.source.format.toUpperCase()} → ${result.format.toUpperCase()}`)
        },
        caught => showNotice(caught instanceof Error ? caught.message : 'Conversion failed'),
      )
    } catch (caught) {
      if (!disposed) showNotice(caught instanceof Error ? caught.message : 'Conversion failed')
    }
  }

  function cancelConversion() {
    conversionTask.cancel()
  }

  async function openFile(file: File) {
    if (file.size > MAX_WORKSPACE_SOURCE_LENGTH) {
      error.value = translate('fileTooLarge')
      return
    }
    const source = await file.text()
    if (source.length > MAX_WORKSPACE_SOURCE_LENGTH) {
      error.value = translate('fileTooLarge')
      return
    }
    code.value = source
    const requestedName = withSourceExtension(file.name, source)
    fileName.value = requestedName.slice(0, MAX_WORKSPACE_FILE_NAME_LENGTH)
    markFitNextRender()
    showNotice(translate('opened'))
  }

  function saveSource() {
    const blob = new Blob([code.value], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = sanitizeFileName(fileName.value)
    link.click()
    URL.revokeObjectURL(url)
    showNotice(translate('saved'))
  }

  async function exportScene(target: MeshExportFormat, sourceExtension: RegExp) {
    if (disposed || conversionPending.value || !canExport.value) return
    const mesh = sceneMeshes.value, sourceName = fileName.value
    conversionOperation.value = 'export'
    await conversionTask.run(
      { kind: 'meshExport', mesh, target, options: {} },
      artifact => {
        downloadBlob(
          new Blob([artifact.data.slice().buffer as ArrayBuffer], { type: artifact.mimeType }),
          sanitizeFileName(sourceName).replace(sourceExtension, '.' + artifact.extension),
        )
        showNotice(translate('exported'))
      },
      caught => showNotice(caught instanceof Error ? caught.message : 'Export failed'),
    )
  }

  function exportAdditionalMesh() {
    return exportScene(additionalExportFormat.value, /\.scad$/i)
  }

  function exportStl() {
    return exportScene('stl_binary', SOURCE_FILE_EXTENSION)
  }

  function exportPerformance() {
    downloadBlob(
      new Blob(
        [
          JSON.stringify(
            {
              schemaVersion: 1,
              recordedAt: new Date().toISOString(),
              frameBoundary: 'GPU queue submission, not display presentation or GPU completion',
              counters: buildCounters.value,
              samples: performanceSamples.value,
            },
            null,
            2,
          ),
        ],
        { type: 'application/json' },
      ),
      'build-performance.json',
    )
  }

  function exportObj() {
    return exportScene('obj', /\.scad$/i)
  }

  async function shareSource() {
    const url = new URL(window.location.href)
    url.hash = `code=${encodeWorkspaceShare(code.value)}`
    history.replaceState(null, '', url)
    try {
      await navigator.clipboard.writeText(url.toString())
      showNotice(translate('copied'))
    } catch {
      showNotice(translate('copyFailed'))
    }
  }

  return {
    fileInputRef,
    convertInputRef,
    exportDialogOpen,
    exportDialogRef,
    additionalExportFormat,
    conversionPending,
    conversionProgress,
    conversionOperation,
    cancelConversion,
    openExportDialog,
    confirmExport,
    triggerOpen,
    openSelectedFile,
    handleDrop,
    triggerConvert,
    convertSelectedFile,
    convertFile,
    openFile,
    saveSource,
    exportAdditionalMesh,
    exportStl,
    exportPerformance,
    exportObj,
    shareSource,
  }
}
