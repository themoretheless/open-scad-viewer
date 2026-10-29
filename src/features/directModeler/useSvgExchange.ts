import { onUnmounted, ref, watch } from 'vue'
import { parseDirectDocument, serializeDirectDocument } from '../../services/directModeling'
import { xyPlane } from '../../services/directSketchGeometry'
import { download } from './helpers'
import type { UseFileExchangeOptions } from './useFileExchange'

type SvgExchangeOptions = Pick<UseFileExchangeOptions,
  | 'props'
  | 'label'
  | 'document'
  | 'getSaveGeneration'
  | 'isRestoreDisposed'
  | 'error'
  | 'history'
  | 'commit'
  | 'operation'
  | 'advancedOp'
  | 'subtract'
  | 'activePlane'
  | 'workplaneOutline'
  | 'selection'
  | 'extraSelection'
  | 'mode'
  | 'sketchPaneOpen'
  | 'fit'
  | 'setNotice'
  | 'restoringDraft'
  | 'selectedBody'
>

export function useSvgExchange(options: SvgExchangeOptions) {
  const {
    props,
    label,
    document,
    getSaveGeneration,
    isRestoreDisposed,
    error,
    history,
    commit,
    operation,
    advancedOp,
    subtract,
    activePlane,
    workplaneOutline,
    selection,
    extraSelection,
    mode,
    sketchPaneOpen,
    fit,
    setNotice,
    restoringDraft,
    selectedBody,
  } = options

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

  return { svgInput, svgBusy, importSvg, exportBodySvg }
}
