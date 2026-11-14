import { nextTick, ref, watch, type Ref } from 'svet'
import { boundedSourceHistory, restoreSourceHistory, type SourceEdit } from '../app/mainSourceEditing'
import { MAX_WORKSPACE_SOURCE_LENGTH } from '../app/workspaceDocument'
import { storageGet, storageSet } from '../core/safeStorage'
import { isRushFrontend } from '../language/rushFrontendDetect'

export interface UseMainSourceHistoryOptions {
  code: Ref<string>
  fileName: Ref<string>
  lang: Ref<'ru' | 'en'>
  error: Ref<string>
  notice: Ref<string>
  selectedIndex: () => number | null
  previewMainGeometry: (meshes: null) => void
  setMainEditSelection: (selection: { source: string; index: number } | null) => void
  replaceSource: (source: string) => void
  renderFull: () => void
}

const MAX_EDITS = 30
const MAX_EDIT_CHARACTERS = 8_000_000

/** Undo history of geometry edits made from the Code viewport, persisted per file name. */
export function useMainSourceHistory(options: UseMainSourceHistoryOptions) {
  const { code, fileName, error } = options
  const storageKey = () => 'scad-source-history-v1:' + fileName.value
  const restored = restoreSourceHistory(storageGet(storageKey()), code.value)
  const past = ref<SourceEdit[]>(restored.past)
  const future = ref<SourceEdit[]>(restored.future)
  watch(
    fileName,
    () => {
      const history = restoreSourceHistory(storageGet(storageKey()), code.value)
      past.value = history.past
      future.value = history.future
    },
    { flush: 'post' },
  )
  watch(
    [past, future],
    () => {
      const history = boundedSourceHistory({ past: past.value, future: future.value })
      if (!storageSet(storageKey(), JSON.stringify(history)))
        options.notice.value = 'Could not persist modeling history.'
    },
    { deep: true, flush: 'post' },
  )

  function publish(source: string) {
    options.replaceSource(source)
    void nextTick(options.renderFull)
  }

  function commitMainSource(source: string, selectIndex = options.selectedIndex()) {
    try {
      if (source.length > MAX_WORKSPACE_SOURCE_LENGTH) throw Error('source limit')
      options.previewMainGeometry(null)
      if (source === code.value) return
      if (past.value.at(-1)?.after !== code.value) past.value = []
      past.value.push({ before: code.value, after: source })
      const size = () => past.value.reduce((n, e) => n + e.before.length + e.after.length, 0)
      while (past.value.length > MAX_EDITS || (past.value.length > 1 && size() > MAX_EDIT_CHARACTERS))
        past.value.shift()
      future.value = []
      options.setMainEditSelection(selectIndex === null ? null : { source, index: selectIndex })
      publish(source)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  function appendMainPrimitive(source: string) {
    if (isRushFrontend(code.value)) {
      error.value =
        options.lang.value === 'ru'
          ? 'Примитивы доступны в документе OpenSCAD.'
          : 'Primitives require an OpenSCAD document.'
      return
    }
    commitMainSource(code.value + '\n' + source, -1)
  }

  function undoMainGeometry(redo = false) {
    const from = redo ? future : past
    const to = redo ? past : future
    const entry = from.value.at(-1)
    if (!entry || code.value !== (redo ? entry.before : entry.after)) return
    options.previewMainGeometry(null)
    from.value.pop()
    to.value.push(entry)
    publish(redo ? entry.after : entry.before)
  }

  return { mainEditPast: past, mainEditFuture: future, commitMainSource, appendMainPrimitive, undoMainGeometry }
}
