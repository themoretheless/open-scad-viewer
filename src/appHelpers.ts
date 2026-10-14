// Pure helpers of App.vue: no component state, extracted by scripts/vue-extract-pure.mjs.
import { type ComponentPublicInstance } from 'vue'
import { type RovingFocusKey } from './services/rovingFocus'
import { encodeWorkspaceShare } from './services/workspaceShare'
import { clamp } from './services/math3d'
import { storageGetJSON } from './services/safeStorage'

export const blockColors = ['#7999e8', '#c792ea', '#d7a457', '#55bba4', '#d87d9d']

// Virtualized gutter: only the visible window of rows (± overscan) is rendered.
// Row heights must match the CSS: .gutter-row is 1.58em at 12.5px, .guide-row is 1.58em at .82rem.
export const GUTTER_OVERSCAN_ROWS = 12

export const GUTTER_CONTENT_PADDING_PX = 14

export const gutterRowPx = 1.58 * 12.5

export type DockTab = 'scene' | 'inspect' | 'parameters' | 'svg' | 'photo' | 'perf'

export const MAIN_COMMAND_PREFIX = 'main:'

export const dockTabs: readonly DockTab[] = ['scene', 'inspect', 'parameters', 'svg', 'photo', 'perf']

export const DOCK_ICONS: Record<DockTab, string> = {
  scene: 'M4 6h16M4 12h16M4 18h16',
  inspect: 'M12 3v3M12 18v3M3 12h3M18 12h3M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10z',
  parameters: 'M4 8h10M18 8h2M4 16h4M12 16h8M16 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM10 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
  svg: 'M4 4h16v16H4zM4 15l5-5 4 4 3-3 4 4M15 8h.01',
  photo: 'M4 8h3l2-3h6l2 3h3v11H4zM12 17a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z',
  perf: 'M4 20V10M10 20V4M16 20v-8M22 20H2',
}

// Panels moved into the dock are <details> components; open them when their tab shows.
export function revealDockDetails(el: Element | ComponentPublicInstance | null) {
  if (!(el instanceof HTMLElement)) return
  for (const details of el.querySelectorAll<HTMLDetailsElement>(':scope > details')) {
    if (!details.open) details.open = true
  }
}

export const DOCK_KEY_MAP: Record<string, RovingFocusKey> = {
  ArrowLeft: 'ArrowLeft', ArrowUp: 'ArrowLeft', ArrowRight: 'ArrowRight', ArrowDown: 'ArrowRight', Home: 'Home', End: 'End',
}

export function closeMenus() {
  for (const menu of document.querySelectorAll<HTMLDetailsElement>('.app details.menu[open]')) menu.open = false
}

export function closeMenusOutside(event: Event) {
  const target = event.target as Element | null
  if (target?.closest('details.menu')) return
  closeMenus()
}

/** Rate limit for the storage-write-failure notice. */
export const STORAGE_NOTICE_THROTTLE_MS = 10_000

export const buildSources = new Map<number, string>()

export function downloadBlob(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = name
  link.click()
  URL.revokeObjectURL(url)
}

export function replaceSharedRecoveryHash(source: string) {
  const url = new URL(window.location.href)
  url.hash = `code=${encodeWorkspaceShare(source)}`
  history.replaceState(null, '', url)
}

export function hexToRgb(hex: string): [number, number, number] {
  const value = /^#[0-9a-f]{6}$/i.test(hex) ? hex : '#ffffff'
  return [1, 3, 5].map(offset => Number.parseInt(value.slice(offset, offset + 2), 16) / 255) as [number, number, number]
}

export function lineAndColumn(source: string, offset: number) {
  const before = source.slice(0, clamp(offset, 0, source.length))
  const line = before.split('\n').length
  return { line, column: before.length - before.lastIndexOf('\n') }
}

export function readCommandMru(): string[] {
  const value = storageGetJSON<unknown[]>('scad-command-mru', [], Array.isArray)
  return [...new Set(value.filter((item): item is string => typeof item === 'string' && item.length > 0))].slice(0, 12)
}

export function sanitizeFileName(name: string) { return (name.replace(/[^\w.() -]+/g, '_') || 'model.scad').replace(/\.r.*$/i, '.r').replace(/\.scad.*$/i, '.scad') }
