import { computed, nextTick, ref, watch, type Ref } from 'svet'
import type { EditorAdapter } from '../editor/EditorAdapter'
import {
  findLiteralMatches,
  replaceAllLiteral,
  replaceExpectedMatch,
  wrappedMatchIndex,
} from '../app/textSearch'

export interface EditorFindOptions {
  /** The edited source; replacements write back to it. */
  code: Ref<string>
  editor: EditorAdapter
  input: Ref<HTMLInputElement | null>
  translate(key: string): string
  notify(message: string): void
  /** Runs after a replacement changed the editor text, like a user edit. */
  onEdited(): void
}

/** Literal find / replace over the source editor, with wrap-around navigation. */
export function useEditorFind(options: EditorFindOptions) {
  const { code, editor, input: findInputRef, translate: t } = options
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
    const selection = editor.getSelection()
    if (!findOpen.value && editor.mounted && selection.start !== selection.end) {
      findQuery.value = editor.getText().slice(selection.start, selection.end)
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
    void nextTick(() => editor.focus())
  }

  function revealFindMatch() {
    const match = findResult.value.matches[activeFindIndex.value]
    if (!match || !editor.mounted) return
    editor.setSelection(match.start, match.end)
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
    const match = findResult.value.matches[activeFindIndex.value]
    if (!editor.mounted || !match) return
    if (replaceExpectedMatch(code.value, match, findReplacement.value) === null) {
      activeFindIndex.value = findResult.value.matches.length ? 0 : -1
      revealFindMatch()
      return
    }
    editor.replaceRange(findReplacement.value, match.start, match.end, 'select')
    code.value = editor.getText()
    options.onEdited()
    activeFindIndex.value = findResult.value.matches.length
      ? Math.min(activeFindIndex.value, findResult.value.matches.length - 1)
      : -1
    revealFindMatch()
  }

  function replaceAllFindMatches() {
    if (!editor.mounted) return
    const result = replaceAllLiteral(code.value, findQuery.value, findReplacement.value, {
      caseSensitive: findCaseSensitive.value,
    })
    if (result.status === 'too-many' || result.status === 'too-large') {
      options.notify(t('tooManyMatches'))
      return
    }
    if (result.status !== 'replaced') return
    editor.setText(result.source, 'preserve')
    code.value = editor.getText()
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
