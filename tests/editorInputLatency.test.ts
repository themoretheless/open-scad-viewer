import { computed, ref } from 'svet'
import { describe, expect, it } from 'vitest'
import { useCodeEditorViewport } from '../src/composables/useCodeEditorViewport'
import { useSourceProvenanceCaret } from '../src/composables/useSourceProvenanceCaret'
import type { MeshData } from '../src/core/mesh'
import { createTextareaEditorAdapter, type TextareaLike } from '../src/editor/EditorAdapter'
import type { WebGPURenderer } from '../src/render/webgpuRenderer'

/**
 * Input-latency baseline of the textarea editor (ROADMAP P4). Measures the
 * main-thread work one keystroke triggers in the app model, without browser
 * layout/paint: adapter edit → `code` update → caret↔scene highlight sync
 * ("model"), then the syntax highlight / block structure recompute and the
 * visible gutter window ("highlight"). Numbers are logged for the design doc;
 * the assertion only guards against pathological regressions.
 */
function source(lines: number) {
  const out: string[] = []
  for (let i = 0; i < lines; i++) {
    const m = i % 10
    if (m === 0) out.push(`module part${i}(size = ${(i % 7) + 1}) {`)
    else if (m === 9) out.push('}')
    else out.push(`  translate([${i}, ${i % 13}, 0]) cube([size, size * 2, ${m}]); // part ${i}`)
  }
  return out.join('\n')
}

function textarea(value: string): TextareaLike {
  return {
    value,
    selectionStart: 0,
    selectionEnd: 0,
    selectionDirection: 'none',
    scrollTop: 0,
    scrollLeft: 0,
    clientHeight: 800,
    setRangeText(text, start, end) {
      this.value = this.value.slice(0, start) + text + this.value.slice(end)
      this.selectionStart = this.selectionEnd = start + text.length
    },
    setSelectionRange(start, end) {
      this.selectionStart = start
      this.selectionEnd = end
    },
    focus() {},
    addEventListener() {},
    removeEventListener() {},
  }
}

function median(values: number[]) {
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)]
}

function measure(lines: number, keystrokes: number) {
  const initial = source(lines)
  const el = textarea(initial)
  const adapter = createTextareaEditorAdapter(ref(el))
  const code = ref(initial)
  const selectedEditorName = ref('')
  const viewport = useCodeEditorViewport({ code, selectedEditorName, editor: adapter, highlightRef: ref(null) })
  const meshes: MeshData[] = []
  for (let i = 0, offset = 0; i < lines; i += 10) {
    meshes.push({ provenance: [{ source: { id: i, start: offset, end: offset + 60 } }] } as unknown as MeshData)
    offset += 700
  }
  const caret = useSourceProvenanceCaret({
    code,
    editor: adapter,
    selectedEditorName,
    sceneMeshes: computed(() => meshes),
    sourceMatchesEditor: computed(() => true),
    getRenderer: () => ({ setSourceHighlight() {} }) as unknown as WebGPURenderer,
    showNotice: () => {},
    translate: k => k,
  })
  // Warm the computeds the way the first render does.
  void viewport.highlightedCode.value
  void viewport.visibleGutterRows.value
  const model: number[] = []
  const highlight: number[] = []
  const middle = initial.length >> 1
  for (let k = 0; k < keystrokes; k++) {
    const t0 = performance.now()
    adapter.replaceRange('x', middle + k, middle + k)
    code.value = adapter.getText()
    caret.syncSourceHighlightFromEditor()
    const t1 = performance.now()
    void viewport.highlightedCode.value
    void viewport.visibleGutterRows.value
    void viewport.visibleGuideRows.value.map(row => row.guides)
    const t2 = performance.now()
    model.push(t1 - t0)
    highlight.push(t2 - t1)
  }
  return { model: median(model), highlight: median(highlight), bytes: initial.length }
}

describe('editor input latency baseline', () => {
  it.each([5_000, 20_000])('measures keystroke latency on %i lines', lines => {
    const result = measure(lines, 15)
    console.log(
      `[editor-latency] lines=${lines} chars=${result.bytes} keystroke→model median=${result.model.toFixed(2)}ms ` +
        `highlight refresh median=${result.highlight.toFixed(2)}ms`,
    )
    expect(result.model + result.highlight).toBeLessThan(5_000)
  })
})
