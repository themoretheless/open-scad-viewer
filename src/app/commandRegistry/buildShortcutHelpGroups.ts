import { COMMAND_REGISTRY } from '../commandRegistry'
import type { BuildPaletteDescriptorsOptions, CommandDefinition, CommandId, CommandKeybinding, CommandLabelKey, CommandPaletteDescriptor, CommandScope, ElementLike, KeyboardCandidate, KeyboardEventInput, NormalizedKeyboardEvent, PaletteCommandDefinition, ResolveKeyboardCommandOptions, ShortcutHelpGroup, ShortcutHelpGroupId, ShortcutHelpRow, ShortcutPlatform } from '../commandRegistry'

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

export function bindingKeyLabel(binding: CommandKeybinding): string {
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

export function helpGroupFor(definition: CommandDefinition): ShortcutHelpGroupId {
  const keywords = new Set<string>(definition.keywords)
  const scopes = definition.scopes as readonly CommandScope[]
  if (keywords.has('inspect') || keywords.has('distance') || keywords.has('clipping')) return 'inspection'
  if (keywords.has('selection') || keywords.has('visibility') || keywords.has('vertex') || keywords.has('surface')) return 'selection'
  if (keywords.has('camera') || keywords.has('view') || keywords.has('projection') || keywords.has('history')) return 'navigation'
  if (scopes.includes('editor')) return 'editor'
  if (scopes.includes('global')) return 'workspace'
  return 'display'
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

export function normalizeKey(key: string): string {
  if (key === 'Esc') return 'escape'
  if (key === 'Spacebar' || key === 'Space') return ' '
  return key.toLowerCase()
}

export function getBindings(definition: CommandDefinition): readonly CommandKeybinding[] {
  return 'bindings' in definition ? definition.bindings : []
}

export function compareKeyboardCandidates(a: KeyboardCandidate, b: KeyboardCandidate): number {
  return b.scopeRank - a.scopeRank
    || b.priority - a.priority
    || b.modifierCount - a.modifierCount
    || a.commandOrder - b.commandOrder
}

export function sameKeyboardPrecedence(a: KeyboardCandidate, b: KeyboardCandidate): boolean {
  return a.scopeRank === b.scopeRank
    && a.priority === b.priority
    && a.modifierCount === b.modifierCount
}

export function isNormalizedKeyboardEvent(
  event: KeyboardEventInput | NormalizedKeyboardEvent,
): event is NormalizedKeyboardEvent {
  return 'editableTarget' in event && 'primary' in event
}

export function asElementLike(value: unknown): ElementLike | null {
  return typeof value === 'object' && value !== null ? value as ElementLike : null
}
