import type { Ref } from 'vue'
import { storageSet } from '../services/safeStorage'
import { clampEditorWidth, editorWidthBounds } from '../services/layoutSizing'
interface Options {
  mainRef: Ref<HTMLElement | null>
  editorWidth: Ref<number>
  editorMaxWidth: Ref<number>
}
export function useEditorResize(options: Options) {
  const { mainRef, editorWidth, editorMaxWidth } = options
  let resizing = false
function startResize(event: PointerEvent) {
  if (window.matchMedia('(max-width: 800px)').matches) return
  resizing = true
  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  resizeEditor(event.clientX)
}
function moveResize(event: PointerEvent) { if (resizing) resizeEditor(event.clientX) }
function stopResize() {
  if (!resizing) return
  resizing = false
  storageSet('scad-editor-width', String(Math.round(editorWidth.value)))
}
function resizeEditor(clientX: number) {
  const rect = mainRef.value?.getBoundingClientRect()
  if (!rect) return
  editorMaxWidth.value = editorWidthBounds(rect.width).max
  editorWidth.value = clampEditorWidth(clientX - rect.left, rect.width)
}
function resizeEditorWithKeyboard(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const width = mainRef.value?.getBoundingClientRect().width ?? 0
  const bounds = editorWidthBounds(width)
  editorMaxWidth.value = bounds.max
  if (event.key === 'Home') editorWidth.value = bounds.min
  else if (event.key === 'End') editorWidth.value = bounds.max
  else editorWidth.value = clampEditorWidth(editorWidth.value + (event.key === 'ArrowRight' ? 20 : -20), width)
  storageSet('scad-editor-width', String(Math.round(editorWidth.value)))
}

  return { startResize, moveResize, stopResize, resizeEditorWithKeyboard }
}
