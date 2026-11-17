import { nextTick, type Ref } from 'svet'
import { formatCode } from '../language/codeFormat'
import { indentSelection } from '../language/editorBlocks'
import type { EditorAdapter } from '../editor/EditorAdapter'
interface Options {
  code: Ref<string>
  editor: EditorAdapter
  foldedLines: Ref<Set<number>>
  syncHighlightScroll: () => void
  handleEditorInput: () => void
  dispatchKeyboardCommand: (event: KeyboardEvent, scope: 'editor') => void
}
export function useEditorFormatting(options: Options) {
  const { code, editor, foldedLines, syncHighlightScroll, handleEditorInput, dispatchKeyboardCommand } = options
  function formatEditor() {
    const formatted = formatCode(code.value)
    if (formatted === code.value) return
    const caret = editor.mounted ? editor.getSelection().start : 0
    editor.setText(formatted, 'preserve')
    code.value = formatted
    foldedLines.value = new Set()
    void nextTick(() => {
      if (editor.mounted) {
        editor.focus()
        editor.setSelection(Math.min(caret, formatted.length), Math.min(caret, formatted.length))
      }
      syncHighlightScroll()
    })
    handleEditorInput()
  }

  function handleEditorKey(event: KeyboardEvent) {
    if (event.altKey && event.shiftKey && event.code === 'KeyF') {
      event.preventDefault()
      formatEditor()
      return
    }
    if (event.key === 'Tab' && !event.ctrlKey && !event.metaKey && !event.altKey && !event.isComposing) {
      event.preventDefault()
      if (!editor.mounted) return
      const selection = editor.getSelection()
      const edit = indentSelection(code.value, selection.start, selection.end, event.shiftKey)
      editor.replaceRange(edit.text, edit.start, edit.end, 'preserve')
      code.value = editor.getText()
      editor.setSelection(edit.selectionStart, edit.selectionEnd)
      handleEditorInput()
      return
    }
    dispatchKeyboardCommand(event, 'editor')
  }

  return { formatEditor, handleEditorKey }
}
