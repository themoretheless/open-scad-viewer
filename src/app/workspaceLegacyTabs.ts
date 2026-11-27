import { isRushFrontend } from '../language/rushFrontendDetect'
import type { WorkspaceStorage } from './workspaceDocument'
import { MAX_WORKSPACE_FILE_NAME_LENGTH, MAX_WORKSPACE_SOURCE_LENGTH } from './workspaceSourceLimits'

export const WORKSPACE_LEGACY_TABS_KEY = 'scad-tabs'
export const WORKSPACE_LEGACY_ACTIVE_TAB_KEY = 'scad-active-tab'
const MAX_LEGACY_TABS = 64
const MAX_LEGACY_TABS_SERIALIZED_LENGTH = 4 * 1024 * 1024

/** Select the donor editor's active tab without trusting its unversioned shape. */
export function readLegacyWorkspaceTab(storage: WorkspaceStorage): {source: string; fileName: string} | null {
  try {
    const serialized = storage.getItem(WORKSPACE_LEGACY_TABS_KEY)
    if (!serialized || serialized.length > MAX_LEGACY_TABS_SERIALIZED_LENGTH) return null
    const value = JSON.parse(serialized) as unknown
    if (!Array.isArray(value) || value.length === 0 || value.length > MAX_LEGACY_TABS) return null
    const tabs = value.filter(
      (candidate): candidate is { id: string; name: string; code: string } =>
        !!candidate &&
        typeof candidate === 'object' &&
        typeof (candidate as { id?: unknown }).id === 'string' &&
        (candidate as { id: string }).id.length > 0 &&
        (candidate as { id: string }).id.length <= 128 &&
        typeof (candidate as { name?: unknown }).name === 'string' &&
        typeof (candidate as { code?: unknown }).code === 'string' &&
        (candidate as { code: string }).code.length <= MAX_WORKSPACE_SOURCE_LENGTH,
    )
    if (tabs.length !== value.length || new Set(tabs.map(tab => tab.id)).size !== tabs.length) return null
    const activeId = storage.getItem(WORKSPACE_LEGACY_ACTIVE_TAB_KEY)
    const selected = tabs.find(tab => tab.id === activeId) ?? tabs[0]!
    const rawName = selected.name.trim()
    const fileName =
      rawName.length > 0 && rawName.length <= MAX_WORKSPACE_FILE_NAME_LENGTH
        ? /\.(scad|r)$/i.test(rawName)
          ? rawName
          : `${rawName.slice(0, MAX_WORKSPACE_FILE_NAME_LENGTH - 5)}${isRushFrontend(selected.code) ? '.r' : '.scad'}`
        : 'model.scad'
    return {source: selected.code, fileName}
  } catch {
    return null
  }
}
