import { onUnmounted, shallowRef, watch, type Ref } from 'svet'
import type { CodeMirrorEngine } from '../editor/codemirrorAdapter'
import type { MountedCodeMirror } from '../editor/codemirrorView'

export interface UseCodeMirrorEditorOptions {
  /** Host element the view mounts into; null when the textarea fallback is active. */
  host: Ref<HTMLElement | null>
  code: Ref<string>
  ariaLabel: () => string
  maxLength: number
  /** A user edit (typing, paste, IME commit, native undo) changed the document. */
  onUserEdit(): void
  onSelection(): void
  onKeydown(event: KeyboardEvent): void
}

/**
 * Lazily loads the CodeMirror chunk and keeps the view and the `code` model in
 * sync. Returns the engine ref that `createCodeMirrorEditorAdapter` wraps.
 */
export function useCodeMirrorEditor(options: UseCodeMirrorEditorOptions) {
  const engine = shallowRef<CodeMirrorEngine | null>(null)
  let mounted: MountedCodeMirror | null = null
  let disposed = false
  let generation = 0
  let ariaWatching = false

  const teardown = () => {
    generation++
    const host = mounted?.view.dom.parentElement
    host?.removeEventListener('keydown', onKeydown)
    mounted?.destroy()
    mounted = null
    engine.value = null
  }
  const onKeydown = (event: KeyboardEvent) => {
    // Keys CodeMirror already handled (undo, redo, cursor moves) stop here.
    if (!event.defaultPrevented) options.onKeydown(event)
  }

  watch(
    options.host,
    async host => {
      teardown()
      if (!host || disposed) return
      const ticket = generation
      const { mountCodeMirror } = await import('../editor/codemirrorView')
      if (ticket !== generation || disposed) return
      mounted = mountCodeMirror({
        parent: host,
        text: options.code.value,
        ariaLabel: options.ariaLabel(),
        maxLength: options.maxLength,
        onChange(text, user) {
          options.code.value = text
          if (user) options.onUserEdit()
        },
        onSelection: options.onSelection,
      })
      host.addEventListener('keydown', onKeydown)
      engine.value = mounted
      if (!ariaWatching) {
        ariaWatching = true
        watch(options.ariaLabel, label => mounted?.setAriaLabel(label))
      }
    },
    { immediate: true, flush: 'post' },
  )

  // External document loads (open file, example, workspace restore) replace the view's text.
  watch(
    options.code,
    text => {
      if (mounted && mounted.getText() !== text) mounted.setText(text, 'preserve')
    },
    { flush: 'sync' },
  )

  onUnmounted(() => {
    disposed = true
    teardown()
  })

  return { engine }
}
