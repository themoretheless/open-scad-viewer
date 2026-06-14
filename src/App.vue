<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { parseOpenSCAD } from './services/openscadParser'
import type { MeshData } from './services/openscadParser'
import { WebGPURenderer } from './services/webgpuRenderer'
import { exportSTL } from './services/stlExport'

const lang = ref<'ru'|'en'>((localStorage.getItem('scad-lang') as any) || 'ru')
const isDark = ref(true)

/* ── Editor Themes ── */
interface EditorTheme {
  id: string
  name: { ru: string; en: string }
  dark: boolean
  vars: Record<string, string>
}

const EDITOR_THEMES: EditorTheme[] = [
  {
    id: 'default-dark', name: { ru: 'Тёмная', en: 'Default Dark' }, dark: true,
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
    id: 'default-light', name: { ru: 'Светлая', en: 'Default Light' }, dark: false,
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
    id: 'monokai', name: { ru: 'Monokai', en: 'Monokai' }, dark: true,
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
    id: 'solarized', name: { ru: 'Solarized', en: 'Solarized' }, dark: true,
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
    id: 'nord', name: { ru: 'Nord', en: 'Nord' }, dark: true,
    vars: {
      '--bg': '#2e3440', '--surface': '#3b4252', '--border': '#4c566a',
      '--text': '#d8dee9', '--text-dim': '#8690a3', '--accent': '#88c0d0',
      '--hover': '#434c5e', '--canvas-bg': '#242933',
      '--hl-comment': '#616e88', '--hl-keyword': '#81a1c1',
      '--hl-number': '#b48ead', '--hl-string': '#a3be8c',
      '--hl-boolean': '#d08770', '--hl-special': '#8fbcbb',
    },
  },
]

const activeThemeId = ref(localStorage.getItem('scad-editor-theme') || 'default-dark')
const showThemeDropdown = ref(false)

const L: Record<string, Record<string, string>> = {
  ru: {
    title: 'OpenSCAD 3D Просмотрщик',
    subtitle: 'Редактор OpenSCAD с рендерингом через WebGPU',
    render: 'Рендер', auto: 'Авто', examples: 'Примеры',
    basic: 'Базовый', csg: 'CSG', house: 'Домик', tower: 'Башня',
    meshes: 'Объектов', triangles: 'Треугольников',
    hint: 'ЛКМ: вращение · ПКМ/Shift: перемещение · колёсико: зум · Ctrl+Enter: рендер',
    noGpu: 'WebGPU не поддерживается. Используйте Chrome 113+ / Edge 113+ / Firefox Nightly.',
    theme: 'Тема',
    diff_note: 'difference() — вычитаемые тела показаны полупрозрачным красным',
    renderTime: 'Рендер',
    top: 'Свр', front: 'Фрн', right: 'Прв', iso: 'Изо', reset: 'Сбр',
    screenshot: 'Скриншот',
    wireframe: 'Каркас', grid: 'Сетка', fullscreen: 'Полный экран',
    autoRotate: 'Вращение',
    errorAtLine: 'Ошибка в строке',
    shortcuts: 'Горячие клавиши',
    shortcutsTitle: 'Горячие клавиши',
    close: 'Закрыть',
    bgColor: 'Фон',
    sc_render: 'Рендер',
    sc_indent: 'Отступ',
    sc_undo: 'Отменить (браузер)',
    sc_find: 'Поиск (браузер)',
    sc_shortcuts: 'Показать горячие клавиши',
    sc_close: 'Закрыть модальное / выйти из полноэкранного',
    open: 'Открыть',
    save: 'Сохранить',
    dropHint: 'Перетащите файл .scad сюда',
    find: 'Найти',
    replace: 'Заменить',
    replaceAll: 'Заменить все',
    next: 'Далее',
    prev: 'Назад',
    matchCount: '{x} из {y}',
    noMatches: 'Нет совпадений',
    fps: 'FPS',
    vertices: 'Вершин',
    size: 'Размер',
    sc_findReplace: 'Найти и заменить',
    sc_findOnly: 'Найти',
    exportStl: 'Экспорт STL',
    share: 'Поделиться',
    copied: 'Скопировано!',
    recent: 'Недавние',
    noRecent: 'Нет недавних файлов',
    newTab: 'Новая вкладка',
    closeTab: 'Закрыть вкладку',
    minimap: 'Миникарта',
    untitled: 'Без имени',
    justNow: 'только что',
    minutesAgo: '{n} мин. назад',
    hoursAgo: '{n} ч. назад',
    daysAgo: '{n} дн. назад',
    persp: 'Перс',
    ortho: 'Орто',
    zoomIn: 'Приблизить',
    zoomOut: 'Отдалить',
    format: 'Формат',
    themeSelector: 'Тема оформления',
  },
  en: {
    title: 'OpenSCAD 3D Viewer',
    subtitle: 'OpenSCAD editor with WebGPU rendering',
    render: 'Render', auto: 'Auto', examples: 'Examples',
    basic: 'Basic', csg: 'CSG', house: 'House', tower: 'Tower',
    meshes: 'Meshes', triangles: 'Triangles',
    hint: 'LMB: rotate · RMB/Shift: pan · wheel: zoom · Ctrl+Enter: render',
    noGpu: 'WebGPU not supported. Use Chrome 113+ / Edge 113+ / Firefox Nightly.',
    theme: 'Theme',
    diff_note: 'difference() — subtracted bodies shown as translucent red',
    renderTime: 'Render',
    top: 'Top', front: 'Front', right: 'Right', iso: 'Iso', reset: 'Reset',
    screenshot: 'Screenshot',
    wireframe: 'Wire', grid: 'Grid', fullscreen: 'Fullscreen',
    autoRotate: 'Rotate',
    errorAtLine: 'Error at line',
    shortcuts: 'Shortcuts',
    shortcutsTitle: 'Keyboard Shortcuts',
    close: 'Close',
    bgColor: 'BG',
    sc_render: 'Render',
    sc_indent: 'Indent',
    sc_undo: 'Undo (browser native)',
    sc_find: 'Find (browser native)',
    sc_shortcuts: 'Show shortcuts',
    sc_close: 'Close modal / exit fullscreen',
    open: 'Open',
    save: 'Save',
    dropHint: 'Drop .scad file here',
    find: 'Find',
    replace: 'Replace',
    replaceAll: 'Replace All',
    next: 'Next',
    prev: 'Prev',
    matchCount: '{x} of {y}',
    noMatches: 'No matches',
    fps: 'FPS',
    vertices: 'Vertices',
    size: 'Size',
    sc_findReplace: 'Find & Replace',
    sc_findOnly: 'Find',
    exportStl: 'Export STL',
    share: 'Share',
    copied: 'Copied!',
    recent: 'Recent',
    noRecent: 'No recent files',
    newTab: 'New tab',
    closeTab: 'Close tab',
    minimap: 'Minimap',
    untitled: 'Untitled',
    justNow: 'just now',
    minutesAgo: '{n}m ago',
    hoursAgo: '{n}h ago',
    daysAgo: '{n}d ago',
    persp: 'Persp',
    ortho: 'Ortho',
    zoomIn: 'Zoom In',
    zoomOut: 'Zoom Out',
    format: 'Format',
    themeSelector: 'Editor Theme',
  },
}

const t = (k: string) => L[lang.value]?.[k] ?? k
const toggleLang = () => { lang.value = lang.value === 'ru' ? 'en' : 'ru'; localStorage.setItem('scad-lang', lang.value) }

onMounted(() => {
  const savedThemeId = localStorage.getItem('scad-editor-theme')
  if (savedThemeId) {
    activeThemeId.value = savedThemeId
  } else {
    const saved = localStorage.getItem('scad-theme')
    activeThemeId.value = saved === 'light' ? 'default-light' : 'default-dark'
  }
  applyTheme()
})

function applyTheme() {
  const theme = EDITOR_THEMES.find(th => th.id === activeThemeId.value) || EDITOR_THEMES[0]
  isDark.value = theme.dark
  document.documentElement.setAttribute('data-theme', theme.dark ? '' : 'light')
  // Apply CSS variables from theme
  const root = document.documentElement
  for (const [key, val] of Object.entries(theme.vars)) {
    root.style.setProperty(key, val)
  }
  localStorage.setItem('scad-editor-theme', theme.id)
  localStorage.setItem('scad-theme', theme.dark ? 'dark' : 'light')
}

function selectTheme(id: string) {
  activeThemeId.value = id
  applyTheme()
  showThemeDropdown.value = false
}

function toggleTheme() {
  // Simple toggle: switch between first dark and first light
  const curTheme = EDITOR_THEMES.find(th => th.id === activeThemeId.value) || EDITOR_THEMES[0]
  if (curTheme.dark) {
    activeThemeId.value = 'default-light'
  } else {
    activeThemeId.value = 'default-dark'
  }
  applyTheme()
}

/* ── Multi-tab Editor ── */

interface EditorTab {
  id: string
  name: string
  code: string
}

function generateTabId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
}

function loadTabsFromStorage(): EditorTab[] {
  try {
    const raw = localStorage.getItem('scad-tabs')
    if (raw) {
      const parsed = JSON.parse(raw) as EditorTab[]
      if (Array.isArray(parsed) && parsed.length > 0) return parsed
    }
  } catch { /* ignore */ }
  // Migrate from old single-code storage
  const oldCode = localStorage.getItem('scad-code')
  return [{ id: generateTabId(), name: t('untitled') + ' 1', code: oldCode || EXAMPLES.basic }]
}

const tabs = ref<EditorTab[]>(loadTabsFromStorage())
const activeTabId = ref(localStorage.getItem('scad-active-tab') || tabs.value[0].id)

// Ensure activeTabId points to a valid tab
if (!tabs.value.find(tb => tb.id === activeTabId.value)) {
  activeTabId.value = tabs.value[0].id
}

const activeTab = computed(() => tabs.value.find(tb => tb.id === activeTabId.value) || tabs.value[0])

const code = computed({
  get: () => activeTab.value.code,
  set: (v: string) => { activeTab.value.code = v },
})

function saveTabs() {
  localStorage.setItem('scad-tabs', JSON.stringify(tabs.value))
  localStorage.setItem('scad-active-tab', activeTabId.value)
}

function switchTab(id: string) {
  activeTabId.value = id
  saveTabs()
  nextTick(() => {
    syncScroll()
    renderMinimap()
  })
}

function addTab() {
  const idx = tabs.value.length + 1
  const tab: EditorTab = { id: generateTabId(), name: `${t('untitled')} ${idx}`, code: '' }
  tabs.value.push(tab)
  activeTabId.value = tab.id
  saveTabs()
}

function closeTab(id: string) {
  if (tabs.value.length <= 1) return
  const idx = tabs.value.findIndex(tb => tb.id === id)
  if (idx === -1) return
  tabs.value.splice(idx, 1)
  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id
  }
  saveTabs()
}

const editingTabId = ref<string | null>(null)
const editingTabName = ref('')
const tabNameInputRef = ref<HTMLInputElement | null>(null)

function startRenameTab(id: string) {
  const tab = tabs.value.find(tb => tb.id === id)
  if (!tab) return
  editingTabId.value = id
  editingTabName.value = tab.name
  nextTick(() => tabNameInputRef.value?.select())
}

function finishRenameTab() {
  if (!editingTabId.value) return
  const tab = tabs.value.find(tb => tb.id === editingTabId.value)
  if (tab && editingTabName.value.trim()) {
    tab.name = editingTabName.value.trim()
  }
  editingTabId.value = null
  saveTabs()
}

function cancelRenameTab() {
  editingTabId.value = null
}

/* ── Editor + Renderer ── */

const canvasRef = ref<HTMLCanvasElement | null>(null)
const error = ref('')
const errorLine = ref(-1)
const meshCount = ref(0)
const triCount = ref(0)
const gpuOk = ref(true)
const autoRender = ref(true)
const renderTime = ref(0)
const isFullscreen = ref(false)
const showWireframe = ref(false)
const showGrid = ref(true)
const isAutoRotate = ref(false)
const isOrthographic = ref(false)

let renderer: WebGPURenderer | null = null
let debounce: ReturnType<typeof setTimeout> | null = null
let lastParsedMeshes: MeshData[] = []

/* ── Drag & drop state ── */
const isDragOver = ref(false)
let dragCounter = 0

/* ── Stats polling ── */
const fpsVal = ref(0)
const vertexCount = ref(0)
const boundsSize = ref<[number, number, number]>([0, 0, 0])
let statsInterval: ReturnType<typeof setInterval> | null = null

/* ── Find & Replace ── */
const showFind = ref(false)
const showReplace = ref(false)
const findText = ref('')
const replaceText = ref('')
const findMatchIndex = ref(-1)
const findMatches = ref<{ start: number; end: number }[]>([])
const findInputRef = ref<HTMLInputElement | null>(null)

/* ── Shortcuts modal ── */
const showShortcuts = ref(false)

/* ── Viewport background color ── */
const bgColors = [
  { name: 'Dark',  hex: '#18181c', r: 0.09, g: 0.09, b: 0.11 },
  { name: 'Light', hex: '#e8e8ec', r: 0.91, g: 0.91, b: 0.93 },
  { name: 'Blue',  hex: '#1a2332', r: 0.10, g: 0.14, b: 0.20 },
  { name: 'Green', hex: '#1a2a1e', r: 0.10, g: 0.16, b: 0.12 },
]
const activeBg = ref(0)

function setBgColor(index: number) {
  activeBg.value = index
  const c = bgColors[index]
  renderer?.setClearColor(c.r, c.g, c.b)
}

/* ── STL Export ── */
function doExportSTL() {
  if (!lastParsedMeshes.length) return
  const tabName = activeTab.value.name.replace(/[^a-zA-Z0-9_-]/g, '_') || 'model'
  exportSTL(lastParsedMeshes, `${tabName}.stl`)
}

/* ── Share Link ── */
const showCopied = ref(false)

function shareLink() {
  const encoded = btoa(encodeURIComponent(code.value))
  const url = window.location.origin + window.location.pathname + '#code=' + encoded
  navigator.clipboard.writeText(url).then(() => {
    showCopied.value = true
    setTimeout(() => { showCopied.value = false }, 1500)
  }).catch(() => {
    // Fallback: manual copy
    const ta = document.createElement('textarea')
    ta.value = url
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    showCopied.value = true
    setTimeout(() => { showCopied.value = false }, 1500)
  })
}

function loadFromHash() {
  const hash = window.location.hash
  if (hash.startsWith('#code=')) {
    try {
      const encoded = hash.slice(6)
      const decoded = decodeURIComponent(atob(encoded))
      code.value = decoded
      // Clear hash after loading
      history.replaceState(null, '', window.location.pathname)
    } catch { /* ignore invalid hash */ }
  }
}

/* ── Recent Files History ── */
interface RecentEntry {
  name: string
  code: string
  timestamp: number
}

const recentFiles = ref<RecentEntry[]>([])
const showRecent = ref(false)

function loadRecentFiles() {
  try {
    const raw = localStorage.getItem('scad-recent')
    if (raw) {
      recentFiles.value = JSON.parse(raw) as RecentEntry[]
    }
  } catch { /* ignore */ }
}

function saveRecentFiles() {
  localStorage.setItem('scad-recent', JSON.stringify(recentFiles.value))
}

function addToRecent(name: string, codeStr: string) {
  // Remove duplicate
  recentFiles.value = recentFiles.value.filter(r => r.code !== codeStr)
  // Add to front
  recentFiles.value.unshift({ name, code: codeStr, timestamp: Date.now() })
  // Keep max 10
  if (recentFiles.value.length > 10) recentFiles.value = recentFiles.value.slice(0, 10)
  saveRecentFiles()
}

function loadRecent(entry: RecentEntry) {
  code.value = entry.code
  showRecent.value = false
}

function formatRelativeTime(ts: number): string {
  const diff = Date.now() - ts
  const mins = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)
  if (mins < 1) return t('justNow')
  if (mins < 60) return t('minutesAgo').replace('{n}', String(mins))
  if (hours < 24) return t('hoursAgo').replace('{n}', String(hours))
  return t('daysAgo').replace('{n}', String(days))
}

/* ── Code Minimap ── */
const showMinimap = ref(false)
const minimapCanvasRef = ref<HTMLCanvasElement | null>(null)
let minimapDebounce: ReturnType<typeof setTimeout> | null = null
const minimapDragging = ref(false)

const MINIMAP_KEYWORDS = new Set([
  'cube','sphere','cylinder','translate','rotate','scale','color',
  'difference','union','intersection','mirror','module','function',
  'if','else','for','let','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','each','echo','assert',
])

function getLineType(line: string): string {
  const trimmed = line.trim()
  if (!trimmed) return 'empty'
  if (trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('*')) return 'comment'
  if (trimmed.includes('"')) return 'string'
  // Check for keyword at start
  const wordMatch = trimmed.match(/^([a-zA-Z_]\w*)/)
  if (wordMatch && MINIMAP_KEYWORDS.has(wordMatch[1])) return 'keyword'
  // Check for numbers
  if (/^\d/.test(trimmed) || /[=:]\s*\d/.test(trimmed)) return 'number'
  return 'code'
}

function renderMinimap() {
  const canvas = minimapCanvasRef.value
  if (!canvas || !showMinimap.value) return
  const ctx2d = canvas.getContext('2d')
  if (!ctx2d) return

  const lines = code.value.split('\n')
  const dpr = window.devicePixelRatio || 1
  const lineH = 2 * dpr
  const canvasW = 60 * dpr
  const canvasH = canvas.clientHeight * dpr
  canvas.width = canvasW
  canvas.height = canvasH

  ctx2d.clearRect(0, 0, canvasW, canvasH)

  // Background
  ctx2d.fillStyle = isDark.value ? 'rgba(20,20,24,0.6)' : 'rgba(240,240,244,0.6)'
  ctx2d.fillRect(0, 0, canvasW, canvasH)

  const colorMap: Record<string, string> = {
    keyword: '#5c9eff',
    comment: '#6a6a7a',
    string: '#6ec87a',
    number: '#d19a66',
    code: isDark.value ? 'rgba(228,228,232,0.4)' : 'rgba(26,26,30,0.4)',
    empty: 'transparent',
  }

  // Draw lines
  const maxVisibleLines = Math.floor(canvasH / lineH)
  const totalLines = lines.length
  const scale = totalLines > maxVisibleLines ? maxVisibleLines / totalLines : 1

  for (let i = 0; i < totalLines; i++) {
    const lineType = getLineType(lines[i])
    if (lineType === 'empty') continue
    ctx2d.fillStyle = colorMap[lineType] || colorMap.code
    const y = i * lineH * scale
    const lineLen = Math.min(lines[i].length, 50)
    const w = (lineLen / 50) * canvasW * 0.85
    ctx2d.fillRect(2 * dpr, y, Math.max(w, 2 * dpr), Math.max(lineH * scale, 1))
  }

  // Draw viewport indicator
  const textarea = textareaRef.value
  if (textarea) {
    const lineHeight = parseFloat(getComputedStyle(textarea).lineHeight) || 20
    const scrollTop = textarea.scrollTop
    const viewH = textarea.clientHeight
    const firstLine = Math.floor(scrollTop / lineHeight)
    const visibleLines = Math.ceil(viewH / lineHeight)

    const vpY = firstLine * lineH * scale
    const vpH = Math.max(visibleLines * lineH * scale, 10 * dpr)

    ctx2d.fillStyle = isDark.value ? 'rgba(74,158,255,0.12)' : 'rgba(43,125,233,0.12)'
    ctx2d.fillRect(0, vpY, canvasW, vpH)
    ctx2d.strokeStyle = isDark.value ? 'rgba(74,158,255,0.35)' : 'rgba(43,125,233,0.35)'
    ctx2d.lineWidth = dpr
    ctx2d.strokeRect(0.5, vpY + 0.5, canvasW - 1, vpH - 1)
  }
}

function minimapScrollTo(e: MouseEvent) {
  const canvas = minimapCanvasRef.value
  const textarea = textareaRef.value
  if (!canvas || !textarea) return

  const rect = canvas.getBoundingClientRect()
  const y = e.clientY - rect.top
  const ratio = y / rect.height
  const lines = code.value.split('\n').length
  const lineHeight = parseFloat(getComputedStyle(textarea).lineHeight) || 20
  const totalH = lines * lineHeight
  textarea.scrollTop = ratio * totalH - textarea.clientHeight / 2
  syncScroll()
  renderMinimap()
}

function onMinimapMouseDown(e: MouseEvent) {
  minimapDragging.value = true
  minimapScrollTo(e)
  document.addEventListener('mousemove', onMinimapMouseMove)
  document.addEventListener('mouseup', onMinimapMouseUp)
}

function onMinimapMouseMove(e: MouseEvent) {
  if (minimapDragging.value) minimapScrollTo(e)
}

function onMinimapMouseUp() {
  minimapDragging.value = false
  document.removeEventListener('mousemove', onMinimapMouseMove)
  document.removeEventListener('mouseup', onMinimapMouseUp)
}

function toggleMinimap() {
  showMinimap.value = !showMinimap.value
  if (showMinimap.value) nextTick(renderMinimap)
}

/* ── Autocomplete ── */
const AUTOCOMPLETE_KEYWORDS = [
  'cube','sphere','cylinder','translate','rotate','scale','mirror','color',
  'difference','union','intersection','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','module','function','for','if','else',
  'let','each','echo','assert','$fn','$fa','$fs','true','false','undef','PI',
]

const acVisible = ref(false)
const acItems = ref<string[]>([])
const acIndex = ref(0)
const acTop = ref(0)
const acLeft = ref(0)
let acPrefix = ''
let acStart = 0

function getWordAtCursor(el: HTMLTextAreaElement): { word: string; start: number } {
  const pos = el.selectionStart
  const text = el.value
  let start = pos
  while (start > 0 && /[a-zA-Z0-9_$]/.test(text[start - 1])) start--
  return { word: text.slice(start, pos), start }
}

function updateAutocomplete() {
  const el = textareaRef.value
  if (!el) return
  const { word, start } = getWordAtCursor(el)
  if (word.length < 1) { acVisible.value = false; return }

  const lower = word.toLowerCase()
  const matches = AUTOCOMPLETE_KEYWORDS.filter(k => k.toLowerCase().startsWith(lower) && k.toLowerCase() !== lower)
  if (!matches.length) { acVisible.value = false; return }

  acPrefix = word
  acStart = start
  acItems.value = matches.slice(0, 6)
  acIndex.value = 0

  // Position the popup near the cursor
  const pos = getCaretCoordinates(el)
  acTop.value = pos.top
  acLeft.value = pos.left
  acVisible.value = true
}

function getCaretCoordinates(el: HTMLTextAreaElement): { top: number; left: number } {
  // Create a mirror div to measure cursor position
  const div = document.createElement('div')
  const style = window.getComputedStyle(el)
  const props = [
    'fontFamily','fontSize','fontWeight','letterSpacing','lineHeight',
    'paddingTop','paddingLeft','paddingRight','paddingBottom',
    'borderTopWidth','borderLeftWidth','borderRightWidth','borderBottomWidth',
    'whiteSpace','wordWrap','tabSize',
  ]
  div.style.position = 'absolute'
  div.style.visibility = 'hidden'
  div.style.whiteSpace = 'pre'
  div.style.overflow = 'hidden'
  for (const p of props) {
    (div.style as any)[p] = style.getPropertyValue(p.replace(/([A-Z])/g, '-$1').toLowerCase())
  }
  div.style.width = el.clientWidth + 'px'
  div.style.height = 'auto'

  const text = el.value.substring(0, el.selectionStart)
  const textNode = document.createTextNode(text)
  const span = document.createElement('span')
  span.textContent = '|'
  div.appendChild(textNode)
  div.appendChild(span)
  document.body.appendChild(div)

  const rect = el.getBoundingClientRect()
  const codeArea = el.closest('.code-area')
  const codeAreaRect = codeArea ? codeArea.getBoundingClientRect() : rect
  const top = span.offsetTop - el.scrollTop + rect.top - codeAreaRect.top + parseInt(style.lineHeight || '20')
  const left = span.offsetLeft - el.scrollLeft + rect.left - codeAreaRect.left

  document.body.removeChild(div)
  return { top: Math.max(0, top), left: Math.max(0, left) }
}

function acceptAutocomplete() {
  const el = textareaRef.value
  if (!el || !acVisible.value || !acItems.value.length) return
  const item = acItems.value[acIndex.value]
  const before = code.value.substring(0, acStart)
  const after = code.value.substring(acStart + acPrefix.length)
  code.value = before + item + after
  acVisible.value = false
  nextTick(() => {
    const pos = acStart + item.length
    el.selectionStart = el.selectionEnd = pos
    el.focus()
  })
}

function dismissAutocomplete() {
  acVisible.value = false
}

/* ── Bracket matching ── */
const bracketMatchA = ref(-1)
const bracketMatchB = ref(-1)

const BRACKET_PAIRS: Record<string, string> = {
  '(': ')', ')': '(',
  '[': ']', ']': '[',
  '{': '}', '}': '{',
}
const OPEN_BRACKETS = new Set(['(', '[', '{'])
const CLOSE_BRACKETS = new Set([')', ']', '}'])

function findMatchingBracket(src: string, pos: number): number {
  const ch = src[pos]
  if (!ch || !BRACKET_PAIRS[ch]) return -1

  if (OPEN_BRACKETS.has(ch)) {
    // Search forward
    const target = BRACKET_PAIRS[ch]
    let depth = 1
    for (let i = pos + 1; i < src.length; i++) {
      if (src[i] === ch) depth++
      else if (src[i] === target) { depth--; if (depth === 0) return i }
    }
  } else if (CLOSE_BRACKETS.has(ch)) {
    // Search backward
    const target = BRACKET_PAIRS[ch]
    let depth = 1
    for (let i = pos - 1; i >= 0; i--) {
      if (src[i] === ch) depth++
      else if (src[i] === target) { depth--; if (depth === 0) return i }
    }
  }
  return -1
}

function updateBracketMatch() {
  const el = textareaRef.value
  if (!el) { bracketMatchA.value = -1; bracketMatchB.value = -1; return }
  const pos = el.selectionStart
  const src = code.value

  // Check character at cursor position and cursor-1
  let matchPos = -1
  let bracketPos = -1

  if (pos < src.length && BRACKET_PAIRS[src[pos]]) {
    bracketPos = pos
    matchPos = findMatchingBracket(src, pos)
  }
  if (matchPos === -1 && pos > 0 && BRACKET_PAIRS[src[pos - 1]]) {
    bracketPos = pos - 1
    matchPos = findMatchingBracket(src, pos - 1)
  }

  bracketMatchA.value = bracketPos !== -1 && matchPos !== -1 ? bracketPos : -1
  bracketMatchB.value = matchPos
}

/* ── Resizable split pane ── */
const editorWidth = ref(parseInt(localStorage.getItem('scad-editor-width') || '420'))
const isDraggingDivider = ref(false)

function onDividerDown(e: MouseEvent) {
  e.preventDefault()
  isDraggingDivider.value = true
  document.addEventListener('mousemove', onDividerMove)
  document.addEventListener('mouseup', onDividerUp)
}
function onDividerMove(e: MouseEvent) {
  if (!isDraggingDivider.value) return
  const newW = Math.max(240, Math.min(window.innerWidth * 0.6, e.clientX))
  editorWidth.value = newW
}
function onDividerUp() {
  isDraggingDivider.value = false
  localStorage.setItem('scad-editor-width', String(editorWidth.value | 0))
  document.removeEventListener('mousemove', onDividerMove)
  document.removeEventListener('mouseup', onDividerUp)
}

/* ── Syntax highlighting with bracket matching ── */
const KEYWORDS = new Set([
  'cube','sphere','cylinder','translate','rotate','scale','color',
  'difference','union','intersection','mirror','module','function',
  'if','else','for','let','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','each','echo','assert',
])
const BOOLEANS = new Set(['true','false','undef'])
const SPECIALS = new Set(['$fn','$fa','$fs'])

function highlightCode(src: string, bmA: number, bmB: number, fMatches: { start: number; end: number }[], fActiveIdx: number): string {
  const esc = (s: string) => s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')

  // Build a set of find-match ranges for quick lookup
  const findSet = new Map<number, { end: number; active: boolean }>()
  for (let fi = 0; fi < fMatches.length; fi++) {
    findSet.set(fMatches[fi].start, { end: fMatches[fi].end, active: fi === fActiveIdx })
  }

  const result: string[] = []
  let i = 0
  const len = src.length

  const isBracketMatch = (pos: number) => pos === bmA || pos === bmB

  // Track if we are inside a find-match highlight span
  let inFindMatch = false
  let findMatchEnd = 0
  let findMatchActive = false

  while (i < len) {
    // Check if we enter a find-match range
    const fm = findSet.get(i)
    if (fm && !inFindMatch) {
      inFindMatch = true
      findMatchEnd = fm.end
      findMatchActive = fm.active
      result.push(`<span class="${findMatchActive ? 'find-match-active' : 'find-match'}">`)
    }

    // Close find-match if we passed the end
    if (inFindMatch && i >= findMatchEnd) {
      result.push('</span>')
      inFindMatch = false
    }

    // Block comments
    if (src[i] === '/' && src[i+1] === '*') {
      let end = src.indexOf('*/', i + 2)
      if (end === -1) end = len - 2
      const cmt = src.slice(i, end + 2)
      result.push(`<span class="hl-comment">${esc(cmt)}</span>`)
      i = end + 2
      continue
    }
    // Line comments
    if (src[i] === '/' && src[i+1] === '/') {
      let end = src.indexOf('\n', i)
      if (end === -1) end = len
      const cmt = src.slice(i, end)
      result.push(`<span class="hl-comment">${esc(cmt)}</span>`)
      i = end
      continue
    }
    // Strings
    if (src[i] === '"') {
      let j = i + 1
      while (j < len && src[j] !== '"') {
        if (src[j] === '\\') j++
        j++
      }
      const str = src.slice(i, j + 1)
      result.push(`<span class="hl-string">${esc(str)}</span>`)
      i = j + 1
      continue
    }
    // Bracket match highlight
    if (isBracketMatch(i)) {
      result.push(`<span class="bracket-match">${esc(src[i])}</span>`)
      i++
      continue
    }
    // Special variables ($fn, $fa, $fs)
    if (src[i] === '$') {
      let j = i + 1
      while (j < len && /[a-zA-Z0-9_]/.test(src[j])) j++
      const word = src.slice(i, j)
      if (SPECIALS.has(word)) {
        result.push(`<span class="hl-special">${esc(word)}</span>`)
      } else {
        result.push(esc(word))
      }
      i = j
      continue
    }
    // Numbers
    if (/[0-9]/.test(src[i]) || (src[i] === '.' && i + 1 < len && /[0-9]/.test(src[i+1]))) {
      let j = i
      while (j < len && /[0-9.]/.test(src[j])) j++
      if (j < len && (src[j] === 'e' || src[j] === 'E')) {
        j++
        if (j < len && (src[j] === '+' || src[j] === '-')) j++
        while (j < len && /[0-9]/.test(src[j])) j++
      }
      result.push(`<span class="hl-number">${esc(src.slice(i, j))}</span>`)
      i = j
      continue
    }
    // Identifiers / keywords
    if (/[a-zA-Z_]/.test(src[i])) {
      let j = i
      while (j < len && /[a-zA-Z0-9_]/.test(src[j])) j++
      const word = src.slice(i, j)
      if (KEYWORDS.has(word)) {
        result.push(`<span class="hl-keyword">${esc(word)}</span>`)
      } else if (BOOLEANS.has(word)) {
        result.push(`<span class="hl-boolean">${esc(word)}</span>`)
      } else {
        result.push(esc(word))
      }
      i = j
      continue
    }
    // Newlines (preserve them)
    if (src[i] === '\n') {
      result.push('\n')
      i++
      continue
    }
    // Everything else
    result.push(esc(src[i]))
    i++
  }
  // Close any dangling find-match span
  if (inFindMatch) result.push('</span>')
  return result.join('') + '\n'
}

const highlightedCode = computed(() => highlightCode(code.value, bracketMatchA.value, bracketMatchB.value, findMatches.value, findMatchIndex.value))

/* ── Line numbers ── */
const lineCount = computed(() => code.value.split('\n').length)
const lineNumbers = computed(() => {
  const n = lineCount.value
  const errL = errorLine.value
  const nums: string[] = []
  for (let i = 1; i <= n; i++) {
    if (i === errL) {
      nums.push(`<span class="line-error">${i}</span>`)
    } else {
      nums.push(String(i))
    }
  }
  return nums.join('\n')
})

/* ── Sync scroll ── */
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const highlightRef = ref<HTMLElement | null>(null)
const lineNumRef = ref<HTMLElement | null>(null)

function syncScroll() {
  const ta = textareaRef.value
  if (!ta) return
  if (highlightRef.value) {
    highlightRef.value.scrollTop = ta.scrollTop
    highlightRef.value.scrollLeft = ta.scrollLeft
  }
  if (lineNumRef.value) {
    lineNumRef.value.scrollTop = ta.scrollTop
  }
  // Debounce minimap update on scroll
  if (showMinimap.value) {
    if (minimapDebounce) clearTimeout(minimapDebounce)
    minimapDebounce = setTimeout(renderMinimap, 50)
  }
}

/* ── File Import / Export ── */
function openFile() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.scad'
  input.onchange = () => {
    const file = input.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => {
      const content = reader.result as string
      code.value = content
      addToRecent(file.name.replace(/\.scad$/, ''), content)
    }
    reader.readAsText(file)
  }
  input.click()
}

function saveFile() {
  const blob = new Blob([code.value], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = (activeTab.value.name || 'model') + '.scad'
  a.click()
  URL.revokeObjectURL(url)
  addToRecent(activeTab.value.name, code.value)
}

function onEditorDragEnter(e: DragEvent) {
  e.preventDefault()
  dragCounter++
  isDragOver.value = true
}
function onEditorDragOver(e: DragEvent) {
  e.preventDefault()
}
function onEditorDragLeave(e: DragEvent) {
  e.preventDefault()
  dragCounter--
  if (dragCounter <= 0) { dragCounter = 0; isDragOver.value = false }
}
function onEditorDrop(e: DragEvent) {
  e.preventDefault()
  dragCounter = 0
  isDragOver.value = false
  const file = e.dataTransfer?.files?.[0]
  if (!file || !file.name.endsWith('.scad')) return
  const reader = new FileReader()
  reader.onload = () => {
    const content = reader.result as string
    code.value = content
    addToRecent(file.name.replace(/\.scad$/, ''), content)
  }
  reader.readAsText(file)
}

/* ── Find & Replace ── */
function openFindReplace(replaceMode: boolean) {
  showFind.value = true
  showReplace.value = replaceMode
  nextTick(() => findInputRef.value?.focus())
  updateFindMatches()
}

function closeFindReplace() {
  showFind.value = false
  showReplace.value = false
  findText.value = ''
  replaceText.value = ''
  findMatches.value = []
  findMatchIndex.value = -1
}

function updateFindMatches() {
  if (!findText.value) { findMatches.value = []; findMatchIndex.value = -1; return }
  const src = code.value
  const needle = findText.value
  const matches: { start: number; end: number }[] = []
  let pos = 0
  const lowerSrc = src.toLowerCase()
  const lowerNeedle = needle.toLowerCase()
  while (pos < src.length) {
    const idx = lowerSrc.indexOf(lowerNeedle, pos)
    if (idx === -1) break
    matches.push({ start: idx, end: idx + needle.length })
    pos = idx + 1
  }
  findMatches.value = matches
  if (matches.length > 0) {
    if (findMatchIndex.value < 0 || findMatchIndex.value >= matches.length) findMatchIndex.value = 0
  } else {
    findMatchIndex.value = -1
  }
}

watch(findText, updateFindMatches)

function findNext() {
  if (!findMatches.value.length) return
  findMatchIndex.value = (findMatchIndex.value + 1) % findMatches.value.length
  scrollToMatch()
}

function findPrev() {
  if (!findMatches.value.length) return
  findMatchIndex.value = (findMatchIndex.value - 1 + findMatches.value.length) % findMatches.value.length
  scrollToMatch()
}

function scrollToMatch() {
  const el = textareaRef.value
  if (!el || findMatchIndex.value < 0) return
  const match = findMatches.value[findMatchIndex.value]
  if (!match) return
  el.focus()
  el.setSelectionRange(match.start, match.end)
  // Scroll the textarea so the match is visible
  const textBefore = code.value.substring(0, match.start)
  const lineNum = textBefore.split('\n').length
  const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20
  el.scrollTop = Math.max(0, (lineNum - 3) * lineHeight)
}

function doReplace() {
  if (findMatchIndex.value < 0 || !findMatches.value.length) return
  const match = findMatches.value[findMatchIndex.value]
  code.value = code.value.substring(0, match.start) + replaceText.value + code.value.substring(match.end)
  updateFindMatches()
}

function doReplaceAll() {
  if (!findText.value || !findMatches.value.length) return
  const lowerSrc = code.value.toLowerCase()
  const lowerNeedle = findText.value.toLowerCase()
  let result = ''
  let pos = 0
  const src = code.value
  const needle = findText.value
  while (pos < src.length) {
    const idx = lowerSrc.indexOf(lowerNeedle, pos)
    if (idx === -1) { result += src.substring(pos); break }
    result += src.substring(pos, idx) + replaceText.value
    pos = idx + needle.length
  }
  code.value = result
  updateFindMatches()
}

function handleFindKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') { e.preventDefault(); findNext() }
  if (e.key === 'Escape') { e.preventDefault(); closeFindReplace() }
}

function handleReplaceKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') { e.preventDefault(); closeFindReplace() }
}

/* ── View presets (animated) ── */
function setView(name: string) {
  if (!renderer) return
  switch (name) {
    case 'top':
      renderer.animateTo(0, Math.PI / 2)
      break
    case 'front':
      renderer.animateTo(0, 0)
      break
    case 'right':
      renderer.animateTo(Math.PI / 2, 0)
      break
    case 'iso':
      renderer.animateTo(0.6, 0.4)
      break
    case 'reset':
      renderer.autoFitAll()
      renderer.animateTo(0.6, 0.4)
      break
  }
}

/* ── Screenshot ── */
function takeScreenshot() {
  if (!renderer) return
  renderer.screenshot()
}

/* ── Toggle wireframe ── */
function toggleWireframe() {
  if (!renderer) return
  showWireframe.value = renderer.toggleWireframe()
}

/* ── Toggle grid ── */
function toggleGrid() {
  if (!renderer) return
  showGrid.value = renderer.toggleGrid()
}

/* ── Toggle fullscreen canvas ── */
function toggleFullscreen() {
  isFullscreen.value = !isFullscreen.value
}

/* ── Toggle auto-rotate ── */
function toggleAutoRotate() {
  if (!renderer) return
  isAutoRotate.value = renderer.toggleAutoRotate()
}

/* ── Orthographic toggle ── */
function toggleProjection() {
  if (!renderer) return
  isOrthographic.value = renderer.toggleProjection()
}

/* ── Zoom controls ── */
function doZoomIn() {
  if (!renderer) return
  renderer.zoomIn()
}
function doZoomOut() {
  if (!renderer) return
  renderer.zoomOut()
}

/* ── Code Formatter / Auto-indent ── */
function formatCode() {
  const src = code.value
  const lines = src.split('\n')
  const result: string[] = []
  let indent = 0
  const INDENT = '    '

  for (const raw of lines) {
    const trimmed = raw.trim()
    if (!trimmed) {
      result.push('')
      continue
    }

    // Count closing braces at start of line to decrease indent first
    let leadingCloses = 0
    for (const ch of trimmed) {
      if (ch === '}' || ch === ')' || ch === ']') leadingCloses++
      else break
    }
    indent = Math.max(0, indent - leadingCloses)

    result.push(INDENT.repeat(indent) + trimmed)

    // Count net brace changes for next line
    let opens = 0
    let closes = 0
    let inStr = false
    let inLineComment = false
    for (let i = 0; i < trimmed.length; i++) {
      const ch = trimmed[i]
      if (inLineComment) break
      if (ch === '"' && (i === 0 || trimmed[i - 1] !== '\\')) { inStr = !inStr; continue }
      if (inStr) continue
      if (ch === '/' && i + 1 < trimmed.length && trimmed[i + 1] === '/') { inLineComment = true; break }
      if (ch === '{' || ch === '(' || ch === '[') opens++
      if (ch === '}' || ch === ')' || ch === ']') closes++
    }
    // We already subtracted leadingCloses from indent above,
    // so add all opens and subtract remaining closes (closes minus leadingCloses)
    indent += opens - (closes - leadingCloses)
    indent = Math.max(0, indent)
  }

  code.value = result.join('\n')
}

/* ── Axis Labels ── */
const axisLabelX = ref({ x: 0, y: 0, visible: false })
const axisLabelY = ref({ x: 0, y: 0, visible: false })
const axisLabelZ = ref({ x: 0, y: 0, visible: false })
let axisLabelRAF = 0

function updateAxisLabels() {
  if (!renderer) return
  const len = 210 // axis line length (matches grid gs=200, a bit past end)
  const px = renderer.getScreenPosition(len, 0, 0)
  const py = renderer.getScreenPosition(0, len, 0)
  const pz = renderer.getScreenPosition(0, 0, len)
  axisLabelX.value = { x: px.x, y: px.y, visible: !px.behind }
  axisLabelY.value = { x: py.x, y: py.y, visible: !py.behind }
  axisLabelZ.value = { x: pz.x, y: pz.y, visible: !pz.behind }
  axisLabelRAF = requestAnimationFrame(updateAxisLabels)
}

/* ── Close recent dropdown on outside click ── */
function onDocClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (showRecent.value && !target.closest('.recent-wrapper')) {
    showRecent.value = false
  }
  if (showThemeDropdown.value && !target.closest('.theme-selector-wrapper')) {
    showThemeDropdown.value = false
  }
}

/* ── Global keyboard handler ── */
function onGlobalKeydown(e: KeyboardEvent) {
  // "?" to open shortcuts (only when not typing in textarea)
  if (e.key === '?' && !(e.target instanceof HTMLTextAreaElement) && !(e.target instanceof HTMLInputElement)) {
    e.preventDefault()
    showShortcuts.value = true
  }
  // Ctrl+H to open find & replace
  if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
    e.preventDefault()
    openFindReplace(true)
    return
  }
  // Ctrl+F to open find-only
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    openFindReplace(false)
    return
  }
  // Escape to close modal or exit fullscreen
  if (e.key === 'Escape') {
    if (showFind.value) { closeFindReplace(); return }
    if (showShortcuts.value) { showShortcuts.value = false; return }
    if (acVisible.value) { acVisible.value = false; return }
    if (showRecent.value) { showRecent.value = false; return }
    if (isFullscreen.value) { isFullscreen.value = false }
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onGlobalKeydown)
  document.addEventListener('click', onDocClick)
  loadRecentFiles()
  loadFromHash()

  if (!canvasRef.value) return
  renderer = new WebGPURenderer()
  const ok = await renderer.init(canvasRef.value)
  if (!ok) { gpuOk.value = false; return }
  doRender()

  // Start axis label updates
  updateAxisLabels()

  // Stats polling
  statsInterval = setInterval(() => {
    if (!renderer) return
    fpsVal.value = renderer.getFPS()
    vertexCount.value = renderer.getVertexCount()
    const bounds = renderer.getBounds()
    boundsSize.value = bounds.size
  }, 500)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  document.removeEventListener('click', onDocClick)
  if (debounce) clearTimeout(debounce)
  if (minimapDebounce) clearTimeout(minimapDebounce)
  if (statsInterval) clearInterval(statsInterval)
  if (axisLabelRAF) cancelAnimationFrame(axisLabelRAF)
  renderer?.destroy(); renderer = null
})

watch(code, (v) => {
  // Save tabs
  saveTabs()
  // Also keep legacy key for backwards compat
  localStorage.setItem('scad-code', v)
  if (!autoRender.value) return
  if (debounce) clearTimeout(debounce)
  debounce = setTimeout(doRender, 400)
  // Debounce minimap render
  if (showMinimap.value) {
    if (minimapDebounce) clearTimeout(minimapDebounce)
    minimapDebounce = setTimeout(renderMinimap, 300)
  }
})

function doRender() {
  if (!renderer) return
  error.value = ''
  errorLine.value = -1
  try {
    const t0 = performance.now()
    const meshes = parseOpenSCAD(code.value)
    const t1 = performance.now()
    renderTime.value = Math.round(t1 - t0)
    meshCount.value = meshes.length
    triCount.value = meshes.reduce((s, m) => s + m.indices.length / 3, 0)
    lastParsedMeshes = meshes
    renderer.setMeshes(meshes)
  } catch (e: any) {
    let msg = e.message || String(e)
    const posMatch = msg.match(/@(\d+)/)
    if (posMatch) {
      const pos = parseInt(posMatch[1])
      const prefix = code.value.substring(0, pos)
      const line = prefix.split('\n').length
      errorLine.value = line
      msg = `${t('errorAtLine')} ${line}: ${msg}`
    }
    error.value = msg
  }
}

function loadExample(name: string) {
  if (EXAMPLES[name]) {
    code.value = EXAMPLES[name]
    addToRecent(name, EXAMPLES[name])
  }
}

function handleKey(e: KeyboardEvent) {
  // Autocomplete navigation
  if (acVisible.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      acIndex.value = (acIndex.value + 1) % acItems.value.length
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      acIndex.value = (acIndex.value - 1 + acItems.value.length) % acItems.value.length
      return
    }
    if (e.key === 'Enter' || e.key === 'Tab') {
      if (acItems.value.length) {
        e.preventDefault()
        acceptAutocomplete()
        return
      }
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      dismissAutocomplete()
      return
    }
  }

  if (e.key === 'Tab' && !acVisible.value) {
    e.preventDefault()
    const el = e.target as HTMLTextAreaElement
    const s = el.selectionStart, end = el.selectionEnd
    code.value = code.value.substring(0, s) + '    ' + code.value.substring(end)
    requestAnimationFrame(() => { el.selectionStart = el.selectionEnd = s + 4 })
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); doRender() }
}

function handleKeyUp() {
  updateAutocomplete()
  updateBracketMatch()
}

function handleClick() {
  updateBracketMatch()
  dismissAutocomplete()
}
</script>

<script lang="ts">
const EXAMPLES: Record<string, string> = {
  basic: `// Basic OpenSCAD primitives
cube([20, 15, 10]);

translate([30, 0, 0])
  sphere(r = 8, $fn = 32);

translate([0, 25, 0])
  cylinder(h = 15, r = 6, $fn = 24);

translate([30, 25, 0])
  cylinder(h = 15, r1 = 8, r2 = 3, $fn = 6);
`,

  csg: `// CSG Boolean Operations
// difference: subtracted parts in red

difference() {
    cube([30, 30, 30], center = true);
    sphere(r = 19, $fn = 32);
    translate([0, 0, 15])
        cylinder(h = 10, r = 10, $fn = 32);
}

translate([50, 0, 0])
color([0.2, 0.8, 0.5])
difference() {
    cylinder(h = 25, r = 12, center = true, $fn = 32);
    translate([0, 0, 3])
        cylinder(h = 22, r = 9, center = true, $fn = 32);
}
`,

  house: `// A simple house
// Walls
color([0.85, 0.75, 0.55])
difference() {
    cube([40, 25, 30]);
    // door
    translate([15, -1, 0])
        cube([10, 10, 18]);
    // windows
    translate([3, -1, 10])
        cube([8, 10, 8]);
    translate([29, -1, 10])
        cube([8, 10, 8]);
}

// Roof
color([0.7, 0.2, 0.15])
translate([20, 12.5, 25])
rotate([90, 0, 0])
    cylinder(h = 27, r1 = 0, r2 = 25, $fn = 4, center = true);

// Chimney
color([0.5, 0.3, 0.2])
translate([32, 5, 25])
    cube([5, 5, 12]);

// Door step
color([0.6, 0.6, 0.6])
translate([13, -3, 0])
    cube([14, 3, 2]);
`,

  tower: `// Decorative tower
color([0.4, 0.4, 0.5]) {
    // base
    cylinder(h = 5, r = 20, $fn = 8);

    // tiers
    translate([0, 0, 5])
        cylinder(h = 15, r1 = 18, r2 = 14, $fn = 8);

    translate([0, 0, 20])
        cylinder(h = 15, r1 = 14, r2 = 10, $fn = 8);

    translate([0, 0, 35])
        cylinder(h = 12, r1 = 10, r2 = 7, $fn = 8);
}

// dome
color([0.8, 0.7, 0.2])
translate([0, 0, 47])
    sphere(r = 8, $fn = 24);

// spire
color([0.8, 0.7, 0.2])
translate([0, 0, 53])
    cylinder(h = 15, r1 = 2, r2 = 0.3, $fn = 12);

// balconies
color([0.6, 0.55, 0.65])
translate([0, 0, 20])
    cylinder(h = 1.5, r = 16, $fn = 8);

color([0.6, 0.55, 0.65])
translate([0, 0, 35])
    cylinder(h = 1.5, r = 12, $fn = 8);
`,
}
</script>

<template>
  <div class="app" :class="isDark ? 'dark' : 'light'">
    <nav class="topbar">
      <div class="topbar-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
        </svg>
        <span class="brand">{{ t('title') }}</span>
      </div>
      <div class="topbar-right">
        <button class="tb-btn tb-btn-help" @click="showShortcuts = true" :title="t('shortcuts')">?</button>
        <button class="tb-btn" @click="toggleLang">{{ lang === 'ru' ? 'RU' : 'EN' }}</button>
        <!-- Theme selector dropdown -->
        <div class="theme-selector-wrapper">
          <button class="tb-btn" @click.stop="showThemeDropdown = !showThemeDropdown" :title="t('themeSelector')">
            {{ isDark ? '&#9790;' : '&#9788;' }}
            <svg width="8" height="8" viewBox="0 0 12 12" fill="currentColor" style="margin-left:3px;vertical-align:0px;"><path d="M2 4l4 4 4-4z"/></svg>
          </button>
          <div v-if="showThemeDropdown" class="theme-dropdown">
            <div
              v-for="theme in EDITOR_THEMES"
              :key="theme.id"
              class="theme-option"
              :class="{ active: theme.id === activeThemeId }"
              @click="selectTheme(theme.id)"
            >
              <span class="theme-swatch" :style="{ background: theme.vars['--bg'], borderColor: theme.vars['--accent'] }"></span>
              <span class="theme-option-name">{{ theme.name[lang] }}</span>
              <span v-if="theme.id === activeThemeId" class="theme-check">&#10003;</span>
            </div>
          </div>
        </div>
      </div>
    </nav>

    <!-- Shortcuts modal -->
    <Teleport to="body">
      <div v-if="showShortcuts" class="modal-backdrop" @click.self="showShortcuts = false">
        <div class="modal-box">
          <div class="modal-header">
            <span class="modal-title">{{ t('shortcutsTitle') }}</span>
            <button class="modal-close" @click="showShortcuts = false">&times;</button>
          </div>
          <div class="modal-body">
            <div class="shortcut-row"><kbd>Ctrl+Enter</kbd><span>{{ t('sc_render') }}</span></div>
            <div class="shortcut-row"><kbd>Tab</kbd><span>{{ t('sc_indent') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Z</kbd><span>{{ t('sc_undo') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+F</kbd><span>{{ t('sc_findOnly') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+H</kbd><span>{{ t('sc_findReplace') }}</span></div>
            <div class="shortcut-row"><kbd>?</kbd><span>{{ t('sc_shortcuts') }}</span></div>
            <div class="shortcut-row"><kbd>Escape</kbd><span>{{ t('sc_close') }}</span></div>
          </div>
        </div>
      </div>
    </Teleport>

    <div v-if="!gpuOk" class="no-gpu">{{ t('noGpu') }}</div>

    <div v-else class="main" :class="{ dragging: isDraggingDivider, fullscreen: isFullscreen }">
      <div
        class="editor-panel"
        :style="{ width: editorWidth + 'px' }"
        v-show="!isFullscreen"
        @dragenter="onEditorDragEnter"
        @dragover="onEditorDragOver"
        @dragleave="onEditorDragLeave"
        @drop="onEditorDrop"
      >
        <!-- Drag overlay -->
        <div v-if="isDragOver" class="drag-overlay">
          <div class="drag-overlay-content">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="12" y2="12"/><line x1="15" y1="15" x2="12" y2="12"/>
            </svg>
            <span>{{ t('dropHint') }}</span>
          </div>
        </div>

        <div class="toolbar">
          <button class="btn btn-primary" @click="doRender" title="Ctrl+Enter">
            {{ t('render') }}
          </button>
          <label class="auto-check">
            <input type="checkbox" v-model="autoRender" /> {{ t('auto') }}
          </label>
          <button class="btn btn-sm btn-icon" @click="openFile" :title="t('open')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
          </button>
          <button class="btn btn-sm btn-icon" @click="saveFile" :title="t('save')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
          </button>
          <!-- STL Export button -->
          <button class="btn btn-sm btn-icon" @click="doExportSTL" :title="t('exportStl')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              <rect x="14" y="1" width="8" height="6" rx="1" fill="currentColor" opacity="0.3"/>
            </svg>
          </button>
          <!-- Share button -->
          <div class="share-wrapper">
            <button class="btn btn-sm" @click="shareLink" :title="t('share')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
                <circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/>
                <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/>
              </svg>
              {{ t('share') }}
            </button>
            <span v-if="showCopied" class="copied-tooltip">{{ t('copied') }}</span>
          </div>
          <!-- Format button -->
          <button class="btn btn-sm" @click="formatCode" :title="t('format')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <line x1="3" y1="6" x2="21" y2="6"/><line x1="7" y1="12" x2="21" y2="12"/><line x1="5" y1="18" x2="21" y2="18"/>
            </svg>
            {{ t('format') }}
          </button>
          <!-- Recent dropdown -->
          <div class="recent-wrapper">
            <button class="btn btn-sm" @click.stop="showRecent = !showRecent">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
                <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
              </svg>
              {{ t('recent') }}
            </button>
            <div v-if="showRecent" class="recent-dropdown">
              <div v-if="!recentFiles.length" class="recent-empty">{{ t('noRecent') }}</div>
              <div
                v-for="entry in recentFiles"
                :key="entry.timestamp"
                class="recent-item"
                @click="loadRecent(entry)"
              >
                <span class="recent-name">{{ entry.name }}</span>
                <span class="recent-time">{{ formatRelativeTime(entry.timestamp) }}</span>
              </div>
            </div>
          </div>
          <span class="spacer" />
          <span class="ex-label">{{ t('examples') }}:</span>
          <button class="btn btn-sm" @click="loadExample('basic')">{{ t('basic') }}</button>
          <button class="btn btn-sm" @click="loadExample('csg')">{{ t('csg') }}</button>
          <button class="btn btn-sm" @click="loadExample('house')">{{ t('house') }}</button>
          <button class="btn btn-sm" @click="loadExample('tower')">{{ t('tower') }}</button>
        </div>

        <!-- Tab bar -->
        <div class="tab-bar">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            class="tab-item"
            :class="{ active: tab.id === activeTabId }"
            @click="switchTab(tab.id)"
            @dblclick.stop="startRenameTab(tab.id)"
          >
            <template v-if="editingTabId === tab.id">
              <input
                ref="tabNameInputRef"
                class="tab-name-input"
                v-model="editingTabName"
                @blur="finishRenameTab"
                @keydown.enter.prevent="finishRenameTab"
                @keydown.escape.prevent="cancelRenameTab"
                @click.stop
              />
            </template>
            <template v-else>
              <span class="tab-name">{{ tab.name }}</span>
              <button
                v-if="tabs.length > 1"
                class="tab-close"
                @click.stop="closeTab(tab.id)"
                :title="t('closeTab')"
              >&times;</button>
            </template>
          </div>
          <button class="tab-add" @click="addTab" :title="t('newTab')">+</button>
        </div>

        <!-- Find & Replace panel -->
        <div v-if="showFind" class="find-panel">
          <div class="find-row">
            <input
              ref="findInputRef"
              class="find-input"
              v-model="findText"
              :placeholder="t('find')"
              @keydown="handleFindKeydown"
            />
            <span class="find-count" v-if="findText">
              {{ findMatches.length > 0
                ? t('matchCount').replace('{x}', String(findMatchIndex + 1)).replace('{y}', String(findMatches.length))
                : t('noMatches') }}
            </span>
            <button class="find-btn" @click="findPrev" :title="t('prev')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="18 15 12 9 6 15"/></svg>
            </button>
            <button class="find-btn" @click="findNext" :title="t('next')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 12 15 18 9"/></svg>
            </button>
            <button class="find-btn find-close" @click="closeFindReplace">&times;</button>
          </div>
          <div v-if="showReplace" class="find-row">
            <input
              class="find-input"
              v-model="replaceText"
              :placeholder="t('replace')"
              @keydown="handleReplaceKeydown"
            />
            <button class="find-btn find-btn-text" @click="doReplace">{{ t('replace') }}</button>
            <button class="find-btn find-btn-text" @click="doReplaceAll">{{ t('replaceAll') }}</button>
          </div>
        </div>

        <div class="code-editor">
          <pre class="line-numbers" ref="lineNumRef" aria-hidden="true" v-html="lineNumbers"></pre>
          <div class="code-area">
            <pre class="highlight-layer" ref="highlightRef" aria-hidden="true"><code v-html="highlightedCode"></code></pre>
            <textarea
              ref="textareaRef"
              class="code"
              v-model="code"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              @keydown="handleKey"
              @keyup="handleKeyUp"
              @click="handleClick"
              @scroll="syncScroll"
            />
            <!-- Autocomplete popup -->
            <div
              v-if="acVisible && acItems.length"
              class="ac-popup"
              :style="{ top: acTop + 'px', left: acLeft + 'px' }"
            >
              <div
                v-for="(item, idx) in acItems"
                :key="item"
                class="ac-item"
                :class="{ active: idx === acIndex }"
                @mousedown.prevent="acIndex = idx; acceptAutocomplete()"
              >
                {{ item }}
              </div>
            </div>
          </div>
          <!-- Minimap -->
          <div v-if="showMinimap" class="minimap-container">
            <canvas
              ref="minimapCanvasRef"
              class="minimap-canvas"
              @mousedown="onMinimapMouseDown"
            />
          </div>
          <!-- Minimap toggle -->
          <button class="minimap-toggle" @click="toggleMinimap" :title="t('minimap')" :class="{ active: showMinimap }">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="7" height="18" rx="1"/><line x1="14" y1="5" x2="21" y2="5"/><line x1="14" y1="9" x2="21" y2="9"/><line x1="14" y1="13" x2="19" y2="13"/><line x1="14" y1="17" x2="20" y2="17"/>
            </svg>
          </button>
        </div>

        <div v-if="error" class="error">{{ error }}</div>

        <div class="stats">
          {{ t('meshes') }}: {{ meshCount }} &middot;
          {{ t('triangles') }}: {{ triCount }} &middot;
          {{ t('vertices') }}: {{ vertexCount }}
          <span v-if="renderTime > 0" class="render-time">&middot; {{ t('renderTime') }}: {{ renderTime }}ms</span>
          <span class="stat-fps">&middot; {{ t('fps') }}: {{ fpsVal }}</span>
          <span v-if="boundsSize[0] > 0 || boundsSize[1] > 0 || boundsSize[2] > 0" class="stat-size">
            &middot; {{ t('size') }}: {{ boundsSize[0].toFixed(1) }}&times;{{ boundsSize[1].toFixed(1) }}&times;{{ boundsSize[2].toFixed(1) }}
          </span>
          <span class="diff-note">{{ t('diff_note') }}</span>
        </div>
      </div>

      <div class="divider" v-show="!isFullscreen" @mousedown="onDividerDown"></div>

      <div class="canvas-panel">
        <canvas ref="canvasRef" class="gpu-canvas" />

        <div class="view-buttons">
          <button class="view-btn" @click="setView('top')" :title="t('top')">{{ t('top') }}</button>
          <button class="view-btn" @click="setView('front')" :title="t('front')">{{ t('front') }}</button>
          <button class="view-btn" @click="setView('right')" :title="t('right')">{{ t('right') }}</button>
          <button class="view-btn" @click="setView('iso')" :title="t('iso')">{{ t('iso') }}</button>
          <button class="view-btn" @click="setView('reset')" :title="t('reset')">{{ t('reset') }}</button>
          <button class="view-btn view-btn-icon" @click="takeScreenshot" :title="t('screenshot')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2z"/>
              <circle cx="12" cy="13" r="4"/>
            </svg>
          </button>
          <div class="view-separator"></div>
          <button class="view-btn view-btn-toggle" :class="{ active: showWireframe }" @click="toggleWireframe" :title="t('wireframe')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
            </svg>
          </button>
          <button class="view-btn view-btn-toggle" :class="{ active: showGrid }" @click="toggleGrid" :title="t('grid')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/><line x1="15" y1="3" x2="15" y2="21"/>
            </svg>
          </button>
          <button class="view-btn view-btn-toggle" :class="{ active: isAutoRotate }" @click="toggleAutoRotate" :title="t('autoRotate')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
            </svg>
          </button>
          <button class="view-btn view-btn-toggle" :class="{ active: isFullscreen }" @click="toggleFullscreen" :title="t('fullscreen')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path v-if="!isFullscreen" d="M8 3H5a2 2 0 00-2 2v3m18 0V5a2 2 0 00-2-2h-3m0 18h3a2 2 0 002-2v-3M3 16v3a2 2 0 002 2h3"/>
              <path v-else d="M4 14h3a2 2 0 012 2v3m4-5h3a2 2 0 002-2V9m-9 0V6a2 2 0 012-2h3m4 5V6a2 2 0 00-2-2h-3"/>
            </svg>
          </button>
          <div class="view-separator"></div>
          <!-- Orthographic toggle -->
          <button class="view-btn view-btn-toggle" :class="{ active: isOrthographic }" @click="toggleProjection" :title="isOrthographic ? t('persp') : t('ortho')">
            {{ isOrthographic ? t('ortho') : t('persp') }}
          </button>
          <!-- Zoom controls -->
          <div class="zoom-row">
            <button class="view-btn zoom-btn" @click="doZoomIn" :title="t('zoomIn')">+</button>
            <button class="view-btn zoom-btn" @click="doZoomOut" :title="t('zoomOut')">&minus;</button>
          </div>
          <div class="view-separator"></div>
          <!-- Background color swatches -->
          <div class="bg-color-row">
            <span class="bg-label">{{ t('bgColor') }}</span>
            <button
              v-for="(c, idx) in bgColors"
              :key="c.hex"
              class="bg-swatch"
              :class="{ active: idx === activeBg }"
              :style="{ background: c.hex }"
              :title="c.name"
              @click="setBgColor(idx)"
            />
          </div>
        </div>

        <!-- Axis labels -->
        <span
          v-if="axisLabelX.visible"
          class="axis-label axis-label-x"
          :style="{ left: axisLabelX.x + 'px', top: axisLabelX.y + 'px' }"
        >X</span>
        <span
          v-if="axisLabelY.visible"
          class="axis-label axis-label-y"
          :style="{ left: axisLabelY.x + 'px', top: axisLabelY.y + 'px' }"
        >Y</span>
        <span
          v-if="axisLabelZ.visible"
          class="axis-label axis-label-z"
          :style="{ left: axisLabelZ.x + 'px', top: axisLabelZ.y + 'px' }"
        >Z</span>

        <div class="canvas-hint">{{ t('hint') }}</div>
      </div>
    </div>
  </div>
</template>

<style>
:root {
  --bg: #141416;
  --surface: #1e1e22;
  --border: #2e2e34;
  --text: #e4e4e8;
  --text-dim: #888;
  --accent: #4a9eff;
  --hover: #28282e;
  --danger: #e74c3c;
  --canvas-bg: #18181c;

  --hl-comment: #6a6a7a;
  --hl-keyword: #5c9eff;
  --hl-number: #d19a66;
  --hl-string: #6ec87a;
  --hl-boolean: #c678dd;
  --hl-special: #56c8d8;
}

[data-theme="light"] {
  --bg: #f4f4f6;
  --surface: #fff;
  --border: #d4d4da;
  --text: #1a1a1e;
  --text-dim: #777;
  --accent: #2b7de9;
  --hover: #eaeaee;
  --canvas-bg: #e8e8ec;

  --hl-comment: #999;
  --hl-keyword: #1a6dd4;
  --hl-number: #c5600a;
  --hl-string: #2a8c3a;
  --hl-boolean: #9040b0;
  --hl-special: #1a8a99;
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

[data-theme="light"] .view-btn {
  background: rgba(255,255,255,.75);
  border-color: rgba(0,0,0,.12);
  color: rgba(0,0,0,.7);
}
[data-theme="light"] .view-btn:hover {
  background: rgba(43,125,233,.2);
  border-color: rgba(43,125,233,.4);
  color: var(--accent);
}

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
}
</style>

<style scoped>
.app { display: flex; flex-direction: column; height: 100vh; }

.topbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.topbar-left { display: flex; align-items: center; gap: 10px; }
.topbar-right { display: flex; align-items: center; gap: 8px; }
.logo { color: var(--accent); }
.brand { font-weight: 700; font-size: 0.95rem; }
.theme-label { font-size: 0.78rem; color: var(--text-dim); }
.tb-btn {
  padding: 4px 10px; border-radius: 5px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text); cursor: pointer; font-size: 0.8rem;
}
.tb-btn:hover { background: var(--hover); }

.no-gpu {
  flex: 1; display: flex; align-items: center; justify-content: center;
  color: var(--danger); font-size: 1.1rem; padding: 40px; text-align: center;
}

.main { flex: 1; display: flex; gap: 0; overflow: hidden; }
.main.dragging { cursor: col-resize; user-select: none; }

.editor-panel {
  min-width: 240px; max-width: 60vw;
  display: flex; flex-direction: column;
  background: var(--surface);
  flex-shrink: 0;
  position: relative;
}

.divider {
  width: 5px; cursor: col-resize;
  background: var(--border);
  flex-shrink: 0;
  transition: background 0.15s;
  position: relative;
}
.divider:hover, .main.dragging .divider {
  background: var(--accent);
}

.toolbar {
  display: flex; align-items: center; gap: 8px; padding: 8px 12px;
  border-bottom: 1px solid var(--border); flex-wrap: wrap;
}
.btn {
  padding: 5px 12px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--surface); color: var(--text); cursor: pointer; font-size: 0.8rem;
  transition: background 0.12s;
}
.btn:hover { background: var(--hover); }
.btn-primary {
  background: var(--accent); color: #fff; border-color: var(--accent); font-weight: 600;
}
.btn-primary:hover { opacity: 0.9; }
.btn-sm { padding: 3px 8px; font-size: 0.72rem; }

.auto-check {
  font-size: 0.75rem; color: var(--text-dim); display: flex; align-items: center; gap: 4px; cursor: pointer;
}
.spacer { flex: 1; }
.ex-label { font-size: 0.72rem; color: var(--text-dim); }

/* ── Tab bar ── */
.tab-bar {
  display: flex; align-items: center;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  height: 30px;
  flex-shrink: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.tab-bar::-webkit-scrollbar { display: none; }

.tab-item {
  display: flex; align-items: center; gap: 4px;
  padding: 0 12px;
  height: 100%;
  font-size: 0.72rem;
  color: var(--text-dim);
  cursor: pointer;
  border-right: 1px solid var(--border);
  white-space: nowrap;
  user-select: none;
  transition: background 0.1s, color 0.1s;
  position: relative;
}
.tab-item:hover { background: var(--hover); color: var(--text); }
.tab-item.active {
  background: var(--surface);
  color: var(--text);
  border-bottom: 2px solid var(--accent);
}

.tab-name { max-width: 120px; overflow: hidden; text-overflow: ellipsis; }

.tab-name-input {
  width: 80px;
  font-size: 0.72rem;
  padding: 1px 4px;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--accent);
  border-radius: 3px;
  outline: none;
}

.tab-close {
  background: none; border: none; color: var(--text-dim);
  font-size: 0.85rem; cursor: pointer; padding: 0 2px;
  line-height: 1; opacity: 0;
  transition: opacity 0.1s;
}
.tab-item:hover .tab-close,
.tab-item.active .tab-close { opacity: 1; }
.tab-close:hover { color: var(--danger); }

.tab-add {
  background: none; border: none; color: var(--text-dim);
  font-size: 1rem; cursor: pointer; padding: 0 10px;
  height: 100%; display: flex; align-items: center;
  transition: color 0.1s;
}
.tab-add:hover { color: var(--accent); }

/* ── Code editor with syntax highlight + line numbers ── */

.code-editor {
  flex: 1; display: flex; overflow: hidden; position: relative;
  background: var(--bg);
}

.line-numbers {
  width: 44px; flex-shrink: 0;
  padding: 12px 8px 12px 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'SF Mono', monospace;
  font-size: 0.82rem; line-height: 1.55;
  text-align: right; color: var(--text-dim);
  background: var(--surface);
  border-right: 1px solid var(--border);
  overflow: hidden;
  user-select: none;
  white-space: pre;
}

.code-area {
  flex: 1; position: relative; overflow: hidden;
}

.highlight-layer {
  position: absolute; top: 0; left: 0; right: 0; bottom: 0;
  padding: 12px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'SF Mono', monospace;
  font-size: 0.82rem; line-height: 1.55;
  color: var(--text);
  overflow: hidden;
  pointer-events: none;
  white-space: pre;
  tab-size: 4;
  margin: 0;
}
.highlight-layer code {
  font-family: inherit; font-size: inherit; line-height: inherit;
  tab-size: inherit;
}

.code {
  position: absolute; top: 0; left: 0;
  width: 100%; height: 100%;
  resize: none;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'SF Mono', monospace;
  font-size: 0.82rem; line-height: 1.55;
  padding: 12px; border: none; outline: none;
  background: transparent;
  color: transparent;
  caret-color: var(--text);
  tab-size: 4;
  white-space: pre;
  overflow: auto;
  z-index: 1;
}
.code::selection {
  background: rgba(74, 158, 255, 0.25);
}

/* Syntax highlighting colors */
:deep(.hl-comment) { color: var(--hl-comment); font-style: italic; }
:deep(.hl-keyword) { color: var(--hl-keyword); }
:deep(.hl-number) { color: var(--hl-number); }
:deep(.hl-string) { color: var(--hl-string); }
:deep(.hl-boolean) { color: var(--hl-boolean); }
:deep(.hl-special) { color: var(--hl-special); }
:deep(.bracket-match) {
  background: rgba(74, 158, 255, 0.18);
  border-radius: 2px;
  outline: 1px solid rgba(74, 158, 255, 0.35);
}

.error {
  padding: 8px 12px; margin: 6px 10px;
  background: rgba(231,76,60,.1); border: 1px solid rgba(231,76,60,.3);
  border-radius: 6px; color: var(--danger);
  font-size: 0.76rem; font-family: monospace; white-space: pre-wrap;
}

.stats {
  padding: 6px 12px; font-size: 0.72rem; color: var(--text-dim);
  border-top: 1px solid var(--border);
  display: flex; align-items: center; gap: 6px;
  flex-shrink: 0;
}
.render-time { color: var(--accent); }
.diff-note { margin-left: auto; font-style: italic; opacity: 0.7; }

.canvas-panel {
  flex: 1; position: relative; background: var(--canvas-bg); overflow: hidden;
}
.gpu-canvas { width: 100%; height: 100%; display: block; }
.canvas-hint {
  position: absolute; bottom: 8px; left: 50%; transform: translateX(-50%);
  font-size: 0.68rem; color: rgba(255,255,255,.35); pointer-events: none;
  white-space: nowrap;
}

/* ── View preset buttons ── */
.view-buttons {
  position: absolute; top: 10px; right: 10px;
  display: flex; flex-direction: column; gap: 4px;
  z-index: 10;
}
.view-btn {
  padding: 3px 10px;
  border-radius: 10px;
  border: 1px solid rgba(255,255,255,.15);
  background: rgba(30,30,34,.7);
  color: rgba(255,255,255,.8);
  font-size: 0.68rem;
  font-weight: 500;
  cursor: pointer;
  backdrop-filter: blur(6px);
  transition: background 0.12s, border-color 0.12s;
  text-align: center;
  min-width: 44px;
  line-height: 1.4;
}
.view-btn:hover {
  background: rgba(74,158,255,.3);
  border-color: rgba(74,158,255,.5);
  color: #fff;
}
.view-btn-icon {
  display: flex; align-items: center; justify-content: center;
  padding: 5px 10px;
}
.view-btn-toggle {
  display: flex; align-items: center; justify-content: center;
  padding: 5px 10px;
}
.view-btn-toggle.active {
  background: rgba(74,158,255,.35);
  border-color: rgba(74,158,255,.6);
  color: #fff;
}
[data-theme="light"] .view-btn-toggle.active {
  background: rgba(43,125,233,.25);
  border-color: rgba(43,125,233,.5);
  color: var(--accent);
}
.view-separator {
  height: 1px;
  background: rgba(255,255,255,.12);
  margin: 2px 4px;
}
[data-theme="light"] .view-separator {
  background: rgba(0,0,0,.1);
}

/* ── Error line highlight in gutter ── */
:deep(.line-error) {
  background: rgba(231,76,60,.35);
  color: #ff6b5a;
  border-radius: 2px;
  padding: 0 2px;
}

/* ── Help button in topbar ── */
.tb-btn-help {
  font-weight: 700;
  font-size: 0.85rem;
  width: 28px; height: 28px;
  display: inline-flex; align-items: center; justify-content: center;
  padding: 0;
  border-radius: 50%;
}

/* ── Shortcuts modal ── */
.modal-backdrop {
  position: fixed; inset: 0;
  background: rgba(0,0,0,.55);
  display: flex; align-items: center; justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(4px);
}
.modal-box {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  min-width: 320px; max-width: 440px;
  box-shadow: 0 12px 40px rgba(0,0,0,.4);
  overflow: hidden;
}
.modal-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
.modal-title { font-weight: 700; font-size: 0.95rem; }
.modal-close {
  background: none; border: none; color: var(--text-dim); font-size: 1.4rem;
  cursor: pointer; line-height: 1; padding: 0 4px;
}
.modal-close:hover { color: var(--text); }
.modal-body { padding: 14px 18px; }
.shortcut-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 7px 0;
  font-size: 0.82rem;
  border-bottom: 1px solid rgba(128,128,128,.12);
}
.shortcut-row:last-child { border-bottom: none; }
.shortcut-row kbd {
  background: var(--hover);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 2px 8px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
  color: var(--text);
  white-space: nowrap;
}
.shortcut-row span { color: var(--text-dim); font-size: 0.8rem; }

/* ── Autocomplete popup ── */
.ac-popup {
  position: absolute;
  z-index: 100;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 6px 20px rgba(0,0,0,.35);
  min-width: 150px;
  max-width: 240px;
  overflow: hidden;
  backdrop-filter: blur(8px);
}
.ac-item {
  padding: 5px 12px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.78rem;
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
}
.ac-item:hover, .ac-item.active {
  background: rgba(74,158,255,.2);
  color: var(--accent);
}

/* ── Background color swatches ── */
.bg-color-row {
  display: flex; align-items: center; gap: 4px;
  padding: 2px 4px;
}
.bg-label {
  font-size: 0.6rem;
  color: rgba(255,255,255,.5);
  margin-right: 2px;
}
[data-theme="light"] .bg-label {
  color: rgba(0,0,0,.45);
}
.bg-swatch {
  width: 16px; height: 16px;
  border-radius: 50%;
  border: 2px solid rgba(255,255,255,.15);
  cursor: pointer;
  transition: border-color 0.12s, transform 0.12s;
  padding: 0;
}
.bg-swatch:hover {
  border-color: rgba(74,158,255,.5);
  transform: scale(1.15);
}
.bg-swatch.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}
[data-theme="light"] .bg-swatch {
  border-color: rgba(0,0,0,.15);
}
[data-theme="light"] .bg-swatch.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

/* ── Drag & drop overlay ── */
.drag-overlay {
  position: absolute; inset: 0; z-index: 200;
  background: rgba(74, 158, 255, 0.08);
  border: 2.5px dashed var(--accent);
  border-radius: 8px;
  display: flex; align-items: center; justify-content: center;
  pointer-events: none;
}
.drag-overlay-content {
  display: flex; flex-direction: column; align-items: center; gap: 10px;
  color: var(--accent);
  font-size: 0.85rem; font-weight: 600;
}
.drag-overlay-content svg { opacity: 0.7; }

/* ── Open / Save icon buttons in toolbar ── */
.btn-icon {
  display: inline-flex; align-items: center; justify-content: center;
  padding: 4px 7px;
}

/* ── Find & Replace panel ── */
.find-panel {
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  padding: 6px 10px;
  display: flex; flex-direction: column; gap: 5px;
  flex-shrink: 0;
}
.find-row {
  display: flex; align-items: center; gap: 5px;
}
.find-input {
  flex: 1; min-width: 0;
  padding: 4px 8px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.78rem;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  outline: none;
}
.find-input:focus {
  border-color: var(--accent);
}
.find-count {
  font-size: 0.7rem; color: var(--text-dim); white-space: nowrap;
  min-width: 50px; text-align: center;
}
.find-btn {
  background: var(--surface); border: 1px solid var(--border);
  color: var(--text); border-radius: 4px; cursor: pointer;
  padding: 3px 6px; display: inline-flex; align-items: center; justify-content: center;
  font-size: 0.75rem;
}
.find-btn:hover { background: var(--hover); }
.find-btn-text { padding: 3px 8px; font-size: 0.7rem; }
.find-close {
  font-size: 1rem; line-height: 1; padding: 2px 6px;
}

/* ── Find match highlights ── */
:deep(.find-match) {
  background: rgba(255, 210, 0, 0.28);
  border-radius: 2px;
}
:deep(.find-match-active) {
  background: rgba(255, 165, 0, 0.45);
  border-radius: 2px;
  outline: 1px solid rgba(255, 165, 0, 0.6);
}

/* ── Status bar extras ── */
.stat-fps { color: var(--hl-special); }
.stat-size { color: var(--text-dim); }

/* ── Share button wrapper ── */
.share-wrapper {
  position: relative;
  display: inline-flex;
}
.copied-tooltip {
  position: absolute;
  top: -28px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: #fff;
  padding: 3px 10px;
  border-radius: 5px;
  font-size: 0.68rem;
  font-weight: 600;
  white-space: nowrap;
  pointer-events: none;
  animation: fadeInUp 0.2s ease;
  z-index: 100;
}
@keyframes fadeInUp {
  from { opacity: 0; transform: translateX(-50%) translateY(4px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}

/* ── Recent dropdown ── */
.recent-wrapper {
  position: relative;
  display: inline-flex;
}
.recent-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,.35);
  min-width: 220px;
  max-height: 280px;
  overflow-y: auto;
  z-index: 200;
}
.recent-empty {
  padding: 12px 16px;
  font-size: 0.75rem;
  color: var(--text-dim);
  text-align: center;
}
.recent-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 7px 14px;
  cursor: pointer;
  font-size: 0.75rem;
  border-bottom: 1px solid rgba(128,128,128,.08);
  transition: background 0.1s;
}
.recent-item:last-child { border-bottom: none; }
.recent-item:hover { background: var(--hover); }
.recent-name { color: var(--text); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 130px; }
.recent-time { color: var(--text-dim); font-size: 0.68rem; white-space: nowrap; margin-left: 10px; }

/* ── Minimap ── */
.minimap-container {
  width: 60px;
  flex-shrink: 0;
  position: relative;
  border-left: 1px solid var(--border);
  cursor: pointer;
}
.minimap-canvas {
  width: 100%;
  height: 100%;
  display: block;
}
.minimap-toggle {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 5;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 3px 4px;
  cursor: pointer;
  color: var(--text-dim);
  display: flex; align-items: center; justify-content: center;
  transition: color 0.1s, border-color 0.1s;
  opacity: 0.6;
}
.minimap-toggle:hover { opacity: 1; color: var(--text); }
.minimap-toggle.active { opacity: 1; color: var(--accent); border-color: var(--accent); }

/* ── Theme selector dropdown ── */
.theme-selector-wrapper {
  position: relative;
  display: inline-flex;
}
.theme-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,.35);
  min-width: 170px;
  z-index: 200;
  overflow: hidden;
}
.theme-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  cursor: pointer;
  font-size: 0.75rem;
  color: var(--text);
  transition: background 0.1s;
}
.theme-option:hover { background: var(--hover); }
.theme-option.active { background: rgba(74,158,255,.1); }
.theme-swatch {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid;
  flex-shrink: 0;
}
.theme-option-name { flex: 1; }
.theme-check { color: var(--accent); font-size: 0.85rem; font-weight: 700; }

/* ── Orthographic / Zoom in view-buttons ── */
.zoom-row {
  display: flex;
  gap: 4px;
}
.zoom-btn {
  flex: 1;
  font-size: 0.85rem !important;
  font-weight: 700;
  line-height: 1;
  padding: 3px 8px;
}

/* ── Axis labels ── */
.axis-label {
  position: absolute;
  z-index: 5;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 0.72rem;
  font-weight: 700;
  pointer-events: none;
  text-shadow: 0 1px 3px rgba(0,0,0,.7), 0 0 6px rgba(0,0,0,.4);
  transform: translate(-50%, -50%);
  user-select: none;
}
.axis-label-x { color: #e05555; }
.axis-label-y { color: #44cc55; }
.axis-label-z { color: #4488ee; }

@media (max-width: 800px) {
  .main { flex-direction: column; }
  .editor-panel { width: 100% !important; max-width: 100% !important; height: 40vh; border-right: none; border-bottom: 1px solid var(--border); }
  .divider { display: none; }
  .canvas-panel { height: 60vh; }
}
</style>
