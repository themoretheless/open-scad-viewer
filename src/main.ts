import { createApp, markRaw } from 'vue'
import App from './App.vue'
import { EXAMPLES } from './data/examples'
import { storageGet, storageGetEnum, storageKeys, storageRemove, storageSet } from './services/safeStorage'
import { resolveTheme, THEME_SELECTIONS, type ThemeSelection } from './services/themePreferences'
import { IndexedDbWorkspaceRepository } from './services/workspaceIndexedDb'
import { BrowserWorkspacePersistence } from './services/workspacePersistence'
import { readWorkspaceShareHash } from './services/workspaceShare'

// Apply the bounded built-in palette before the asynchronous workspace bootstrap.
const legacyThemeRaw = storageGet('scad-theme')
const legacyTheme: ThemeSelection = legacyThemeRaw === 'dark' || legacyThemeRaw === 'light'
  ? legacyThemeRaw
  : 'system'
const preVersionedTheme = storageGetEnum<ThemeSelection>('scad-theme-selection', THEME_SELECTIONS, legacyTheme)
const initialThemeSelection = storageGetEnum<ThemeSelection>('scad-theme-v1', THEME_SELECTIONS, preVersionedTheme)
const initialTheme = resolveTheme(initialThemeSelection, matchMedia('(prefers-color-scheme: dark)').matches)
document.documentElement.dataset.theme = initialTheme.scheme
document.documentElement.dataset.themePreset = initialThemeSelection
document.documentElement.style.colorScheme = initialTheme.scheme
for (const [token, value] of Object.entries(initialTheme.tokens)) {
  document.documentElement.style.setProperty(token, value)
}

const legacyStorage = {
  getItem: storageGet,
  setItem(key: string, value: string) {
    if (!storageSet(key, value)) throw new Error(`Could not persist ${key}`)
  },
  removeItem(key: string) {
    if (!storageRemove(key)) throw new Error(`Could not remove ${key}`)
  },
  keys: storageKeys,
}
const workspacePersistence = new BrowserWorkspacePersistence(
  new IndexedDbWorkspaceRepository(),
  legacyStorage,
)
const bootstrap = await workspacePersistence.initialize({
  fallbackSource: EXAMPLES.basic,
  importedSource: readWorkspaceShareHash(location.hash),
  importedFileName: 'shared-model.scad',
})

// A valid shared source remains recoverable in the URL until IndexedDB has
// committed it. localStorage-only fallback keeps the URL as a second copy.
const importedCommittedToIndexedDb = bootstrap.imported
  && bootstrap.durable
  && bootstrap.backend === 'indexeddb'
if (importedCommittedToIndexedDb) {
  history.replaceState(null, '', location.pathname + location.search)
}

createApp(App, {
  initialWorkspace: bootstrap.document,
  initialWorkspaceDurable: bootstrap.durable,
  initialSharedImportPending: bootstrap.imported && !importedCommittedToIndexedDb,
  workspacePersistence: markRaw(workspacePersistence),
}).mount('#app')
