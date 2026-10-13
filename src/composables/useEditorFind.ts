import { computed, nextTick, ref, watch, type Ref } from 'vue'
import {
  findLiteralMatches,
  replaceAllLiteral,
  replaceExpectedMatch,
  wrappedMatchIndex,
} from '../services/textSearch'

export interface EditorFindOptions {
  /** The edited source; replacements write back to it. */
  code: Ref<string>
  editor: Ref<HTMLTextAreaElement | null>
  input: Ref<HTMLInputElement | null>
  translate(key: string): string
  notify(message: string): void
  /** Runs after a replacement changed the editor text, like a user edit. */
  onEdited(): void
}

/** Literal find / replace over the source editor, with wrap-around navigation. */
export function useEditorFind(options: EditorFindOptions) {
  const { code, editor: editorRef, input: findInputRef, translate: t } = options
  const findOpen = ref(false)
  const replaceOpen = ref(false)
  const findQuery = ref('')
  const findReplacement = ref('')
  const findCaseSensitive = ref(false)
  const activeFindIndex = ref(-1)
  const findResult = computed(() => findLiteralMatches(code.value, findQuery.value, {
    caseSensitive: findCaseSensitive.value,
  }))
  const findStatus = computed(() => {
    if (findResult.value.truncated) return t('tooManyMatches')
    const count = findResult.value.matches.length
    return count ? `${Math.max(0, activeFindIndex.value) + 1}/${count}` : t('noMatches')
  })
  watch([findQuery, findCaseSensitive, code], () => {
    activeFindIndex.value = findResult.value.matches.length ? 0 : -1
  })

  function openEditorFind(withReplace = false) {
    const editor = editorRef.value
    if (!findOpen.value && editor && editor.selectionStart !== editor.selectionEnd) {
      findQuery.value = editor.value.slice(editor.selectionStart, editor.selectionEnd)
    }
    findOpen.value = true
    replaceOpen.value = withReplace
    activeFindIndex.value = findResult.value.matches.length ? 0 : -1
    // The command palette restores its opener after closing. A second render
    // turn makes the explicit Find target win that documented focus handoff.
    void nextTick(() => nextTick(() => {
      findInputRef.value?.focus()
      findInputRef.value?.select()
      revealFindMatch()
    }))
  }

  function closeEditorFind() {
    findOpen.value = false
    replaceOpen.value = false
    void nextTick(() => editorRef.value?.focus())
  }

  function revealFindMatch() {
    const match = findResult.value.matches[activeFindIndex.value]
    if (!match || !editorRef.value) return
    editorRef.value.setSelectionRange(match.start, match.end)
  }

  function navigateFind(direction: 1 | -1) {
    activeFindIndex.value = wrappedMatchIndex(
      activeFindIndex.value,
      findResult.value.matches.length,
      direction,
    )
    revealFindMatch()
  }

  function handleFindKeydown(event: KeyboardEvent) {
    if (event.isComposing) return
    if (event.key === 'Escape') {
      event.preventDefault()
      closeEditorFind()
    } else if (event.key === 'Enter') {
      event.preventDefault()
      navigateFind(event.shiftKey ? -1 : 1)
    }
  }

  function replaceCurrentFindMatch() {
    const editor = editorRef.value
    const match = findResult.value.matches[activeFindIndex.value]
    if (!editor || !match) return
    if (replaceExpectedMatch(code.value, match, findReplacement.value) === null) {
      activeFindIndex.value = findResult.value.matches.length ? 0 : -1
      revealFindMatch()
      return
    }
    editor.setRangeText(findReplacement.value, match.start, match.end, 'select')
    code.value = editor.value
    options.onEdited()
    activeFindIndex.value = findResult.value.matches.length
      ? Math.min(activeFindIndex.value, findResult.value.matches.length - 1)
      : -1
    revealFindMatch()
  }

  function replaceAllFindMatches() {
    const editor = editorRef.value
    if (!editor) return
    const result = replaceAllLiteral(code.value, findQuery.value, findReplacement.value, {
      caseSensitive: findCaseSensitive.value,
    })
    if (result.status === 'too-many' || result.status === 'too-large') {
      options.notify(t('tooManyMatches'))
      return
    }
    if (result.status !== 'replaced') return
    editor.setRangeText(result.source, 0, editor.value.length, 'preserve')
    code.value = editor.value
    options.onEdited()
    activeFindIndex.value = findResult.value.matches.length ? 0 : -1
    revealFindMatch()
  }

  return {
    findOpen, replaceOpen, findQuery, findReplacement, findCaseSensitive, activeFindIndex, findResult, findStatus,
    openEditorFind, closeEditorFind, revealFindMatch, navigateFind, handleFindKeydown,
    replaceCurrentFindMatch, replaceAllFindMatches,
  }
}
