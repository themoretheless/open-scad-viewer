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

export interface CommandDefinitionShape {
  readonly id: string
  readonly labelKey: string
  readonly aliasKeys: readonly string[]
  readonly shortcutDisplay?: string
  readonly keywords: readonly string[]
  readonly scopes: readonly CommandScope[]
  readonly palette: boolean
  readonly bindings?: readonly CommandKeybinding[]
}
