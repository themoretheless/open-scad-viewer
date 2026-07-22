/**
 * Static configuration tables extracted from App.vue:
 * editor color themes, viewer presets, and keyboard shortcut presets.
 */

/* ── Editor Themes ── */
export interface EditorTheme {
  id: string
  name: { ru: string; en: string; de: string; zh: string }
  dark: boolean
  vars: Record<string, string>
}

export const EDITOR_THEMES: EditorTheme[] = [
  {
    id: 'default-dark', name: { ru: 'Тёмная', en: 'Default Dark', de: 'Standard Dunkel', zh: '默认深色' }, dark: true,
    vars: {
      '--bg': '#141416', '--surface': '#1e1e22', '--border': '#2e2e34',
      '--text': '#e4e4e8', '--text-dim': '#888', '--accent': '#4a9eff',
      '--hover': '#28282e', '--canvas-bg': '#18181c',
      '--hl-comment': '#6a6a7a', '--hl-keyword': '#5c9eff',
      '--hl-number': '#d19a66', '--hl-string': '#6ec87a',
      '--hl-boolean': '#c678dd', '--hl-special': '#56c8d8',
    },
  },
  {
    id: 'default-light', name: { ru: 'Светлая', en: 'Default Light', de: 'Standard Hell', zh: '默认浅色' }, dark: false,
    vars: {
      '--bg': '#f4f4f6', '--surface': '#fff', '--border': '#d4d4da',
      '--text': '#1a1a1e', '--text-dim': '#777', '--accent': '#2b7de9',
      '--hover': '#eaeaee', '--canvas-bg': '#e8e8ec',
      '--hl-comment': '#999', '--hl-keyword': '#1a6dd4',
      '--hl-number': '#c5600a', '--hl-string': '#2a8c3a',
      '--hl-boolean': '#9040b0', '--hl-special': '#1a8a99',
    },
  },
  {
    id: 'monokai', name: { ru: 'Monokai', en: 'Monokai', de: 'Monokai', zh: 'Monokai' }, dark: true,
    vars: {
      '--bg': '#272822', '--surface': '#2e2e28', '--border': '#49483e',
      '--text': '#f8f8f2', '--text-dim': '#75715e', '--accent': '#a6e22e',
      '--hover': '#3e3d32', '--canvas-bg': '#1e1f1c',
      '--hl-comment': '#75715e', '--hl-keyword': '#f92672',
      '--hl-number': '#ae81ff', '--hl-string': '#e6db74',
      '--hl-boolean': '#ae81ff', '--hl-special': '#66d9ef',
    },
  },
  {
    id: 'solarized', name: { ru: 'Solarized', en: 'Solarized', de: 'Solarized', zh: 'Solarized' }, dark: true,
    vars: {
      '--bg': '#002b36', '--surface': '#073642', '--border': '#586e75',
      '--text': '#839496', '--text-dim': '#657b83', '--accent': '#268bd2',
      '--hover': '#094959', '--canvas-bg': '#001e27',
      '--hl-comment': '#586e75', '--hl-keyword': '#859900',
      '--hl-number': '#d33682', '--hl-string': '#2aa198',
      '--hl-boolean': '#cb4b16', '--hl-special': '#6c71c4',
    },
  },
  {
    id: 'nord', name: { ru: 'Nord', en: 'Nord', de: 'Nord', zh: 'Nord' }, dark: true,
    vars: {
      '--bg': '#2e3440', '--surface': '#3b4252', '--border': '#4c566a',
      '--text': '#d8dee9', '--text-dim': '#8690a3', '--accent': '#88c0d0',
      '--hover': '#434c5e', '--canvas-bg': '#242933',
      '--hl-comment': '#616e88', '--hl-keyword': '#81a1c1',
      '--hl-number': '#b48ead', '--hl-string': '#a3be8c',
      '--hl-boolean': '#d08770', '--hl-special': '#8fbcbb',
    },
  },
  {
    id: 'high-contrast', name: { ru: 'Высокий контраст', en: 'High Contrast', de: 'Hoher Kontrast', zh: '高对比度' }, dark: true,
    vars: {
      '--bg': '#000000', '--surface': '#000000', '--border': '#ffffff',
      '--text': '#ffffff', '--text-dim': '#cfcfcf', '--accent': '#ffff00',
      '--hover': '#1a1a1a', '--canvas-bg': '#000000',
      '--hl-comment': '#9adcff', '--hl-keyword': '#00ffff',
      '--hl-number': '#ffd000', '--hl-string': '#00ff7f',
      '--hl-boolean': '#ff79ff', '--hl-special': '#ffff00',
    },
  },
]

/* ── Export Preset Configurations ── */
export interface ViewerPreset {
  name: string
  themeId: string
  lighting: string
  renderMode: string
  skyPreset: string
  colorGrading: string
  showGrid: boolean
  flatShading: boolean
  ssao: boolean
  outline: boolean
  smoothNormals: boolean
  fontSize: number
  tabSize: number
}

export const BUILT_IN_PRESETS: Record<string, ViewerPreset> = {
  cadPro: {
    name: 'CAD Professional',
    themeId: 'default-dark', lighting: 'studio', renderMode: 'solidEdges',
    skyPreset: 'none', colorGrading: 'none', showGrid: true,
    flatShading: true, ssao: true, outline: true, smoothNormals: false,
    fontSize: 13, tabSize: 4,
  },
  printPreview: {
    name: '3D Print Preview',
    themeId: 'default-light', lighting: 'soft', renderMode: 'solid',
    skyPreset: 'neutral', colorGrading: 'none', showGrid: true,
    flatShading: false, ssao: false, outline: false, smoothNormals: true,
    fontSize: 13, tabSize: 2,
  },
  presentation: {
    name: 'Presentation',
    themeId: 'nord', lighting: 'outdoor', renderMode: 'solid',
    skyPreset: 'clearSky', colorGrading: 'vivid', showGrid: false,
    flatShading: false, ssao: true, outline: true, smoothNormals: true,
    fontSize: 14, tabSize: 4,
  },
}

/* ── Shortcut Presets ── */
export type ShortcutPreset = 'default' | 'vscode' | 'sublime' | 'emacs'

export interface KeyBinding {
  render: string
  format: string
  fullscreen: string
  commandPalette: string
}

export const SHORTCUT_PRESETS: Record<ShortcutPreset, KeyBinding> = {
  default: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  vscode: { render: 'Ctrl+Enter', format: 'Shift+Alt+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  sublime: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  emacs: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Alt+X' },
}

export function matchesBinding(e: KeyboardEvent, binding: string): boolean {
  const parts = binding.toLowerCase().split('+')
  const needCtrl = parts.includes('ctrl')
  const needShift = parts.includes('shift')
  const needAlt = parts.includes('alt')
  const key = parts.filter(p => p !== 'ctrl' && p !== 'shift' && p !== 'alt')[0] || ''
  return (e.ctrlKey || e.metaKey) === needCtrl &&
         e.shiftKey === needShift &&
         e.altKey === needAlt &&
         e.key.toLowerCase() === key
}
