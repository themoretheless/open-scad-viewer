<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { parseOpenSCAD, exportToSTL } from './services/openscadParser'
import { WebGPURenderer } from './services/webgpuRenderer'
import { EXAMPLES } from './examples'

const lang = ref<'ru'|'en'>((localStorage.getItem('scad-lang') as 'ru' | 'en' | null) || 'ru')
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
const meshCount = ref(0)
const triCount = ref(0)
const gpuOk = ref(true)
const autoRender = ref(true)

let renderer: WebGPURenderer | null = null
let debounce: ReturnType<typeof setTimeout> | null = null

onMounted(async () => {
  if (!canvasRef.value) return
  renderer = new WebGPURenderer()
  const ok = await renderer.init(canvasRef.value)
  if (!ok) { gpuOk.value = false; return }
  window.addEventListener('resize', onWindowResize)
  doRender()
})

const onWindowResize = () => { renderer?.resize() }

onUnmounted(() => {
  if (debounce) clearTimeout(debounce)
  window.removeEventListener('resize', onWindowResize)
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
  try {
    const meshes = parseOpenSCAD(code.value)
    meshCount.value = meshes.length
    const idxTotal = meshes.reduce((s, m) => s + m.indices.length, 0)
    triCount.value = idxTotal > 0 && idxTotal % 3 === 0 ? idxTotal / 3 : Math.floor(idxTotal / 3)
    renderer.setMeshes(meshes)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

function exportSTL() {
  try {
    const meshes = parseOpenSCAD(code.value)
    if (!meshes.length) {
      error.value = 'No geometry to export'
      return
    }
    const stl = exportToSTL(meshes)
    const blob = new Blob([stl], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'model.stl'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

function loadExample(name: string) {
  if (EXAMPLES[name]) code.value = EXAMPLES[name]
}

function handleKey(e: KeyboardEvent) {
  if (e.key === 'Tab') {
    e.preventDefault()
    const el = e.target as HTMLTextAreaElement
    const s = el.selectionStart, end = el.selectionEnd
    code.value = code.value.substring(0, s) + '    ' + code.value.substring(end)
    requestAnimationFrame(() => { el.selectionStart = el.selectionEnd = s + 4 })
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); doRender() }
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
        <button class="tb-btn" @click="toggleLang">{{ lang === 'ru' ? 'RU' : 'EN' }}</button>
        <span class="theme-label">{{ t('theme') }}</span>
        <button class="tb-btn" @click="toggleTheme">{{ isDark ? '&#9790;' : '&#9788;' }}</button>
      </div>
    </nav>

    <div v-if="!gpuOk" class="no-gpu">{{ t('noGpu') }}</div>

    <div v-else class="main">
      <div class="editor-panel">
        <div class="toolbar">
          <button class="btn btn-primary" @click="doRender" title="Ctrl+Enter">
            {{ t('render') }}
          </button>
          <button class="btn" @click="exportSTL">Export STL</button>
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

        <textarea
          class="code"
          v-model="code"
          spellcheck="false"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          @keydown="handleKey"
        />

        <div v-if="error" class="error">{{ error }}</div>

        <div class="stats">
          {{ t('meshes') }}: {{ meshCount }} &middot;
          {{ t('triangles') }}: {{ triCount }}
          <span class="diff-note">{{ t('diff_note') }}</span>
        </div>
      </div>

      <div class="canvas-panel">
        <canvas ref="canvasRef" class="gpu-canvas" />
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
}

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

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

.editor-panel {
  width: 420px; min-width: 280px; max-width: 50vw;
  display: flex; flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--surface);
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

.code {
  flex: 1; resize: none;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'SF Mono', monospace;
  font-size: 0.82rem; line-height: 1.55;
  padding: 12px; border: none; outline: none;
  background: var(--bg); color: var(--text);
  tab-size: 4;
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
}
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

@media (max-width: 800px) {
  .main { flex-direction: column; }
  .editor-panel { width: 100%; max-width: 100%; height: 40vh; border-right: none; border-bottom: 1px solid var(--border); }
  .canvas-panel { height: 60vh; }
}
</style>
