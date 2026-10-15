import { nextTick, type Ref } from 'vue'
import { formatCode } from '../services/codeFormat'
import { indentSelection } from '../services/editorBlocks'
interface Options {
  code: Ref<string>
  editorRef: Ref<HTMLTextAreaElement | null>
  foldedLines: Ref<Set<number>>
  syncHighlightScroll: () => void
  handleEditorInput: () => void
  dispatchKeyboardCommand: (event: KeyboardEvent, scope: 'editor') => void
}
export function useEditorFormatting(options: Options) {
  const { code, editorRef, foldedLines, syncHighlightScroll, handleEditorInput, dispatchKeyboardCommand } = options
function formatEditor() {
  const formatted = formatCode(code.value)
  if(formatted === code.value) return
  const editor = editorRef.value
  const caret = editor?.selectionStart ?? 0
  if(editor) editor.setRangeText(formatted,0,editor.value.length,'preserve')
  code.value = formatted
  foldedLines.value = new Set()
  void nextTick(()=>{
    if(editor) { editor.focus(); editor.setSelectionRange(Math.min(caret,formatted.length),Math.min(caret,formatted.length)) }
    syncHighlightScroll()
  })
  handleEditorInput()
}

function handleEditorKey(event: KeyboardEvent) {
  if(event.altKey && event.shiftKey && event.code === 'KeyF') { event.preventDefault(); formatEditor(); return }
  if(event.key === 'Tab' && !event.ctrlKey && !event.metaKey && !event.altKey && !event.isComposing) {
    event.preventDefault()
    const editor=editorRef.value; if(!editor) return
    const edit=indentSelection(code.value,editor.selectionStart,editor.selectionEnd,event.shiftKey)
    editor.setRangeText(edit.text,edit.start,edit.end,'preserve')
    code.value=editor.value; editor.setSelectionRange(edit.selectionStart,edit.selectionEnd)
    handleEditorInput(); return
  }
  dispatchKeyboardCommand(event, 'editor')
}

  return { formatEditor, handleEditorKey }
}
