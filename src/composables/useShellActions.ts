import { nextTick, ref, type Ref } from 'svet'
import { nextRovingIndex } from '../app/rovingFocus'
import type { WorkspaceMode } from '../app/workspaceModes'
import type { SelectionMode } from '../render/rendererContracts'
import { DOCK_KEY_MAP, dockTabs, type DockTab } from '../appHelpers'
import type { useViewportDisplayControls } from './useViewportDisplayControls'

export interface UseShellActionsOptions {
  dockTab: Ref<DockTab>
  dockOpen: Ref<boolean>
  editorOpen: Ref<boolean>
  autoRender: Ref<boolean>
  viewport: ReturnType<typeof useViewportDisplayControls>
  openWorkspaceMode: (mode: WorkspaceMode) => void
  saveSource: () => void
  resolveWorkspaceConflict: (preferCurrentDraft: boolean) => Promise<unknown>
  retryWorkspacePersistence: () => unknown
  openCommandPalette: () => void
  openExportDialog: () => void
  renderFull: () => unknown
  cancelGeometryBuild: () => void
  cancelSolidBuild: () => void
  runGeometryAnalysis: () => Promise<void>
  cancelGeometryAnalysis: () => void
  cancelConversion: () => void
}

/** Named actions and bindings of the declarative shell regions (`src/ui/*.ui`), plus dock tab focus. */
export function useShellActions(options: UseShellActionsOptions) {
  const { dockTab, dockOpen, viewport: v } = options
  const dockToggleRef = ref<HTMLButtonElement | null>(null)

  function handleDockTabKeydown(event: KeyboardEvent) {
    const key = DOCK_KEY_MAP[event.key]
    if (!key) return
    event.preventDefault()
    const next = nextRovingIndex(dockTabs.indexOf(dockTab.value), dockTabs.length, key)
    dockTab.value = dockTabs[next]
    void nextTick(() => document.getElementById(`dock-tab-${dockTab.value}`)?.focus())
  }

  function closeDock() {
    dockOpen.value = false
    void nextTick(() => dockToggleRef.value?.focus())
  }

  const actions: Record<string, (args: unknown[], event?: Event) => void> = {
    openWorkspaceMode: args => options.openWorkspaceMode(args[0] as WorkspaceMode),
    saveSource: () => options.saveSource(),
    resolveConflict: args => void options.resolveWorkspaceConflict(Boolean(args[0])),
    retryPersistence: () => void options.retryWorkspacePersistence(),
    toggleEditor: () => (options.editorOpen.value = !options.editorOpen.value),
    openCommandPalette: () => options.openCommandPalette(),
    openExportDialog: () => options.openExportDialog(),
    renderFull: () => void options.renderFull(),
    cancelGeometryBuild: () => options.cancelGeometryBuild(),
    cancelSolidBuild: () => options.cancelSolidBuild(),
    fitView: () => v.fitView(),
    resetView: () => v.resetView(),
    previousView: () => v.previousView(),
    toggleProjection: () => v.toggleProjection(),
    toggleGrid: () => v.toggleGrid(),
    setDisplayMode: args => v.setDisplayMode(args[0] as 'shaded' | 'edges' | 'xray'),
    setSelectionMode: args => v.setSelectionMode(args[0] as SelectionMode),
    focusSelection: () => v.focusSelection(),
    toggleIsolate: () => v.toggleIsolate(),
    clearSelection: () => v.clearSelection(),
    dockTabKeydown: (_args, event) => {
      if (event) handleDockTabKeydown(event as KeyboardEvent)
    },
    selectDockTab: args => (dockTab.value = args[0] as DockTab),
    closeDock: () => closeDock(),
    runGeometryAnalysis: () => void options.runGeometryAnalysis(),
    cancelGeometryAnalysis: () => options.cancelGeometryAnalysis(),
    cancelConversion: () => options.cancelConversion(),
  }

  function handleShellAction(name: string, args: unknown[], event?: Event) {
    if (Object.hasOwn(actions, name)) actions[name](args, event)
  }

  function handleShellBinding(path: string, value: unknown) {
    if (path === 'autoRender') options.autoRender.value = Boolean(value)
  }

  return { dockToggleRef, handleDockTabKeydown, closeDock, handleShellAction, handleShellBinding }
}
