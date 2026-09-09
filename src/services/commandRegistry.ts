import type { PaletteCommand } from './commandSearch'

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

/** Build the help surface from the same definitions used by event routing. */
export function buildShortcutHelpGroups(
  resolveLabel: (key: CommandLabelKey) => string,
  platform: ShortcutPlatform = 'unknown',
): ShortcutHelpGroup[] {
  const order: readonly ShortcutHelpGroupId[] = [
    'workspace', 'editor', 'navigation', 'selection', 'inspection', 'display',
  ]
  const grouped = new Map<ShortcutHelpGroupId, ShortcutHelpRow[]>()
  for (const definition of COMMAND_REGISTRY) {
    if (!('bindings' in definition) || !definition.bindings?.length) continue
    const id = helpGroupFor(definition)
    const rows = grouped.get(id) ?? []
    rows.push({
      id: definition.id,
      label: resolveLabel(definition.labelKey),
      shortcuts: definition.bindings.map(binding => formatKeyboardBinding(binding, platform)),
      scopes: definition.scopes,
    })
    grouped.set(id, rows)
  }
  return order.flatMap(id => {
    const rows = grouped.get(id)
    return rows?.length ? [{ id, rows }] : []
  })
}

export type ShortcutPlatform = 'mac' | 'windows-linux' | 'unknown'

/** Platform-aware visual formatter for the actual binding matcher contract. */
export function formatKeyboardBinding(
  binding: CommandKeybinding,
  platform: ShortcutPlatform,
): string {
  const mac = platform === 'mac'
  const modifiers: string[] = []
  if (binding.primary) modifiers.push(platform === 'unknown' ? 'Ctrl/⌘' : (mac ? '⌘' : 'Ctrl'))
  if (binding.ctrl) modifiers.push(mac ? '⌃' : 'Ctrl')
  if (binding.meta) modifiers.push(mac ? '⌘' : 'Meta')
  if (binding.alt) modifiers.push(mac ? '⌥' : 'Alt')
  const key = bindingKeyLabel(binding)
  // The question-mark glyph already communicates Shift+/ on common layouts.
  if (binding.shift && key !== '?') modifiers.push(mac ? '⇧' : 'Shift')
  return mac
    ? `${modifiers.join('')}${key}`
    : [...modifiers, key].join('+')
}

function bindingKeyLabel(binding: CommandKeybinding): string {
  const codeLabels: Record<string, string> = {
    BracketLeft: '[', Equal: '=', Numpad0: 'Num 0', Numpad1: 'Num 1',
    Numpad3: 'Num 3', Numpad7: 'Num 7', Digit1: '1', Digit3: '3',
    Digit4: '4', Digit5: '5', Slash: '/',
  }
  if (binding.code) return codeLabels[binding.code] ?? binding.code
  const key = binding.key ?? ''
  if (key.length === 1 && /[a-z]/i.test(key)) return key.toLocaleUpperCase('en-US')
  return normalizeKey(key) === 'escape' ? 'Esc' : key
}

function helpGroupFor(definition: CommandDefinition): ShortcutHelpGroupId {
  const keywords = new Set<string>(definition.keywords)
  const scopes = definition.scopes as readonly CommandScope[]
  if (keywords.has('inspect') || keywords.has('distance') || keywords.has('clipping')) return 'inspection'
  if (keywords.has('selection') || keywords.has('visibility') || keywords.has('vertex') || keywords.has('surface')) return 'selection'
  if (keywords.has('camera') || keywords.has('view') || keywords.has('projection') || keywords.has('history')) return 'navigation'
  if (scopes.includes('editor')) return 'editor'
  if (scopes.includes('global')) return 'workspace'
  return 'display'
}

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

/** Materialize palette-ready commands without coupling registry metadata to Vue. */
export function buildPaletteDescriptors(
  options: BuildPaletteDescriptorsOptions,
): CommandPaletteDescriptor[] {
  const aliasResolvers = options.aliasResolvers?.length
    ? options.aliasResolvers
    : [options.resolveLabel]

  return COMMAND_REGISTRY
    .filter((definition): definition is PaletteCommandDefinition => definition.palette)
    .map(definition => {
      const state = options.state?.[definition.id]
      const aliases = definition.aliasKeys
        .flatMap(key => aliasResolvers.map(resolve => resolve(key)))
        .filter((value, index, values) => Boolean(value) && values.indexOf(value) === index)
      const mruRank = options.mru?.indexOf(definition.id) ?? -1

      return {
        id: definition.id,
        label: state?.label ?? options.resolveLabel(definition.labelKey),
        shortcut: 'shortcutDisplay' in definition ? definition.shortcutDisplay : '',
        keywords: definition.keywords,
        aliases,
        ...(state?.detail ? { detail: state.detail } : {}),
        ...(state?.enabled === false ? { enabled: false } : {}),
        ...(state?.disabledReason ? { disabledReason: state.disabledReason } : {}),
        ...(mruRank >= 0 ? { mruRank } : {}),
      }
    })
}

export function getCommandDefinition(id: CommandId): CommandDefinition {
  return COMMAND_REGISTRY.find(definition => definition.id === id) as CommandDefinition
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

export function normalizeKeyboardEvent(event: KeyboardEventInput): NormalizedKeyboardEvent {
  const ctrl = Boolean(event.ctrlKey)
  const meta = Boolean(event.metaKey)
  return {
    key: normalizeKey(event.key),
    code: event.code ?? '',
    ctrl,
    meta,
    primary: ctrl || meta,
    alt: Boolean(event.altKey),
    shift: Boolean(event.shiftKey),
    defaultPrevented: Boolean(event.defaultPrevented),
    composing: Boolean(event.isComposing),
    repeat: Boolean(event.repeat),
    editableTarget: isEditableTarget(event.target),
  }
}

/** Match one binding with exact modifiers so Ctrl+G cannot accidentally toggle Grid. */
export function matchesKeyboardBinding(
  event: NormalizedKeyboardEvent,
  binding: CommandKeybinding,
): boolean {
  if (event.defaultPrevented || event.composing || event.repeat) return false
  if (event.editableTarget && !binding.allowInEditable) return false

  if (binding.code) {
    if (event.code !== binding.code) return false
  } else if (binding.key) {
    if (event.key !== normalizeKey(binding.key)) return false
  } else {
    return false
  }

  if (binding.primary) {
    // Accept one platform primary modifier, never the ambiguous Ctrl+Meta pair.
    if (!event.primary || event.ctrl === event.meta) return false
  } else {
    if (event.ctrl !== Boolean(binding.ctrl)) return false
    if (event.meta !== Boolean(binding.meta)) return false
  }

  const layoutShiftedSymbol = Boolean(
    binding.key
    && binding.shift === undefined
    && normalizeKey(binding.key).length === 1
    && !/[\p{L}\p{N}]/u.test(normalizeKey(binding.key)),
  )
  return event.alt === Boolean(binding.alt)
    && (layoutShiftedSymbol || event.shift === Boolean(binding.shift))
}

export interface ResolveKeyboardCommandOptions {
  /** Runtime guards decide contextual actions such as Escape and Shift+F. */
  readonly isEnabled?: (id: CommandId) => boolean
}

interface KeyboardCandidate {
  readonly id: CommandId
  readonly binding: CommandKeybinding
  readonly commandOrder: number
  readonly scopeRank: number
  readonly priority: number
  readonly modifierCount: number
}

/**
 * Resolve a shortcut deterministically. Exact-scope bindings beat global
 * fallbacks, then explicit priority and modifier specificity are considered.
 * A true tie is treated as ambiguous and safely resolves to null.
 */
export function resolveKeyboardCommand(
  input: KeyboardEventInput | NormalizedKeyboardEvent,
  scope: CommandScope,
  options: ResolveKeyboardCommandOptions = {},
): CommandId | null {
  const event = isNormalizedKeyboardEvent(input) ? input : normalizeKeyboardEvent(input)
  if (event.defaultPrevented || event.composing) return null

  const candidates: KeyboardCandidate[] = []
  COMMAND_REGISTRY.forEach((definition, commandOrder) => {
    if (options.isEnabled && !options.isEnabled(definition.id)) return
    for (const binding of getBindings(definition)) {
      const scopeRank = binding.scope === scope ? 1 : binding.scope === 'global' ? 0 : -1
      if (scopeRank < 0 || !matchesKeyboardBinding(event, binding)) continue
      candidates.push({
        id: definition.id,
        binding,
        commandOrder,
        scopeRank,
        priority: binding.priority ?? 0,
        modifierCount: Number(Boolean(binding.primary))
          + Number(Boolean(binding.ctrl))
          + Number(Boolean(binding.meta))
          + Number(Boolean(binding.alt))
          + Number(Boolean(binding.shift)),
      })
    }
  })

  candidates.sort(compareKeyboardCandidates)
  const first = candidates[0]
  if (!first) return null
  const second = candidates[1]
  if (second && sameKeyboardPrecedence(first, second) && second.id !== first.id) return null
  return first.id
}

/** Avoid hijacking typing, native select controls and contenteditable editors. */
export function isEditableTarget(target: unknown): boolean {
  let current = asElementLike(target)
  let depth = 0
  while (current && depth < 32) {
    const tagName = typeof current.tagName === 'string' ? current.tagName.toLowerCase() : ''
    if (tagName === 'input' || tagName === 'textarea' || tagName === 'select' || tagName === 'option') {
      return true
    }
    if (current.isContentEditable === true || current.contentEditable === 'true') return true
    const role = typeof current.getAttribute === 'function'
      ? current.getAttribute('role')?.toLowerCase()
      : undefined
    if (role === 'textbox' || role === 'searchbox' || role === 'combobox' || role === 'spinbutton') {
      return true
    }
    current = asElementLike(current.parentElement ?? current.parentNode)
    depth += 1
  }
  return false
}

function normalizeKey(key: string): string {
  if (key === 'Esc') return 'escape'
  if (key === 'Spacebar' || key === 'Space') return ' '
  return key.toLowerCase()
}

function getBindings(definition: CommandDefinition): readonly CommandKeybinding[] {
  return 'bindings' in definition ? definition.bindings : []
}

function compareKeyboardCandidates(a: KeyboardCandidate, b: KeyboardCandidate): number {
  return b.scopeRank - a.scopeRank
    || b.priority - a.priority
    || b.modifierCount - a.modifierCount
    || a.commandOrder - b.commandOrder
}

function sameKeyboardPrecedence(a: KeyboardCandidate, b: KeyboardCandidate): boolean {
  return a.scopeRank === b.scopeRank
    && a.priority === b.priority
    && a.modifierCount === b.modifierCount
}

function isNormalizedKeyboardEvent(
  event: KeyboardEventInput | NormalizedKeyboardEvent,
): event is NormalizedKeyboardEvent {
  return 'editableTarget' in event && 'primary' in event
}

interface ElementLike {
  readonly tagName?: unknown
  readonly isContentEditable?: unknown
  readonly contentEditable?: unknown
  readonly parentElement?: unknown
  readonly parentNode?: unknown
  readonly getAttribute?: (name: string) => string | null
}

function asElementLike(value: unknown): ElementLike | null {
  return typeof value === 'object' && value !== null ? value as ElementLike : null
}
