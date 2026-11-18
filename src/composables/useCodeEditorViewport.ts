import { computed, nextTick, ref, type ComputedRef, type Ref } from 'svet'
import { GUTTER_CONTENT_PADDING_PX, GUTTER_OVERSCAN_ROWS, gutterRowPx } from '../appHelpers'
import { highlightCode } from '../language/codeHighlight'
import { editorBlocks, guideFitsIndent, type EditorBlock } from '../language/editorBlocks'
import type { EditorAdapter } from '../editor/EditorAdapter'

export interface UseCodeEditorViewportOptions {
  code: Ref<string>
  selectedEditorName: Ref<string>
  editor: EditorAdapter
  highlightRef: Ref<HTMLPreElement | null>
}

export interface EditorRowGuide extends EditorBlock {
  last: boolean
}

export interface EditorRowView {
  line: number
  text: string
  block: EditorBlock | undefined
  readonly html: string
  readonly guides: EditorRowGuide[]
}

export interface UseCodeEditorViewportReturn {
  blocks: ComputedRef<EditorBlock[]>
  foldedLines: Ref<Set<number>>
  highlightedCode: ComputedRef<string>
  editorRows: ComputedRef<EditorRowView[]>
  foldedRows: ComputedRef<EditorRowView[]>
  lineNumbersRef: Ref<HTMLDivElement | null>
  guidesRef: Ref<HTMLDivElement | null>
  editorLineCount: ComputedRef<number>
  guideRowPx: Ref<number>
  gutterScrollTop: Ref<number>
  gutterViewportHeight: Ref<number>
  visibleGutterRows: ComputedRef<EditorRowView[]>
  visibleGuideRows: ComputedRef<EditorRowView[]>
  gutterLinesHeight: ComputedRef<string>
  toggleFold: (line: number) => void
  editFoldedLine: (line: number) => Promise<void>
  syncHighlightScroll: () => void
}

export function useCodeEditorViewport(options: UseCodeEditorViewportOptions): UseCodeEditorViewportReturn {
  const { code, selectedEditorName, editor, highlightRef } = options

  const blocks = computed(() => editorBlocks(code.value))
  const foldedLines = ref(new Set<number>())
  const highlightedCode = computed(() => highlightCode(code.value, selectedEditorName.value))

  const editorRows = computed<EditorRowView[]>(() => {
    const lines = code.value.split('\n')
    const currentBlocks = blocks.value
    const byStart = new Map<number, EditorBlock>()
    for (const b of currentBlocks) byStart.set(b.start, b)
    // Highlighted lines and guides are needed only by the folded view and the
    // virtualized guide window — compute them lazily, not for every keystroke.
    let highlightedLines: string[] | null = null
    const highlighted = () => (highlightedLines ??= highlightedCode.value.replace(/\n$/, '').split('\n'))
    const guideEnds = new Map<number, number>()
    const guideEnd = (b: EditorBlock) => {
      let end = guideEnds.get(b.start)
      if (end === undefined) {
        end = b.end
        while (end > b.start && !guideFitsIndent(lines[end] ?? '', b.column)) end--
        guideEnds.set(b.start, end)
      }
      return end
    }
    return lines.map((text, line) => {
      let html: string | undefined
      let guides: EditorRowGuide[] | undefined
      return {
        line,
        text,
        block: byStart.get(line),
        get html() {
          return (html ??= highlighted()[line] ?? '')
        },
        get guides() {
          if (!guides) {
            guides = []
            for (const b of currentBlocks) {
              if (b.start >= line) break
              if (b.end >= line && guideFitsIndent(text, b.column)) {
                guides.push({ ...b, last: line === guideEnd(b) })
              }
            }
          }
          return guides
        },
      }
    })
  })

  const foldedRows = computed(() => {
    let hiddenThrough = -1
    return editorRows.value.filter(row => {
      if (row.line <= hiddenThrough) return false
      if (row.block && foldedLines.value.has(row.line)) hiddenThrough = row.block.end
      return true
    })
  })

  function toggleFold(line: number) {
    const next = new Set(foldedLines.value)
    if (next.has(line)) next.delete(line)
    else next.add(line)
    foldedLines.value = next
  }

  const lineNumbersRef = ref<HTMLDivElement | null>(null)
  const guidesRef = ref<HTMLDivElement | null>(null)
  const editorLineCount = computed(() => editorRows.value.length)
  const guideRowPx = ref(1.58 * 0.82 * 16)
  const gutterScrollTop = ref(0)
  const gutterViewportHeight = ref(600)

  function visibleRowSlice(rowPx: number) {
    const rows = editorRows.value
    const first = Math.max(
      0,
      Math.floor((gutterScrollTop.value - GUTTER_CONTENT_PADDING_PX) / rowPx) - GUTTER_OVERSCAN_ROWS,
    )
    const last = Math.min(
      rows.length,
      Math.ceil((gutterScrollTop.value + gutterViewportHeight.value - GUTTER_CONTENT_PADDING_PX) / rowPx) +
        GUTTER_OVERSCAN_ROWS,
    )
    return rows.slice(first, last)
  }

  const visibleGutterRows = computed(() => visibleRowSlice(gutterRowPx))
  const visibleGuideRows = computed(() => visibleRowSlice(guideRowPx.value))
  const gutterLinesHeight = computed(() => `${editorRows.value.length * 1.58}em`)

  function syncHighlightScroll() {
    if (!editor.mounted || !highlightRef.value) return
    const scroll = editor.getScroll()
    const layer = highlightRef.value.firstElementChild as HTMLElement | null
    if (layer) {
      layer.style.transform = `translate(${-scroll.left}px, ${-scroll.top}px)`
    }
    if (guidesRef.value) {
      guidesRef.value.style.transform = `translate(${-scroll.left}px, ${-scroll.top}px)`
    }
    if (lineNumbersRef.value) {
      lineNumbersRef.value.style.transform = `translateY(${-scroll.top}px)`
    }
    gutterScrollTop.value = scroll.top
    gutterViewportHeight.value = scroll.clientHeight
  }

  async function editFoldedLine(line: number) {
    foldedLines.value = new Set()
    await nextTick()
    if (!editor.mounted) return
    const offset = code.value
      .split('\n')
      .slice(0, line)
      .reduce((n, s) => n + s.length + 1, 0)
    editor.focus()
    editor.setSelection(offset, offset)
    editor.setScroll({ top: Math.max(0, line * editor.lineHeightPx() - 40) })
    syncHighlightScroll()
  }

  return {
    blocks,
    foldedLines,
    highlightedCode,
    editorRows,
    foldedRows,
    lineNumbersRef,
    guidesRef,
    editorLineCount,
    guideRowPx,
    gutterScrollTop,
    gutterViewportHeight,
    visibleGutterRows,
    visibleGuideRows,
    gutterLinesHeight,
    toggleFold,
    editFoldedLine,
    syncHighlightScroll,
  }
}
