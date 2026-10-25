import type { ComputedRef, Ref } from 'vue'
import type { MeshData } from '../core/mesh'
import type { SourceProvenanceRow } from '../components/cadPanels.types'
import { clamp } from '../services/math3d'
import type { WebGPURenderer } from '../services/webgpuRenderer'

export interface ProvenanceSpan {
  start: number
  end: number
  id: number
}

export interface UseSourceProvenanceCaretOptions {
  code: Ref<string>
  editorRef: Ref<HTMLTextAreaElement | null>
  selectedEditorName: Ref<string>
  sceneMeshes: ComputedRef<MeshData[]>
  sourceMatchesEditor: ComputedRef<boolean>
  getRenderer: () => WebGPURenderer | null
  showNotice: (message: string) => void
  translate: (key: string) => string
}

export function useSourceProvenanceCaret(options: UseSourceProvenanceCaretOptions) {
  const {
    code,
    editorRef,
    selectedEditorName,
    sceneMeshes,
    sourceMatchesEditor,
    getRenderer,
    showNotice,
    translate,
  } = options

  // Flat provenance span index for caret lookups: built lazily on the first query after a
  // scene publication and invalidated by the sceneMeshes array identity, replacing the
  // per-keystroke scan over every mesh × provenance run.
  let provenanceSpanCache: { meshes: readonly MeshData[]; spans: ProvenanceSpan[]; maxEnd: number[] } | null = null

  function provenanceSpanIndex(meshes: readonly MeshData[]) {
    if (provenanceSpanCache?.meshes === meshes) return provenanceSpanCache
    const entries: (ProvenanceSpan & { order: number })[] = []
    let order = 0
    for (const mesh of meshes) {
      for (const run of mesh.provenance) {
        const source = run.source
        if (source) entries.push({ start: source.start, end: source.end, id: source.id, order: order++ })
      }
    }
    // Sort by start ascending; within equal starts, later entries first, so the downward
    // scan meets the earliest original span first and keeps it on full ties — identical
    // to the previous linear meshes × runs iteration order.
    entries.sort((a, b) => a.start - b.start || b.order - a.order)
    const maxEnd = new Array<number>(entries.length)
    let max = -Infinity
    for (let i = 0; i < entries.length; i++) {
      if (entries[i].end > max) max = entries[i].end
      maxEnd[i] = max
    }
    provenanceSpanCache = { meshes, spans: entries, maxEnd }
    return provenanceSpanCache
  }

  function sourceIdAtEditorCaret(): number | null {
    const editor = editorRef.value
    if (!editor || !sourceMatchesEditor.value) return null
    const caret =
      editor.selectionStart === editor.selectionEnd
        ? editor.selectionStart
        : editor.selectionDirection === 'backward'
          ? editor.selectionStart
          : Math.max(editor.selectionStart, editor.selectionEnd - 1)
    const { spans, maxEnd } = provenanceSpanIndex(sceneMeshes.value)
    // Rightmost span with start <= caret; only that prefix can contain the caret.
    let lo = 0,
      hi = spans.length
    while (lo < hi) {
      const mid = (lo + hi) >>> 1
      if (spans[mid].start <= caret) lo = mid + 1
      else hi = mid
    }
    let sourceId: number | null = null
    let smallestSpan = Infinity
    let deepestStart = -1
    for (let j = lo - 1; j >= 0; j--) {
      // No span at or before j reaches the caret, or none can beat the current best:
      // a containing span starting at s is longer than caret - s, which only grows leftward.
      if (maxEnd[j] <= caret || caret - spans[j].start >= smallestSpan) break
      const span = spans[j]
      if (span.end <= caret) continue
      const length = span.end - span.start
      if (length < smallestSpan || (length === smallestSpan && span.start > deepestStart)) {
        sourceId = span.id
        smallestSpan = length
        deepestStart = span.start
      }
    }
    return sourceId
  }

  function syncSourceHighlightFromEditor() {
    const input = editorRef.value
    if (input) {
      const start = input.selectionStart,
        end = input.selectionEnd
      const selected = input.value.slice(start, end)
      selectedEditorName.value =
        /^\$?[A-Za-z_][\w$]*$/.test(selected) &&
        !/[\w$]/.test(input.value[start - 1] ?? '') &&
        !/[\w$]/.test(input.value[end] ?? '')
          ? selected
          : ''
    }
    getRenderer()?.setSourceHighlight(sourceIdAtEditorCaret())
  }

  function highlightSource(sourceId: number | null) {
    if (sourceId === null) syncSourceHighlightFromEditor()
    else getRenderer()?.setSourceHighlight(sourceMatchesEditor.value ? sourceId : null)
  }

  function revealSource(source: SourceProvenanceRow) {
    if (!sourceMatchesEditor.value) {
      showNotice(translate('sourceStale'))
      return
    }
    const editor = editorRef.value
    if (!editor) return
    const start = clamp(source.sourceStart, 0, code.value.length)
    const end = clamp(source.sourceEnd, start, code.value.length)
    editor.focus({ preventScroll: false })
    editor.setSelectionRange(start, end, 'forward')
    getRenderer()?.setSourceHighlight(source.sourceId)
  }

  return {
    provenanceSpanIndex,
    sourceIdAtEditorCaret,
    syncSourceHighlightFromEditor,
    highlightSource,
    revealSource,
  }
}
