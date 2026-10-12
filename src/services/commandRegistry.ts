import type { PaletteCommand } from './commandSearch'
export { buildShortcutHelpGroups, formatKeyboardBinding, buildPaletteDescriptors, getCommandDefinition, normalizeKeyboardEvent, matchesKeyboardBinding, resolveKeyboardCommand, isEditableTarget } from './commandRegistry/buildShortcutHelpGroups'

/** Keyboard focus region used to decide which shortcuts are active. */
export type CommandScope = 'global' | 'editor' | 'viewport'

export interface CommandKeybinding {
  /** Match a layout-aware KeyboardEvent.key (case-insensitive for letters). */
  readonly key?: string
  /** Match a layout-independent KeyboardEvent.code. Takes precedence over key. */
  readonly code?: string
  /** Ctrl on Windows/Linux or Command on macOS. */
  readonly primary?: boolean
  readonly ctrl?: boolean
  readonly meta?: boolean
  readonly alt?: boolean
  readonly shift?: boolean
  /** Global bindings are also considered from editor and viewport scopes. */
  readonly scope: CommandScope
  /** Global file/application shortcuts opt in; plain viewport keys stay guarded. */
  readonly allowInEditable?: boolean
  /** Higher values win when two intentionally overlapping bindings match. */
  readonly priority?: number
}

interface CommandDefinitionShape {
  readonly id: string
  readonly labelKey: string
  readonly aliasKeys: readonly string[]
  readonly shortcutDisplay?: string
  readonly keywords: readonly string[]
  readonly scopes: readonly CommandScope[]
  readonly palette: boolean
  readonly bindings?: readonly CommandKeybinding[]
}

/**
 * The single declarative inventory for palette actions and keyboard-only
 * actions. Keep command execution outside this module: the registry describes
 * intent and input routing, while the application owns effects.
 */
export const COMMAND_REGISTRY = [
  {
    id: 'render', labelKey: 'render', aliasKeys: ['render'],
    shortcutDisplay: 'Ctrl/⌘+Enter', keywords: ['build', 'compile', 'render'],
    scopes: ['editor'], palette: true,
    bindings: [{ key: 'Enter', primary: true, scope: 'editor', allowInEditable: true }],
  },
  {
    id: 'open', labelKey: 'open', aliasKeys: ['open'],
    shortcutDisplay: 'Ctrl/⌘+O', keywords: ['file', 'import'],
    scopes: ['global'], palette: true,
    bindings: [{ key: 'o', primary: true, scope: 'global', allowInEditable: true }],
  },
  {
    id: 'save', labelKey: 'save', aliasKeys: ['save'],
    shortcutDisplay: 'Ctrl/⌘+S', keywords: ['file', 'export'],
    scopes: ['global'], palette: true,
    bindings: [{ key: 's', primary: true, scope: 'global', allowInEditable: true }],
  },
  {
    id: 'find', labelKey: 'find', aliasKeys: ['find'],
    shortcutDisplay: 'Ctrl/⌘+F', keywords: ['search', 'editor', 'text'],
    scopes: ['editor'], palette: true,
    bindings: [{ key: 'f', primary: true, scope: 'editor', allowInEditable: true }],
  },
  {
    id: 'replace', labelKey: 'replace', aliasKeys: ['replace'],
    shortcutDisplay: 'Ctrl+H / ⌘⌥F', keywords: ['search', 'editor', 'text'],
    scopes: ['editor'], palette: true,
    bindings: [
      { key: 'h', ctrl: true, scope: 'editor', allowInEditable: true },
      { key: 'f', meta: true, alt: true, scope: 'editor', allowInEditable: true },
    ],
  },
  {
    id: 'export-stl', labelKey: 'exportStl', aliasKeys: ['exportStl'],
    keywords: ['mesh', 'manufacturing', 'print'], scopes: ['global'], palette: true,
  },
  {
    id: 'export-obj', labelKey: 'exportObj', aliasKeys: ['exportObj'],
    keywords: ['mesh', 'interchange'], scopes: ['global'], palette: true,
  },
  {
    id: 'convert-mesh', labelKey: 'convertFile', aliasKeys: ['convertFile'],
    keywords: ['convert', 'import', 'stl', 'obj', 'ply', 'off', 'amf', '3mf', 'конвертировать'], scopes: ['global'], palette: true,
  },
  {
    id: 'share', labelKey: 'share', aliasKeys: ['share'],
    keywords: ['link', 'copy'], scopes: ['global'], palette: true,
  },
  {
    id: 'focus', labelKey: 'focus', aliasKeys: ['focus'],
    shortcutDisplay: 'F /', keywords: ['selection', 'frame'],
    scopes: ['viewport'], palette: true,
    bindings: [
      { key: 'f', scope: 'viewport' },
      { key: '/', scope: 'viewport' },
    ],
  },
  {
    id: 'isolate', labelKey: 'isolate', aliasKeys: ['isolate', 'unisolate'],
    shortcutDisplay: '.', keywords: ['selection', 'visibility'],
    scopes: ['viewport'], palette: true,
    bindings: [{ key: '.', scope: 'viewport' }],
  },
  {
    id: 'deselect', labelKey: 'deselect', aliasKeys: ['deselect'],
    shortcutDisplay: 'Esc', keywords: ['selection', 'clear'],
    scopes: ['viewport'], palette: true,
    bindings: [{ key: 'Escape', scope: 'viewport' }],
  },
  {
    // F is intentionally owned by Focus, matching the existing viewport and
    // help text. The old palette-only "F" hint on Fit was contradictory.
    id: 'fit', labelKey: 'fit', aliasKeys: ['fit'],
    keywords: ['camera', 'frame', 'all'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'reset', labelKey: 'reset', aliasKeys: ['reset'],
    keywords: ['camera', 'home'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'previous-view', labelKey: 'previousView', aliasKeys: ['previousView'],
    shortcutDisplay: '[', keywords: ['camera', 'history', 'back'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'BracketLeft', scope: 'viewport' }],
  },
  {
    // The runtime label should describe the target projection mode.
    id: 'projection', labelKey: 'orthographic', aliasKeys: ['perspective', 'orthographic'],
    shortcutDisplay: '5', keywords: ['camera', 'projection'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Digit5', scope: 'viewport' }],
  },
  {
    id: 'grid', labelKey: 'grid', aliasKeys: ['grid'],
    shortcutDisplay: 'G', keywords: ['overlay'], scopes: ['viewport'], palette: true,
    bindings: [{ key: 'g', scope: 'viewport' }],
  },
  {
    id: 'shaded', labelKey: 'shaded', aliasKeys: ['shaded'],
    keywords: ['shader', 'display'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'edges', labelKey: 'edges', aliasKeys: ['edges'],
    shortcutDisplay: 'E', keywords: ['wireframe', 'mesh', 'display'],
    scopes: ['viewport'], palette: true,
    bindings: [{ key: 'e', scope: 'viewport' }],
  },
  {
    id: 'xray', labelKey: 'xray', aliasKeys: ['xray'],
    shortcutDisplay: 'X', keywords: ['transparent', 'display'],
    scopes: ['viewport'], palette: true,
    bindings: [{ key: 'x', scope: 'viewport' }],
  },
  {
    id: 'shading-phong', labelKey: 'shadingPhong', aliasKeys: ['shadingPhong'],
    keywords: ['shader', 'display', 'shading', 'material'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'shading-pbr', labelKey: 'shadingPbr', aliasKeys: ['shadingPbr'],
    keywords: ['shader', 'display', 'shading', 'material'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'shading-matcap', labelKey: 'shadingMatcap', aliasKeys: ['shadingMatcap'],
    keywords: ['shader', 'display', 'shading', 'material'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'shading-toon', labelKey: 'shadingToon', aliasKeys: ['shadingToon'],
    keywords: ['shader', 'display', 'shading', 'material'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'shading-unlit', labelKey: 'shadingUnlit', aliasKeys: ['shadingUnlit'],
    keywords: ['shader', 'display', 'shading', 'material'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'render-theme-default', labelKey: 'renderThemeDefault', aliasKeys: ['renderThemeDefault'],
    keywords: ['theme', 'display', 'colors'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'render-theme-dark-contrast', labelKey: 'renderThemeDarkContrast', aliasKeys: ['renderThemeDarkContrast'],
    keywords: ['theme', 'display', 'colors'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'render-theme-light', labelKey: 'renderThemeLight', aliasKeys: ['renderThemeLight'],
    keywords: ['theme', 'display', 'colors'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'select-point', labelKey: 'point', aliasKeys: ['point'],
    shortcutDisplay: '1', keywords: ['selection', 'vertex', 'snap'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Digit1', scope: 'viewport' }],
  },
  {
    id: 'select-face', labelKey: 'face', aliasKeys: ['face'],
    shortcutDisplay: '3', keywords: ['selection', 'surface'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Digit3', scope: 'viewport' }],
  },
  {
    id: 'select-object', labelKey: 'body', aliasKeys: ['body', 'object'],
    shortcutDisplay: '4', keywords: ['selection', 'body', 'solid'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Digit4', scope: 'viewport' }],
  },
  {
    id: 'measure', labelKey: 'measure', aliasKeys: ['measure'],
    shortcutDisplay: 'Ctrl/⌘+=', keywords: ['inspect', 'distance', 'dimension'],
    scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Equal', primary: true, scope: 'viewport' }],
  },
  {
    id: 'section', labelKey: 'section', aliasKeys: ['section', 'scanPlane'],
    keywords: ['inspect', 'slice', 'clipping', 'plane'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'sidebar', labelKey: 'sidebar', aliasKeys: ['sidebar'],
    shortcutDisplay: 'Ctrl/⌘+Shift+B', keywords: ['panel', 'outliner', 'inspect', 'parameters'],
    scopes: ['global'], palette: true,
    bindings: [{ key: 'b', primary: true, shift: true, scope: 'global', allowInEditable: true }],
  },
  {
    id: 'iso', labelKey: 'iso', aliasKeys: ['iso'], shortcutDisplay: 'Num 0',
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Numpad0', scope: 'viewport' }],
  },
  {
    id: 'front', labelKey: 'front', aliasKeys: ['front'], shortcutDisplay: 'Num 1',
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Numpad1', scope: 'viewport' }],
  },
  {
    id: 'right', labelKey: 'right', aliasKeys: ['right'], shortcutDisplay: 'Num 3',
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Numpad3', scope: 'viewport' }],
  },
  {
    id: 'top', labelKey: 'top', aliasKeys: ['top'], shortcutDisplay: 'Num 7',
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
    bindings: [{ code: 'Numpad7', scope: 'viewport' }],
  },
  {
    id: 'back', labelKey: 'back', aliasKeys: ['back'],
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'left', labelKey: 'left', aliasKeys: ['left'],
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'bottom', labelKey: 'bottom', aliasKeys: ['bottom'],
    keywords: ['camera', 'view'], scopes: ['viewport'], palette: true,
  },
  {
    id: 'theme', labelKey: 'theme', aliasKeys: ['theme'],
    keywords: ['dark', 'light'], scopes: ['global'], palette: true,
  },
  {
    id: 'function-reference', labelKey: 'functionReference', aliasKeys: ['functionReference'],
    shortcutDisplay: 'F1', keywords: ['functions', 'reference', 'help', 'documentation', 'translate', 'справочник', 'функции'],
    scopes: ['global'], palette: true,
    bindings: [{ key: 'F1', scope: 'global', allowInEditable: true }],
  },
  {
    id: 'shortcut-help', labelKey: 'shortcuts', aliasKeys: ['commandHelp'],
    shortcutDisplay: '?', keywords: ['keyboard', 'help', 'hotkeys'],
    scopes: ['global'], palette: true,
    bindings: [{ key: '?', scope: 'global' }],
  },
  {
    id: 'command-palette', labelKey: 'commandHelp', aliasKeys: ['commands'],
    shortcutDisplay: 'Ctrl/⌘+K', keywords: ['command', 'search'],
    scopes: ['global'], palette: false,
    bindings: [{ key: 'k', primary: true, scope: 'global', allowInEditable: true }],
  },
  {
    // Conditional Escape action. Enable only while a measurement is active;
    // otherwise the lower-priority Deselect binding wins.
    id: 'cancel-measure', labelKey: 'cancelMeasure', aliasKeys: ['measure'],
    keywords: ['cancel', 'escape'], scopes: ['viewport'], palette: false,
    bindings: [{ key: 'Escape', scope: 'viewport', priority: 100 }],
  },
  {
    id: 'flip-section', labelKey: 'flipSection', aliasKeys: ['section'],
    shortcutDisplay: 'Shift+F', keywords: ['flip', 'clipping', 'plane'],
    scopes: ['viewport'], palette: false,
    bindings: [{ key: 'f', shift: true, scope: 'viewport', priority: 10 }],
  },
  {
    id: 'hide-selected', labelKey: 'hideSelected', aliasKeys: ['hidden'],
    shortcutDisplay: 'H', keywords: ['hide', 'visibility', 'selection'],
    scopes: ['viewport'], palette: false,
    bindings: [{ key: 'h', scope: 'viewport' }],
  },
  {
    id: 'cycle-selection-mode', labelKey: 'cycleSelectionMode', aliasKeys: ['selectionMode'],
    shortcutDisplay: 'Shift+M', keywords: ['selection', 'cycle', 'mode'],
    scopes: ['viewport'], palette: false,
    bindings: [{ key: 'm', shift: true, scope: 'viewport' }],
  },
] as const satisfies readonly CommandDefinitionShape[]

export type CommandDefinition = (typeof COMMAND_REGISTRY)[number]
export type CommandId = CommandDefinition['id']
export type PaletteCommandDefinition = Extract<CommandDefinition, { readonly palette: true }>
export type PaletteCommandId = PaletteCommandDefinition['id']
export type CommandLabelKey = CommandDefinition['labelKey'] | CommandDefinition['aliasKeys'][number]

export interface CommandPaletteDescriptor extends Omit<PaletteCommand, 'id'> {
  id: PaletteCommandId
}

export type ShortcutHelpGroupId = 'workspace' | 'editor' | 'navigation' | 'selection' | 'inspection' | 'display'

export interface ShortcutHelpRow {
  readonly id: CommandId
  readonly label: string
  readonly shortcuts: readonly string[]
  readonly scopes: readonly CommandScope[]
}

export interface ShortcutHelpGroup {
  readonly id: ShortcutHelpGroupId
  readonly rows: readonly ShortcutHelpRow[]
}

export type ShortcutPlatform = 'mac' | 'windows-linux' | 'unknown'

export interface PaletteCommandRuntimeState {
  /** Overrides dynamic labels such as Isolate/Show all and projection target. */
  readonly label?: string
  readonly detail?: string
  readonly enabled?: boolean
  readonly disabledReason?: string
}

export interface BuildPaletteDescriptorsOptions {
  readonly resolveLabel: (key: CommandLabelKey) => string
  /** Supply all locale resolvers to keep bilingual aliases searchable. */
  readonly aliasResolvers?: readonly ((key: CommandLabelKey) => string)[]
  readonly state?: Partial<Record<PaletteCommandId, PaletteCommandRuntimeState>>
  readonly mru?: readonly string[]
}

export interface KeyboardEventInput {
  readonly key: string
  readonly code?: string
  readonly ctrlKey?: boolean
  readonly metaKey?: boolean
  readonly altKey?: boolean
  readonly shiftKey?: boolean
  readonly defaultPrevented?: boolean
  readonly isComposing?: boolean
  readonly repeat?: boolean
  readonly target?: unknown
}

export interface NormalizedKeyboardEvent {
  readonly key: string
  readonly code: string
  readonly ctrl: boolean
  readonly meta: boolean
  readonly primary: boolean
  readonly alt: boolean
  readonly shift: boolean
  readonly defaultPrevented: boolean
  readonly composing: boolean
  readonly repeat: boolean
  readonly editableTarget: boolean
}

export interface ResolveKeyboardCommandOptions {
  /** Runtime guards decide contextual actions such as Escape and Shift+F. */
  readonly isEnabled?: (id: CommandId) => boolean
}

export interface KeyboardCandidate {
  readonly id: CommandId
  readonly binding: CommandKeybinding
  readonly commandOrder: number
  readonly scopeRank: number
  readonly priority: number
  readonly modifierCount: number
}

export interface ElementLike {
  readonly tagName?: unknown
  readonly isContentEditable?: unknown
  readonly contentEditable?: unknown
  readonly parentElement?: unknown
  readonly parentNode?: unknown
  readonly getAttribute?: (name: string) => string | null
}
