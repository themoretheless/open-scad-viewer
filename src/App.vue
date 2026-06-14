<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { parseOpenSCAD } from './services/openscadParser'
import { WebGPURenderer } from './services/webgpuRenderer'

const lang = ref<'ru'|'en'>((localStorage.getItem('scad-lang') as any) || 'ru')
const isDark = ref(true)

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
  },
}

const t = (k: string) => L[lang.value]?.[k] ?? k
const toggleLang = () => { lang.value = lang.value === 'ru' ? 'en' : 'ru'; localStorage.setItem('scad-lang', lang.value) }

onMounted(() => {
  const saved = localStorage.getItem('scad-theme')
  isDark.value = saved !== 'light'
  applyTheme()
})

function applyTheme() {
  document.documentElement.setAttribute('data-theme', isDark.value ? '' : 'light')
  localStorage.setItem('scad-theme', isDark.value ? 'dark' : 'light')
}
function toggleTheme() { isDark.value = !isDark.value; applyTheme() }

/* ── Editor + Renderer ── */

const code = ref(localStorage.getItem('scad-code') || EXAMPLES.basic)
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

let renderer: WebGPURenderer | null = null
let debounce: ReturnType<typeof setTimeout> | null = null

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

function highlightCode(src: string, bmA: number, bmB: number): string {
  const esc = (s: string) => s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')

  const result: string[] = []
  let i = 0
  const len = src.length

  const isBracketMatch = (pos: number) => pos === bmA || pos === bmB

  while (i < len) {
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
  return result.join('') + '\n'
}

const highlightedCode = computed(() => highlightCode(code.value, bracketMatchA.value, bracketMatchB.value))

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
}

/* ── View presets ── */
function setView(name: string) {
  if (!renderer) return
  switch (name) {
    case 'top':
      renderer.setCamera(0, Math.PI / 2)
      break
    case 'front':
      renderer.setCamera(0, 0)
      break
    case 'right':
      renderer.setCamera(Math.PI / 2, 0)
      break
    case 'iso':
      renderer.setCamera(0.6, 0.4)
      break
    case 'reset':
      renderer.autoFitAll()
      renderer.setCamera(0.6, 0.4)
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

/* ── Global keyboard handler ── */
function onGlobalKeydown(e: KeyboardEvent) {
  // "?" to open shortcuts (only when not typing in textarea)
  if (e.key === '?' && !(e.target instanceof HTMLTextAreaElement) && !(e.target instanceof HTMLInputElement)) {
    e.preventDefault()
    showShortcuts.value = true
  }
  // Escape to close modal or exit fullscreen
  if (e.key === 'Escape') {
    if (showShortcuts.value) { showShortcuts.value = false; return }
    if (acVisible.value) { acVisible.value = false; return }
    if (isFullscreen.value) { isFullscreen.value = false }
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onGlobalKeydown)
  if (!canvasRef.value) return
  renderer = new WebGPURenderer()
  const ok = await renderer.init(canvasRef.value)
  if (!ok) { gpuOk.value = false; return }
  doRender()
})

onUnmounted(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  if (debounce) clearTimeout(debounce)
  renderer?.destroy(); renderer = null
})

watch(code, (v) => {
  localStorage.setItem('scad-code', v)
  if (!autoRender.value) return
  if (debounce) clearTimeout(debounce)
  debounce = setTimeout(doRender, 400)
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
  if (EXAMPLES[name]) code.value = EXAMPLES[name]
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
        <span class="theme-label">{{ t('theme') }}</span>
        <button class="tb-btn" @click="toggleTheme">{{ isDark ? '&#9790;' : '&#9788;' }}</button>
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
            <div class="shortcut-row"><kbd>Ctrl+F</kbd><span>{{ t('sc_find') }}</span></div>
            <div class="shortcut-row"><kbd>?</kbd><span>{{ t('sc_shortcuts') }}</span></div>
            <div class="shortcut-row"><kbd>Escape</kbd><span>{{ t('sc_close') }}</span></div>
          </div>
        </div>
      </div>
    </Teleport>

    <div v-if="!gpuOk" class="no-gpu">{{ t('noGpu') }}</div>

    <div v-else class="main" :class="{ dragging: isDraggingDivider, fullscreen: isFullscreen }">
      <div class="editor-panel" :style="{ width: editorWidth + 'px' }" v-show="!isFullscreen">
        <div class="toolbar">
          <button class="btn btn-primary" @click="doRender" title="Ctrl+Enter">
            {{ t('render') }}
          </button>
          <label class="auto-check">
            <input type="checkbox" v-model="autoRender" /> {{ t('auto') }}
          </label>
          <span class="spacer" />
          <span class="ex-label">{{ t('examples') }}:</span>
          <button class="btn btn-sm" @click="loadExample('basic')">{{ t('basic') }}</button>
          <button class="btn btn-sm" @click="loadExample('csg')">{{ t('csg') }}</button>
          <button class="btn btn-sm" @click="loadExample('house')">{{ t('house') }}</button>
          <button class="btn btn-sm" @click="loadExample('tower')">{{ t('tower') }}</button>
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
        </div>

        <div v-if="error" class="error">{{ error }}</div>

        <div class="stats">
          {{ t('meshes') }}: {{ meshCount }} &middot;
          {{ t('triangles') }}: {{ triCount }}
          <span v-if="renderTime > 0" class="render-time">&middot; {{ t('renderTime') }}: {{ renderTime }}ms</span>
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

@media (max-width: 800px) {
  .main { flex-direction: column; }
  .editor-panel { width: 100% !important; max-width: 100% !important; height: 40vh; border-right: none; border-bottom: 1px solid var(--border); }
  .divider { display: none; }
  .canvas-panel { height: 60vh; }
}
</style>
