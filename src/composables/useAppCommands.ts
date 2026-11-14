import { computed, ref, type Ref } from 'svet'
import {
  buildPaletteDescriptors,
  resolveKeyboardCommand,
  type CommandId,
  type CommandScope,
  type PaletteCommandId,
  type PaletteCommandRuntimeState,
} from '../app/commandRegistry'
import { isPaletteCommandEnabled, type PaletteCommand } from '../app/commandSearch'
import { storageSetJSON } from '../core/safeStorage'
import type { MeshData } from '../core/mesh'
import { translate } from '../i18n/appStrings'
import type { SelectionMode, StandardView } from '../render/rendererContracts'
import { MAIN_COMMAND_PREFIX, readCommandMru } from '../appHelpers'
import type { useEditorFind } from './useEditorFind'
import type { useSectionPlaneControls } from './useSectionPlaneControls'
import type { useViewportDisplayControls } from './useViewportDisplayControls'
import type { useWorkspaceFileActions } from './useWorkspaceFileActions'

/** Direct-modeling tools of the Code viewport, reached through the command palette. */
export interface MainToolsHandle {
  commands?: readonly PaletteCommand[]
  execute(id: string): void
}

export interface UsePaletteCommandsOptions {
  t: (key: string) => string
  sceneMeshes: Ref<MeshData[]>
  meshVisibility: Ref<boolean[]>
  selectedMesh: Ref<number | null>
  isolated: Ref<boolean>
  rendering: Ref<boolean>
  measureActive: Ref<boolean>
  canExport: () => boolean
  canPreviousView: Ref<boolean>
  editorOpen: Ref<boolean>
  mainTools: () => MainToolsHandle | null
  viewport: Pick<ReturnType<typeof useViewportDisplayControls>, 'projection'>
  section: Pick<ReturnType<typeof useSectionPlaneControls>, 'sectionAvailable' | 'sectionEnabled'>
}

/** Palette descriptors with their runtime availability, most-recently-used order and keyboard gating. */
export function usePaletteCommands(options: UsePaletteCommandsOptions) {
  const { t, selectedMesh, measureActive } = options
  const commandMru = ref<string[]>(readCommandMru())

  const paletteCommands = computed(() => {
    const hasVisibleModel = options.sceneMeshes.value.some((_, index) => options.meshVisibility.value[index] !== false)
    const hasSelection = selectedMesh.value !== null
    const needsModel = { enabled: hasVisibleModel, disabledReason: t('needsModel') }
    const needsSelection = { enabled: hasSelection, disabledReason: t('needsSelection') }
    const needsFullBuild = { enabled: options.canExport(), disabledReason: t('needsFullBuild') }
    const state: Partial<Record<PaletteCommandId, PaletteCommandRuntimeState>> = {
      render: { enabled: !options.rendering.value, disabledReason: t('buildInProgress') },
      'export-stl': needsFullBuild,
      'export-obj': needsFullBuild,
      focus: { enabled: hasSelection || hasVisibleModel, disabledReason: t('needsModel') },
      isolate: { label: options.isolated.value ? t('unisolate') : t('isolate'), ...needsSelection },
      deselect: needsSelection,
      fit: needsModel,
      'previous-view': { enabled: options.canPreviousView.value, disabledReason: t('noPreviousView') },
      projection: {
        label: options.viewport.projection.value === 'perspective' ? t('orthographic') : t('perspective'),
      },
      measure: { enabled: hasVisibleModel || measureActive.value, disabledReason: t('needsModel') },
      section: {
        enabled: options.section.sectionAvailable.value || options.section.sectionEnabled.value,
        disabledReason: t('needsModel'),
      },
    }
    return buildPaletteDescriptors({
      resolveLabel: key => t(key),
      aliasResolvers: [key => translate('ru', key), key => translate('en', key)],
      state,
      mru: commandMru.value,
    })
  })

  const visiblePaletteCommands = computed<PaletteCommand[]>(() => {
    const base = paletteCommands.value.filter(isPaletteCommandEnabled)
    // The source tools only make sense while the editor drawer is showing.
    if (!options.editorOpen.value) return base
    const tools = options.mainTools()?.commands ?? []
    return [
      ...base,
      ...tools
        .filter(command => command.enabled !== false)
        .map(command => ({ ...command, id: MAIN_COMMAND_PREFIX + command.id })),
    ]
  })

  function recordCommandUsage(id: string) {
    const next = [id, ...commandMru.value.filter(candidate => candidate !== id)].slice(0, 12)
    commandMru.value = next
    storageSetJSON('scad-command-mru', next)
  }

  function isCommandEnabled(id: CommandId): boolean {
    if (id === 'cancel-measure') return measureActive.value
    if (id === 'flip-section') return options.section.sectionEnabled.value
    if (id === 'hide-selected') return selectedMesh.value !== null
    const target = paletteCommands.value.find(candidate => candidate.id === id)
    return target ? isPaletteCommandEnabled(target) : true
  }

  return { commandMru, paletteCommands, visiblePaletteCommands, recordCommandUsage, isCommandEnabled }
}

export interface UseCommandDispatchOptions {
  palette: ReturnType<typeof usePaletteCommands>
  paletteOpen: Ref<boolean>
  shortcutHelpOpen: Ref<boolean>
  /** Open surfaces that own the keyboard; shortcuts stay inert while any is open. */
  blockingSurfaceOpen: () => boolean
  mainTools: () => MainToolsHandle | null
  openFunctionReferenceFromCommand: (paletteWasOpen: boolean) => void
  openCommandPalette: () => void
  renderFull: () => void
  toggleTheme: () => void
  dockOpen: Ref<boolean>
  measureActive: Ref<boolean>
  selectedMesh: Ref<number | null>
  meshVisibility: Ref<boolean[]>
  viewport: ReturnType<typeof useViewportDisplayControls>
  section: ReturnType<typeof useSectionPlaneControls>
  files: Pick<
    ReturnType<typeof useWorkspaceFileActions>,
    'triggerOpen' | 'saveSource' | 'exportStl' | 'exportObj' | 'triggerConvert' | 'shareSource'
  >
  openEditorFind: ReturnType<typeof useEditorFind>['openEditorFind']
}

const STANDARD_VIEWS: readonly string[] = ['iso', 'front', 'back', 'left', 'right', 'top', 'bottom']
const SELECTION_MODES: SelectionMode[] = ['point', 'face', 'object']

/** Runs palette and keyboard commands of the Source workspace. */
export function useCommandDispatch(options: UseCommandDispatchOptions) {
  const { palette, paletteOpen, viewport: v, section, files } = options
  const actions: Record<string, () => void> = {
    render: options.renderFull,
    open: files.triggerOpen,
    save: files.saveSource,
    find: () => options.openEditorFind(false),
    replace: () => options.openEditorFind(true),
    'shortcut-help': () => (options.shortcutHelpOpen.value = true),
    'export-stl': files.exportStl,
    'export-obj': files.exportObj,
    'convert-mesh': files.triggerConvert,
    share: () => void files.shareSource(),
    fit: v.fitView,
    focus: v.focusSelection,
    reset: v.resetView,
    'previous-view': v.previousView,
    isolate: v.toggleIsolate,
    deselect: v.clearSelection,
    projection: v.toggleProjection,
    grid: v.toggleGrid,
    shaded: () => v.setDisplayMode('shaded'),
    edges: () => v.setDisplayMode('edges'),
    xray: () => v.setDisplayMode('xray'),
    'shading-phong': () => v.setShadingModel('phong'),
    'shading-pbr': () => v.setShadingModel('pbr'),
    'shading-matcap': () => v.setShadingModel('matcap'),
    'shading-toon': () => v.setShadingModel('toon'),
    'shading-unlit': () => v.setShadingModel('unlit'),
    'render-theme-default': () => v.setRenderTheme('default'),
    'render-theme-dark-contrast': () => v.setRenderTheme('dark-contrast'),
    'render-theme-light': () => v.setRenderTheme('light'),
    'select-point': () => v.setSelectionMode('point'),
    'select-face': () => v.setSelectionMode('face'),
    'select-object': () => v.setSelectionMode('object'),
    measure: () => (options.measureActive.value ? v.cancelMeasure() : v.startMeasure()),
    section: () => {
      section.scanPanelOpen.value = true
      section.setSectionEnabled(!section.sectionEnabled.value)
    },
    sidebar: () => (options.dockOpen.value = !options.dockOpen.value),
    theme: options.toggleTheme,
    'cancel-measure': v.cancelMeasure,
    'flip-section': () => section.setSectionFlip(!section.sectionFlip.value),
    'hide-selected': () => {
      const index = options.selectedMesh.value
      if (index !== null) v.setSceneMeshVisibility(index, options.meshVisibility.value[index] === false)
    },
    'cycle-selection-mode': () =>
      v.setSelectionMode(
        SELECTION_MODES[(SELECTION_MODES.indexOf(v.selectionMode.value) + 1) % SELECTION_MODES.length],
      ),
  }

  function executeCommand(id: string) {
    if (id.startsWith(MAIN_COMMAND_PREFIX)) {
      paletteOpen.value = false
      options.mainTools()?.execute(id.slice(MAIN_COMMAND_PREFIX.length))
      return
    }
    const target = palette.paletteCommands.value.find(candidate => candidate.id === id)
    if (target && !isPaletteCommandEnabled(target)) return
    const paletteWasOpen = paletteOpen.value
    paletteOpen.value = false
    if (target) palette.recordCommandUsage(id)
    if (STANDARD_VIEWS.includes(id)) v.setStandardView(id as StandardView)
    else if (id === 'function-reference') options.openFunctionReferenceFromCommand(paletteWasOpen)
    else if (id === 'command-palette') {
      if (!paletteWasOpen) options.openCommandPalette()
    } else if (Object.hasOwn(actions, id)) actions[id]()
  }

  function dispatchKeyboardCommand(event: KeyboardEvent, scope: CommandScope) {
    const id = resolveKeyboardCommand(event, scope, { isEnabled: palette.isCommandEnabled })
    if (!id) return
    if (options.shortcutHelpOpen.value || options.blockingSurfaceOpen()) return
    if (paletteOpen.value && id !== 'command-palette') return
    event.preventDefault()
    executeCommand(id)
  }

  return {
    executeCommand,
    dispatchKeyboardCommand,
    handleViewportKey: (event: KeyboardEvent) => dispatchKeyboardCommand(event, 'viewport'),
    handleGlobalKey: (event: KeyboardEvent) => dispatchKeyboardCommand(event, 'global'),
  }
}
