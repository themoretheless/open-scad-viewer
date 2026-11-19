import type { PaletteCommand } from './commandSearch'
export {
  buildShortcutHelpGroups,
  formatKeyboardBinding,
  buildPaletteDescriptors,
  getCommandDefinition,
  normalizeKeyboardEvent,
  matchesKeyboardBinding,
  resolveKeyboardCommand,
  isEditableTarget,
} from './commandRegistry/buildShortcutHelpGroups'

import type { CommandScope, CommandKeybinding } from './commandRegistry/commandBindings'
export type { CommandScope, CommandKeybinding } from './commandRegistry/commandBindings'
import { COMMAND_REGISTRY } from './commandRegistry/commands'
export { COMMAND_REGISTRY }

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
