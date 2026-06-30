<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { parseOpenSCADWithAST } from './services/openscadParser'
import type { MeshData, ASTNode, Annotation, ProfileEntry } from './services/openscadParser'
import { WebGPURenderer } from './services/webgpuRenderer'
import { exportSTL } from './services/stlExport'
import { exportOBJ } from './services/objExport'
import { export3MF } from './services/threemfExport'
import { parseSTL } from './services/stlImport'
import { exportAllTabsAsZip } from './services/zipExport'

const savedLang = localStorage.getItem('scad-lang')
const lang = ref<'ru'|'en'|'de'|'zh'>(['ru','en','de','zh'].includes(savedLang as any) ? (savedLang as 'ru'|'en'|'de'|'zh') : 'ru')
const isDark = ref(true)

/* ── Security helpers ── */
function escapeHtml(s: string): string {
  return String(s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;')
}
function escapeAttr(s: string): string { return escapeHtml(s) }
function safeParse<T>(raw: string | null, fallback: T): T {
  if (!raw) return fallback
  try { const v = JSON.parse(raw); return (v ?? fallback) as T } catch { return fallback }
}

/* ── Editor Themes ── */
interface EditorTheme {
  id: string
  name: { ru: string; en: string; de: string; zh: string }
  dark: boolean
  vars: Record<string, string>
}

const EDITOR_THEMES: EditorTheme[] = [
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

const activeThemeId = ref(localStorage.getItem('scad-editor-theme') || 'default-dark')
const showThemeDropdown = ref(false)

/* ── Responsive hamburger menu ── */
const hamburgerOpen = ref(false)

const L: Record<string, Record<string, string>> = {
  ru: {
    title: 'OpenSCAD 3D Просмотрщик',
    subtitle: 'Редактор OpenSCAD с рендерингом через WebGPU',
    render: 'Рендер', auto: 'Авто', examples: 'Примеры',
    basic: 'Базовый', csg: 'CSG', house: 'Домик', tower: 'Башня',
    meshes: 'Объектов', triangles: 'Треугольников',
    hint: 'ЛКМ: вращение · Ctrl+ЛКМ: привязка 15° · ПКМ/Shift: перемещение · колёсико: зум · Ctrl+Enter: рендер',
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
    shareInvalid: 'Недопустимая ссылка общего доступа',
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
    preferences: 'Настройки',
    fontSize: 'Размер шрифта',
    tabSize: 'Размер табуляции',
    autoRenderDelay: 'Задержка авто-рендера',
    showMinimapPref: 'Миникарта',
    showLineNumbers: 'Номера строк',
    console: 'Консоль',
    consoleClear: 'Очистить',
    copyImage: 'Копировать',
    copiedImage: 'Скопировано!',
    objectTree: 'Дерево объектов',
    parsedNodes: 'Разобрано {n} узлов',
    generatedMeshes: 'Создано {m} мешей, {t} треугольников за {ms}мс',
    sc_commentToggle: 'Закомментировать/раскомментировать',
    commandPalette: 'Палитра команд',
    goToLine: 'Перейти к строке',
    goToLinePlaceholder: 'Перейти к строке (1-{n})',
    wordWrap: 'Перенос строк',
    selChars: 'Выд: {x} симв, {y} строк',
    animationTimeline: 'Анимация',
    animPlay: 'Играть',
    animPause: 'Пауза',
    animDuration: 'Длительность',
    parameters: 'Параметры',
    noParameters: 'Нет числовых переменных',
    lighting: 'Освещение',
    lightDefault: 'Обычное',
    lightStudio: 'Студия',
    lightOutdoor: 'Улица',
    lightDramatic: 'Драма',
    lightSoft: 'Мягкое',
    clipPlane: 'Сечение',
    fog: 'Туман',
    reflection: 'Отражение',
    sc_commandPalette: 'Палитра команд',
    sc_goToLine: 'Перейти к строке',
    sc_wordWrap: 'Перенос строк',
    cmdRender: 'Рендер',
    cmdFormatCode: 'Форматировать код',
    cmdToggleWireframe: 'Переключить каркас',
    cmdToggleGrid: 'Переключить сетку',
    cmdToggleAutoRotate: 'Переключить авто-вращение',
    cmdToggleFullscreen: 'Переключить полный экран',
    cmdToggleOrtho: 'Переключить проекцию',
    cmdTakeScreenshot: 'Сделать скриншот',
    cmdCopyImage: 'Копировать как изображение',
    cmdExportSTL: 'Экспорт STL',
    cmdOpenFile: 'Открыть файл',
    cmdSaveFile: 'Сохранить файл',
    cmdShare: 'Поделиться ссылкой',
    cmdToggleTheme: 'Переключить тему',
    cmdToggleLang: 'Переключить язык',
    cmdShowShortcuts: 'Показать горячие клавиши',
    cmdPreferences: 'Настройки',
    cmdToggleWordWrap: 'Переключить перенос строк',
    cmdGoToLine: 'Перейти к строке',
    cmdToggleMinimap: 'Переключить миникарту',
    cmdToggleConsole: 'Переключить консоль',
    cmdToggleObjectTree: 'Переключить дерево объектов',
    cmdNewTab: 'Новая вкладка',
    exportObj: 'Экспорт OBJ',
    cmdExportOBJ: 'Экспорт OBJ',
    gear: 'Шестерня',
    vase: 'Ваза',
    chess: 'Пешка',
    gearTip: 'Зубчатое колесо',
    vaseTip: 'Фигурная ваза',
    chessTip: 'Шахматная пешка',
    basicTip: 'Базовые формы',
    csgTip: 'Булевы операции',
    houseTip: 'Простой домик',
    towerTip: 'Декоративная башня',
    bgGradDark: 'Тёмный градиент',
    bgGradBlue: 'Голубой градиент',
    bgGradSunset: 'Закат',
    sc_wasd: 'Камера: вперёд/назад/орбита',
    sc_shiftWasd: 'Панорамирование камеры',
    sc_qe: 'Камера: наклон вверх/вниз',
    initWebGPU: 'Инициализация WebGPU…',
    gizmoTip: 'Кликните по оси, чтобы посмотреть вдоль неё',
    ctxResetView: 'Сбросить вид',
    ctxFitScreen: 'Вписать в экран',
    ctxScreenshot: 'Скриншот',
    ctxCopyImage: 'Копировать изображение',
    ctxToggleWireframe: 'Переключить каркас',
    ctxToggleGrid: 'Переключить сетку',
    ctxExportStl: 'Экспорт STL',
    statMeshes: 'Меши',
    statTriangles: 'Треуг.',
    statVertices: 'Вершины',
    statFps: 'FPS',
    statRender: 'Рендер',
    statSize: 'Размер',
    resetPrefs: 'Сбросить настройки',
    prefsReset: 'Настройки сброшены',
    undo: 'Отменить',
    tabClosed: 'Вкладка закрыта',
    tabRestored: 'Вкладка восстановлена',
    // Accessibility (aria) labels
    ariaHelp: 'Горячие клавиши',
    ariaPreferences: 'Настройки',
    ariaToggleLang: 'Переключить язык',
    ariaThemeSelector: 'Выбор темы',
    ariaCloseModal: 'Закрыть',
    ariaShortcutsDialog: 'Горячие клавиши',
    ariaPreferencesDialog: 'Настройки',
    ariaCommandPaletteDialog: 'Палитра команд',
    ariaGoToLineDialog: 'Перейти к строке',
    ariaGizmo: 'Навигационный куб',
    ariaConsole: 'Переключить консоль',
    ariaMinimap: 'Переключить миникарту',
    ariaNewTab: 'Новая вкладка',
    ariaAnimPlay: 'Играть/пауза анимации',
    // Welcome / onboarding
    welcomeTitle: 'OpenSCAD 3D Просмотрщик',
    welcomeTagline: 'Пишите OpenSCAD и смотрите результат в реальном времени с рендерингом через WebGPU.',
    welcomeFeat1: 'Редактирование OpenSCAD в реальном времени',
    welcomeFeat2: 'Быстрый 3D-рендеринг на WebGPU',
    welcomeFeat3: 'Экспорт в STL / OBJ / 3MF',
    welcomeFeat4: 'Темы оформления и горячие клавиши',
    welcomeStart: 'Начать',
    showWelcome: 'Показать приветствие',
    cmdShowWelcome: 'Показать приветствие',
    ariaWelcomeDialog: 'Добро пожаловать',
    // Empty state
    emptyState: 'Пока нет геометрии — напишите OpenSCAD или загрузите пример',
    // Build plate
    buildPlate: 'Стол печати',
    buildPlateX: 'X',
    buildPlateZ: 'Z',
    // Statistics
    statistics: 'Статистика',
    statsTriangles: 'Треугольники',
    statsVertices: 'Вершины',
    statsVolume: 'Объём',
    statsSurfaceArea: 'Площадь поверхности',
    statsBoundingBox: 'Габариты',
    cmdStatistics: 'Статистика модели',
    ariaStatistics: 'Статистика модели',
    sc_buildPlate: 'Стол печати (15° привязка: Ctrl+вращение)',
    sc_orbitSnap: 'Привязка вращения к 15°',
    // PNG export scale
    exportPng: 'Экспорт PNG',
    pngScale: 'Масштаб',
    cmdExportPng2x: 'Экспорт PNG (2×)',
    cmdExportPng4x: 'Экспорт PNG (4×)',
    ariaPngScale: 'Масштаб PNG',
    // Viewport menus
    menuView: 'Вид',
    menuRender: 'Рендер',
    menuExport: 'Экспорт',
    // Camera bookmarks
    bookmarks: 'Закладки',
    saveBookmark: 'Сохранить позицию',
    noBookmarks: 'Нет закладок',
    deleteBookmark: 'Удалить',
    bookmarkSaved: 'Позиция сохранена',
    // STL import
    importStl: 'Импорт STL',
    importStlDrop: 'Перетащите .stl файл сюда',
    importedStl: 'Импортировано',
    cmdImportSTL: 'Импорт STL',
    // Duplicate line / Move line
    sc_duplicateLine: 'Дублировать строку / выделение',
    sc_moveLineUp: 'Переместить строку вверх',
    sc_moveLineDown: 'Переместить строку вниз',
    // Tab navigation
    sc_nextTab: 'Следующая вкладка',
    sc_prevTab: 'Предыдущая вкладка',
    sc_closeTab: 'Закрыть вкладку',
    // Pin tab
    pinTab: 'Закрепить',
    unpinTab: 'Открепить',
    closeOtherTabs: 'Закрыть остальные',
    // Font selector
    fontFamily: 'Шрифт редактора',
    // Hover docs
    hoverDocs: 'Подсказки при наведении',
    // Occurrence highlighting
    occurrences: 'Совпадений',
    // Responsive topbar
    menu: 'Меню',
    // Advanced examples
    mechanical: 'Механизм',
    mechanicalTip: 'Шестерня + призма + массив + капсула',
    honeycomb: 'Соты',
    honeycombTip: 'Сотовая решётка',
    springCoil: 'Пружина',
    springCoilTip: 'Спиральная пружина',
    knurledCylinder: 'Накатка',
    knurledCylinderTip: 'Цилиндр с накаткой',
    chamferedBox: 'Фаска',
    chamferedBoxTip: 'Куб с фасками',
    loftShape: 'Лофт',
    loftShapeTip: 'Лофт между профилями',
    staircase: 'Лестница',
    paramVase: 'Парам. ваза',
    paramGear: 'Парам. шестерня',
    staircaseTip: 'Винтовая лестница',
    paramVaseTip: 'Параметрическая ваза (модуль + цикл)',
    paramGearTip: 'Параметрическая шестерня (модуль + цикл)',
    // Breadcrumbs
    breadcrumbScope: 'Область',
    // Error recovery
    partialRender: 'Частичный рендер с ошибками',
    // Code folding / Color picker / use-include
    codeFolding: 'Сворачивание кода',
    colorPicker: 'Палитра цветов',
    useInclude: 'use/include между вкладками',
    assertFailed: 'Ошибка assert',
    // Render modes
    renderMode: 'Режим рендера',
    modeSolid: 'Сплошной',
    modeSolidEdges: 'Сплошной + рёбра',
    modeWireframe: 'Каркас',
    modeXray: 'Рентген',
    // Flat shading
    flatShading: 'Плоское затенение',
    // SSAO
    ssao: 'Окклюзия (SSAO)',
    // Outline
    outline: 'Контур (силуэт)',
    // Normal smoothing
    smoothNormals: 'Сглаживание нормалей',
    // Color grading
    colorGrading: 'Цветокоррекция',
    cgNone: 'Нет',
    cgWarm: 'Тёплый',
    cgCool: 'Холодный',
    cgVintage: 'Винтаж',
    cgNoir: 'Нуар',
    cgVivid: 'Насыщенный',
    // Sky preset
    skyPreset: 'Небо / Фон',
    skyNone: 'Нет',
    skyClearSky: 'Ясное небо',
    skySunset: 'Закат',
    skyStudio: 'Студия',
    skyNeutral: 'Нейтральный',
    // Clip axis
    clipAxisLabel: 'Ось сечения',
    // 3MF export
    export3mf: 'Экспорт 3MF',
    cmdExport3MF: 'Экспорт 3MF',
    // Device lost
    deviceLost: 'GPU устройство потеряно',
    deviceLostMsg: 'Соединение с GPU потеряно. Попробуйте переинициализировать.',
    reinitialize: 'Переинициализировать',
    reinitSuccess: 'WebGPU переинициализирован',
    reinitFailed: 'Не удалось переинициализировать WebGPU',
    // Performance panel
    perfPanel: 'Производительность',
    perfParseTime: 'Разбор',
    perfMeshGenTime: 'Генерация мешей',
    perfGpuUploadTime: 'Загрузка на GPU',
    perfTriangles: 'Треугольники',
    perfVertices: 'Вершины',
    perfVertexBuffer: 'Буфер вершин',
    perfIndexBuffer: 'Буфер индексов',
    perfGpuAdapter: 'GPU адаптер',
    perfGpuVendor: 'Производитель',
    perfMaxBuffer: 'Макс. буфер',
    perfMaxTexture: 'Макс. текстура',
    cmdPerfPanel: 'Панель производительности',
    // Shortcuts categories
    shortcatEditor: 'Редактор',
    shortcatNavigation: 'Навигация',
    shortcatView: 'Вид',
    shortcatFile: 'Файлы',
    printShortcuts: 'Печать',
    // Undo/redo
    sc_undo_custom: 'Отменить',
    sc_redo: 'Повторить',
    // Diff mode
    showChanges: 'Показать изменения',
    savedState: 'Сохранённая версия',
    linesAdded: 'Добавлено строк',
    linesRemoved: 'Удалено строк',
    linesModified: 'Изменено строк',
    noChanges: 'Нет изменений',
    unsaved: 'Не сохранено',
    cmdToggleDiff: 'Переключить изменения',
    // Split editor
    splitEditor: 'Разделить редактор',
    splitEditorTip: 'Разделить редактор на два панели',
    rightPaneTab: 'Вкладка правой панели',
    // Version history
    history: 'История',
    historyTitle: 'История изменений',
    historyEmpty: 'Нет сохранённых версий',
    historyRestore: 'Восстановить',
    historyPreview: 'Просмотр',
    historyEntry: '{n} символов',
    cmdToggleHistory: 'Переключить историю',
    ariaHistory: 'История изменений',
    // Progressive disclosure
    simpleMode: 'Простой',
    advancedMode: 'Расширенный',
    modeToggle: 'Режим интерфейса',
    // Shortcut presets
    shortcutPreset: 'Набор клавиш',
    presetDefault: 'По умолчанию',
    presetVSCode: 'VS Code',
    presetSublime: 'Sublime',
    presetEmacs: 'Emacs',
    // Example gallery
    exampleGallery: 'Галерея примеров',
    exampleGallerySearch: 'Поиск примеров...',
    loadExample: 'Загрузить',
    cmdExampleGallery: 'Галерея примеров',
    ariaExampleGallery: 'Галерея примеров',
    // Snippets
    snippets: 'Сниппеты',
    cmdInsertSnippet: 'Вставить сниппет',
    snippetCenteredCube: 'Центрированный куб',
    snippetCylinderHole: 'Цилиндр с отверстием',
    snippetRoundedBox: 'Скруглённый куб',
    snippetArrayPattern: 'Массив объектов',
    snippetParametricModule: 'Параметрический модуль',
    snippetThreadedInsert: 'Резьбовая вставка',
    // Code statistics
    codeStats: 'Статистика кода',
    codeLines: 'Строк',
    codeChars: 'Символов',
    codeWords: 'Слов',
    codeModules: 'Модулей',
    codeForLoops: 'Циклов for',
    codeNestingDepth: 'Глубина вложенности',
    // Camera info
    cameraInfo: 'Камера',
    cameraYaw: 'Поворот',
    cameraPitch: 'Наклон',
    cameraDist: 'Расстояние',
    cameraTarget: 'Цель',
    showCameraInfo: 'Показать камеру',
    // Export dropdown
    exportDropdown: 'Экспорт',
    exportScad: 'Экспорт .scad',
    // Responsive
    resetView: 'Сбросить вид',
    // OpenSCAD Reference
    scadReference: 'Справочник OpenSCAD',
    cmdScadReference: 'Справочник OpenSCAD',
    ariaScadReference: 'Справочник OpenSCAD',
    refPrimitives3D: '3D-примитивы',
    refPrimitives2D: '2D-примитивы',
    refTransforms: 'Трансформации',
    refCSG: 'Булевы операции (CSG)',
    refExtrusions: 'Экструзии',
    refMathFunctions: 'Математические функции',
    refSpecialVars: 'Специальные переменные',
    refControlFlow: 'Управление потоком',
    refOther: 'Прочее',
    // Measurement tool
    measureMode: 'Измерение',
    measureClear: 'Очистить',
    measureDistance: 'Расстояние',
    cmdToggleMeasure: 'Переключить измерение',
    measureHint: 'Кликните две точки для измерения расстояния',
    // Ghost comparison
    ghostCompare: 'Призрак',
    ghostTab: 'Вкладка-призрак',
    cmdToggleGhost: 'Переключить призрак',
    noOtherTabs: 'Нет других вкладок',
    // What's New
    whatsNew: 'Что нового',
    whatsNewTitle: 'Что нового',
    // Print Code
    printCode: 'Печать кода',
    cmdPrintCode: 'Печать кода',
    // 3D Annotations
    annotations: 'Аннотации',
    annotationsHint: 'annotation("текст", [x,y,z])',
    // Bookmark thumbnails
    bookmarkThumbnail: 'Предпросмотр',
    // More snippets
    snippetHexGrid: 'Шестиугольная сетка',
    snippetBevel: 'Фаска',
    snippetSpring: 'Пружина',
    snippetBoxWithLid: 'Коробка с крышкой',
    snippetGearWheel: 'Зубчатое колесо',
    // Watermark
    watermark: 'Водяной знак',
    watermarkText: 'Текст',
    watermarkPosition: 'Позиция',
    watermarkOpacity: 'Прозрачность',
    wmTopLeft: 'Верх-лево',
    wmTopRight: 'Верх-право',
    wmBottomLeft: 'Низ-лево',
    wmBottomRight: 'Низ-право',
    // Session recovery
    sessionRestore: 'Восстановить сессию?',
    sessionRestoreMsg: 'Найдена предыдущая сессия. Восстановить?',
    sessionRestoreYes: 'Восстановить',
    sessionRestoreNo: 'Отклонить',
    // Batch export
    exportAllTabs: 'Экспорт всех вкладок (ZIP)',
    cmdExportAllTabs: 'Экспорт всех вкладок',
    colorPalette: 'Палитра цветов',
    perfWarning: 'Модель сложная ({n} треугольников). Уменьшите $fn для ускорения.',
    slowRenderHint: 'Рендер занял {ms}мс. Попробуйте уменьшить $fn.',
    fastPreview: 'Быстрый просмотр',
    fastPreviewOn: 'Быстрый просмотр включён ($fn уменьшен вдвое)',
    fastPreviewOff: 'Быстрый просмотр выключен',
    cmdToggleFastPreview: 'Переключить быстрый просмотр',
    // Ghost comparison stats
    ghostStats: 'Сравнение моделей',
    ghostDeltaTriangles: 'Δ Треугольники',
    ghostDeltaVolume: 'Δ Объём',
    ghostDeltaSize: 'Δ Размер',
    ghostOverlayMode: 'Режим наложения',
    ghostTransparent: 'Прозрачный',
    ghostSideBySide: 'Рядом',
    ghostDifference: 'Различие',
    historyTimeline: 'Временная шкала',
    historyCurrent: 'Текущая',
    // Presets
    presets: 'Пресеты',
    savePreset: 'Сохранить пресет',
    loadPreset: 'Загрузить',
    deletePreset: 'Удалить',
    presetName: 'Имя пресета',
    presetSaved: 'Пресет сохранён',
    presetLoaded: 'Пресет загружен',
    presetDeleted: 'Пресет удалён',
    builtInPresets: 'Встроенные',
    userPresets: 'Пользовательские',
    presetCadPro: 'CAD профессионал',
    presetPrintPreview: '3D печать',
    presetPresentation: 'Презентация',
    // Welcome Tour
    tourStart: 'Интерактивный тур',
    tourSkip: 'Пропустить',
    tourNext: 'Далее',
    tourPrev: 'Назад',
    tourFinish: 'Готово',
    tourStep1: 'Редактор — здесь вы пишете код OpenSCAD',
    tourStep2: 'Кнопка рендера — нажмите для обновления 3D-вида',
    tourStep3: 'Примеры — загрузите готовые модели',
    tourStep4: '3D-вьюпорт — вращайте, масштабируйте, перемещайте модель',
    tourStep5: 'Панель управления видом — режимы рендера и настройки',
    tourStep6: 'Экспорт — сохраните модель в STL, OBJ или 3MF',
    // Numpad navigation
    sc_numpad: 'Numpad 1-9: Виды камеры',
    sc_fitAll: 'F / Numpad.: Вместить всё',
    sc_homeView: 'Home: Сбросить вид',
    // Undo/Redo visualization
    undoBtn: 'Отменить',
    redoBtn: 'Повторить',
    undoSteps: '{n} шагов отмены',
    redoSteps: '{n} шагов повтора',
    // Gutter decorations
    gutterDecorations: 'Декорации строк',
    // Snapshot gallery
    snapshotBtn: 'Снимок',
    snapshotGallery: 'Галерея снимков',
    snapshotSaved: 'Снимок сохранён',
    snapshotDeleted: 'Снимок удалён',
    noSnapshots: 'Нет сохранённых снимков',
    deleteSnapshot: 'Удалить',
    cmdSnapshotGallery: 'Галерея снимков',
    ariaSnapshotGallery: 'Галерея снимков',
    // Code profiling
    profilePanel: 'Профилирование',
    profileNode: 'Узел',
    profileLine: 'Строка',
    profileTime: 'Время',
    cmdToggleProfile: 'Переключить профилирование',
    noProfileData: 'Нет данных профилирования',
    // New features (batch 32)
    refSearch: 'Поиск по справочнику…',
    complexity: 'Сложность',
    complexitySimple: 'Простая',
    complexityMedium: 'Средняя',
    complexityComplex: 'Сложная',
    compassN: 'С',
    compassS: 'Ю',
    compassE: 'В',
    compassW: 'З',
    // Batch 34: editor & rendering improvements
    toonShading: 'Тун-шейдинг',
    cmdToggleToon: 'Переключить тун-шейдинг',
    explodedView: 'Разнесённый вид',
    explodeFactor: 'Степень разнесения',
    vignette: 'Виньетка',
    cmdToggleVignette: 'Переключить виньетку',
    fovSlider: 'Угол обзора',
    orbitInertia: 'Инерция орбиты',
    cmdToggleInertia: 'Переключить инерцию орбиты',
    sortLinesAsc: 'Сортировка строк (по возрастанию)',
    sortLinesDesc: 'Сортировка строк (по убыванию)',
    toggleUpperCase: 'ВЕРХНИЙ РЕГИСТР',
    toggleLowerCase: 'нижний регистр',
    joinLines: 'Объединить строки',
    sc_joinLines: 'Объединить строки',
    sc_upperCase: 'ВЕРХНИЙ РЕГИСТР',
    sc_lowerCase: 'нижний регистр',
    goochShading: 'Шейдинг Гуча',
    cmdToggleGooch: 'Переключить шейдинг Гуча',
    modeHiddenLine: 'Скрытые линии',
    groundShadow: 'Тень на земле',
    cmdToggleGroundShadow: 'Переключить тень на земле',
    exportTurntable: 'Экспорт вращения (ZIP)',
    turntableProgress: 'Экспорт кадров...',
    turntableComplete: 'Вращение экспортировано',
    zenMode: 'Режим Дзен',
    cmdToggleZen: 'Переключить режим Дзен',
    exitZenMode: 'Выйти из режима Дзен',
    stickyScroll: 'Область кода',
    tipOfDay: 'Совет дня',
    dontShowTips: 'Больше не показывать',
    meshVisible: 'Видимый',
    meshHidden: 'Скрытый',
    // Batch 36
    flyCamera: 'Режим полёта',
    cmdToggleFly: 'Переключить режим полёта',
    bloom: 'Свечение (Bloom)',
    cmdToggleBloom: 'Переключить свечение',
    sectionBox: 'Сечение коробкой',
    cmdToggleSectionBox: 'Переключить сечение коробкой',
    colorOverride: 'Цвет объекта',
    resetColor: 'Сбросить цвет',
    orbitMode: 'Орбита',
    flyModeLabel: 'Полёт',
    // Batch 38
    multiReplace: 'Множественная замена',
    multiReplaceActive: 'Замена {n} совпадений',
    multiReplaceHint: 'Ctrl+D: выделить слово / заменить все совпадения',
    sc_multiReplace: 'Выделить все совпадения',
    duplicateTab: 'Дублировать вкладку',
    copyOf: 'Копия {name}',
    tabColor: 'Цвет вкладки',
    tabColorNone: 'Без цвета',
    tabColorRed: 'Красный',
    tabColorGreen: 'Зелёный',
    tabColorBlue: 'Синий',
    tabColorYellow: 'Жёлтый',
    tabColorPurple: 'Фиолетовый',
    quickSwitcher: 'Быстрое переключение',
    quickSwitcherPlaceholder: 'Поиск вкладки...',
    noTabsFound: 'Вкладки не найдены',
    notificationCenter: 'Центр уведомлений',
    clearAllNotifications: 'Очистить все',
    noNotifications: 'Нет уведомлений',
    embedMode: 'Режим встраивания',
    specSheet: 'Спецификация',
    cmdSpecSheet: 'Создать спецификацию',
    specModelName: 'Название модели',
    specDimensions: 'Габариты',
    specVolume: 'Объём',
    specSurfaceArea: 'Площадь поверхности',
    specTriangles: 'Треугольники',
    specMaterialEst: 'Оценка материала',
    specSourceCode: 'Исходный код',
    specGenerated: 'Дата создания',
    specDensity: 'Плотность (г/см³)',
    specWeight: 'Масса (г)',
    uiDensity: 'Плотность интерфейса',
    densityCompact: 'Компактный',
    densityNormal: 'Обычный',
    densityComfortable: 'Просторный',
    sc_quickSwitcher: 'Быстрое переключение вкладок',
    renderAllTabs: 'Рендер всех вкладок',
    batchRendering: 'Пакетный рендер...',
    batchProgress: 'Прогресс: {n}/{total}',
    batchResults: 'Результаты пакетного рендера',
    batchTabName: 'Вкладка',
    batchTriangles: 'Треугольники',
    batchTime: 'Время (мс)',
    batchTotal: 'Итого',
    cmdRenderAll: 'Рендер всех вкладок',
    customizeShortcuts: 'Настроить горячие клавиши',
    shortcutEditorTitle: 'Редактор горячих клавиш',
    shortcutAction: 'Действие',
    shortcutBinding: 'Клавиша',
    shortcutDefault: 'По умолчанию',
    shortcutEdit: 'Изменить',
    shortcutReset: 'Сброс',
    shortcutResetAll: 'Сбросить все',
    shortcutCapturing: 'Нажмите комбинацию клавиш...',
    shortcutCancel: 'Отмена',
    screenshotMetadata: 'Метаданные скриншота',
    metaModelName: 'Название модели',
    metaDimensions: 'Габариты',
    metaTriangles: 'Треугольники',
    metaDate: 'Дата',
    // Batch 40 — New primitive examples
    latticeEx: 'Решётка',
    latticeTip: 'Кубическая решётка',
    slotCrossEx: 'Слот и крест',
    slotCrossTip: 'Слот (стадион) и крест',
    mazeEx: 'Лабиринт',
    mazeTip: 'Случайный лабиринт',
    fibSphereEx: 'Сфера Фибоначчи',
    fibSphereTip: 'Точки по сфере Фибоначчи',
    // Batch 40 — Theme Editor
    customTheme: 'Пользовательская тема',
    customThemeName: 'Название темы',
    themeBackground: 'Фон',
    themeSurface: 'Поверхность',
    themeBorder: 'Граница',
    themeText: 'Текст',
    themeAccent: 'Акцент',
    themeSave: 'Сохранить тему',
    themeDelete: 'Удалить',
    themeNamePlaceholder: 'Моя тема',
    themeSaved: 'Тема сохранена',
    themeDeleted: 'Тема удалена',
    // Batch 40 — Playground
    playground: 'Площадка',
    playgroundTitle: 'Интерактивная площадка',
    playgroundChallenge: 'Задание {n}',
    playgroundGoal: 'Цель',
    playgroundHint: 'Подсказка',
    playgroundCheck: 'Проверить',
    playgroundSuccess: 'Отлично! Задание пройдено!',
    playgroundFail: 'Пока не получилось. Попробуйте ещё раз.',
    playgroundNext: 'Следующее задание',
    playgroundComplete: 'Все задания выполнены! Поздравляем!',
    playgroundReset: 'Начать заново',
    cmdPlayground: 'Площадка (обучение)',
    // Batch 40 — Error Explanation
    errorExplanation: 'Пояснение',
    errorFixHint: 'Возможное исправление',
    showErrorHelp: 'Помощь',
    // Batch 42 — Weight/Cost Estimator
    material: 'Материал',
    weight: 'Вес',
    cost: 'Стоимость',
    materialPLA: 'PLA',
    materialABS: 'ABS',
    materialPETG: 'PETG',
    materialNylon: 'Нейлон',
    materialResin: 'Смола',
    costPerKg: 'Цена за кг',
    currency: 'Валюта',
    // Batch 42 — About Dialog
    about: 'О программе',
    aboutTitle: 'О программе',
    aboutVersion: 'Версия',
    aboutBuiltWith: 'Создано с помощью Vue 3, WebGPU, TypeScript',
    aboutLicense: 'Лицензия: MIT',
    // Batch 42 — Screenshot Comparison
    compareScreenshots: 'Сравнение скриншотов',
    saveReference: 'Сохранить эталон',
    compareWithReference: 'Сравнить с эталоном',
    referenceNotSaved: 'Сначала сохраните эталонный скриншот',
    overlay: 'Наложение',
    sideBySide: 'Рядом',
    difference: 'Разница',
    cmdAbout: 'О программе',
  },
  en: {
    title: 'OpenSCAD 3D Viewer',
    subtitle: 'OpenSCAD editor with WebGPU rendering',
    render: 'Render', auto: 'Auto', examples: 'Examples',
    basic: 'Basic', csg: 'CSG', house: 'House', tower: 'Tower',
    meshes: 'Meshes', triangles: 'Triangles',
    hint: 'LMB: rotate · Ctrl+LMB: 15° snap · RMB/Shift: pan · wheel: zoom · Ctrl+Enter: render',
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
    shareInvalid: 'Invalid share link',
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
    preferences: 'Preferences',
    fontSize: 'Font Size',
    tabSize: 'Tab Size',
    autoRenderDelay: 'Auto-render Delay',
    showMinimapPref: 'Minimap',
    showLineNumbers: 'Line Numbers',
    console: 'Console',
    consoleClear: 'Clear',
    copyImage: 'Copy',
    copiedImage: 'Copied!',
    objectTree: 'Object Tree',
    parsedNodes: 'Parsed {n} nodes',
    generatedMeshes: 'Generated {m} meshes, {t} triangles in {ms}ms',
    sc_commentToggle: 'Toggle comment',
    commandPalette: 'Command Palette',
    goToLine: 'Go to Line',
    goToLinePlaceholder: 'Go to line (1-{n})',
    wordWrap: 'Word Wrap',
    selChars: 'Sel: {x} chars, {y} lines',
    animationTimeline: 'Animation',
    animPlay: 'Play',
    animPause: 'Pause',
    animDuration: 'Duration',
    parameters: 'Parameters',
    noParameters: 'No numeric variables',
    lighting: 'Lighting',
    lightDefault: 'Default',
    lightStudio: 'Studio',
    lightOutdoor: 'Outdoor',
    lightDramatic: 'Dramatic',
    lightSoft: 'Soft',
    clipPlane: 'Clip Plane',
    fog: 'Fog',
    reflection: 'Reflection',
    sc_commandPalette: 'Command Palette',
    sc_goToLine: 'Go to Line',
    sc_wordWrap: 'Word Wrap',
    cmdRender: 'Render',
    cmdFormatCode: 'Format Code',
    cmdToggleWireframe: 'Toggle Wireframe',
    cmdToggleGrid: 'Toggle Grid',
    cmdToggleAutoRotate: 'Toggle Auto-rotate',
    cmdToggleFullscreen: 'Toggle Fullscreen',
    cmdToggleOrtho: 'Toggle Ortho',
    cmdTakeScreenshot: 'Take Screenshot',
    cmdCopyImage: 'Copy as Image',
    cmdExportSTL: 'Export STL',
    cmdOpenFile: 'Open File',
    cmdSaveFile: 'Save File',
    cmdShare: 'Share Link',
    cmdToggleTheme: 'Toggle Theme',
    cmdToggleLang: 'Toggle Language',
    cmdShowShortcuts: 'Show Shortcuts',
    cmdPreferences: 'Preferences',
    cmdToggleWordWrap: 'Toggle Word Wrap',
    cmdGoToLine: 'Go to Line',
    cmdToggleMinimap: 'Toggle Minimap',
    cmdToggleConsole: 'Toggle Console',
    cmdToggleObjectTree: 'Toggle Object Tree',
    cmdNewTab: 'New Tab',
    exportObj: 'Export OBJ',
    cmdExportOBJ: 'Export OBJ',
    gear: 'Gear',
    vase: 'Vase',
    chess: 'Pawn',
    gearTip: 'Toothed gear',
    vaseTip: 'Curved vase',
    chessTip: 'Chess pawn',
    basicTip: 'Basic shapes',
    csgTip: 'Boolean ops',
    houseTip: 'Simple house',
    towerTip: 'Decorative tower',
    bgGradDark: 'Dark Gradient',
    bgGradBlue: 'Blue Gradient',
    bgGradSunset: 'Sunset Gradient',
    sc_wasd: 'Camera: forward/back/orbit',
    sc_shiftWasd: 'Camera pan',
    sc_qe: 'Camera: pitch up/down',
    initWebGPU: 'Initializing WebGPU…',
    gizmoTip: 'Click an axis to look down it',
    ctxResetView: 'Reset View',
    ctxFitScreen: 'Fit to Screen',
    ctxScreenshot: 'Screenshot',
    ctxCopyImage: 'Copy Image',
    ctxToggleWireframe: 'Toggle Wireframe',
    ctxToggleGrid: 'Toggle Grid',
    ctxExportStl: 'Export STL',
    statMeshes: 'Meshes',
    statTriangles: 'Tris',
    statVertices: 'Vertices',
    statFps: 'FPS',
    statRender: 'Render',
    statSize: 'Size',
    resetPrefs: 'Reset to defaults',
    prefsReset: 'Preferences reset',
    undo: 'Undo',
    tabClosed: 'Tab closed',
    tabRestored: 'Tab restored',
    // Accessibility (aria) labels
    ariaHelp: 'Keyboard shortcuts',
    ariaPreferences: 'Preferences',
    ariaToggleLang: 'Toggle language',
    ariaThemeSelector: 'Select theme',
    ariaCloseModal: 'Close',
    ariaShortcutsDialog: 'Keyboard shortcuts',
    ariaPreferencesDialog: 'Preferences',
    ariaCommandPaletteDialog: 'Command palette',
    ariaGoToLineDialog: 'Go to line',
    ariaGizmo: 'Navigation cube',
    ariaConsole: 'Toggle console',
    ariaMinimap: 'Toggle minimap',
    ariaNewTab: 'New tab',
    ariaAnimPlay: 'Play/pause animation',
    // Welcome / onboarding
    welcomeTitle: 'OpenSCAD 3D Viewer',
    welcomeTagline: 'Write OpenSCAD and see it rendered in real time with WebGPU.',
    welcomeFeat1: 'Real-time OpenSCAD editing',
    welcomeFeat2: 'Fast WebGPU 3D rendering',
    welcomeFeat3: 'Export to STL / OBJ / 3MF',
    welcomeFeat4: 'Editor themes & keyboard shortcuts',
    welcomeStart: 'Get Started',
    showWelcome: 'Show Welcome',
    cmdShowWelcome: 'Show Welcome',
    ariaWelcomeDialog: 'Welcome',
    // Empty state
    emptyState: 'No geometry yet — write some OpenSCAD or load an example',
    // Build plate
    buildPlate: 'Build Plate',
    buildPlateX: 'X',
    buildPlateZ: 'Z',
    // Statistics
    statistics: 'Statistics',
    statsTriangles: 'Triangles',
    statsVertices: 'Vertices',
    statsVolume: 'Volume',
    statsSurfaceArea: 'Surface Area',
    statsBoundingBox: 'Bounding Box',
    cmdStatistics: 'Model Statistics',
    ariaStatistics: 'Model statistics',
    sc_buildPlate: 'Build plate (15° snap: Ctrl+orbit)',
    sc_orbitSnap: 'Snap orbit to 15°',
    // PNG export scale
    exportPng: 'Export PNG',
    pngScale: 'Scale',
    cmdExportPng2x: 'Export PNG (2×)',
    cmdExportPng4x: 'Export PNG (4×)',
    ariaPngScale: 'PNG scale',
    // Viewport menus
    menuView: 'View',
    menuRender: 'Render',
    menuExport: 'Export',
    // Camera bookmarks
    bookmarks: 'Bookmarks',
    saveBookmark: 'Save Position',
    noBookmarks: 'No bookmarks',
    deleteBookmark: 'Delete',
    bookmarkSaved: 'Position saved',
    // STL import
    importStl: 'Import STL',
    importStlDrop: 'Drop .stl file here',
    importedStl: 'Imported',
    cmdImportSTL: 'Import STL',
    // Duplicate line / Move line
    sc_duplicateLine: 'Duplicate line / selection',
    sc_moveLineUp: 'Move line up',
    sc_moveLineDown: 'Move line down',
    // Tab navigation
    sc_nextTab: 'Next tab',
    sc_prevTab: 'Previous tab',
    sc_closeTab: 'Close tab',
    // Pin tab
    pinTab: 'Pin',
    unpinTab: 'Unpin',
    closeOtherTabs: 'Close Others',
    // Font selector
    fontFamily: 'Editor Font',
    // Hover docs
    hoverDocs: 'Hover Tooltips',
    // Occurrence highlighting
    occurrences: 'Occurrences',
    // Responsive topbar
    menu: 'Menu',
    // Advanced examples
    mechanical: 'Mechanical',
    mechanicalTip: 'Gear + prism + array + capsule',
    honeycomb: 'Honeycomb',
    honeycombTip: 'Hexagonal grid',
    springCoil: 'Spring',
    springCoilTip: 'Coil spring',
    knurledCylinder: 'Knurl',
    knurledCylinderTip: 'Knurled cylinder',
    chamferedBox: 'Chamfer',
    chamferedBoxTip: 'Chamfered cube',
    loftShape: 'Loft',
    loftShapeTip: 'Loft between profiles',
    staircase: 'Staircase',
    paramVase: 'Param Vase',
    paramGear: 'Param Gear',
    staircaseTip: 'Spiral staircase',
    paramVaseTip: 'Parametric vase (module + loop)',
    paramGearTip: 'Parametric gear (module + loop)',
    // Breadcrumbs
    breadcrumbScope: 'Scope',
    // Error recovery
    partialRender: 'Partial render with errors',
    // Code folding / Color picker / use-include
    codeFolding: 'Code Folding',
    colorPicker: 'Color Picker',
    useInclude: 'use/include across tabs',
    assertFailed: 'Assert failed',
    // Render modes
    renderMode: 'Render Mode',
    modeSolid: 'Solid',
    modeSolidEdges: 'Solid + Edges',
    modeWireframe: 'Wireframe',
    modeXray: 'X-Ray',
    // Flat shading
    flatShading: 'Flat Shading',
    // SSAO
    ssao: 'Ambient Occlusion',
    // Outline
    outline: 'Outline',
    // Normal smoothing
    smoothNormals: 'Smooth Normals',
    // Color grading
    colorGrading: 'Color Grading',
    cgNone: 'None',
    cgWarm: 'Warm',
    cgCool: 'Cool',
    cgVintage: 'Vintage',
    cgNoir: 'Noir',
    cgVivid: 'Vivid',
    // Sky preset
    skyPreset: 'Sky / Background',
    skyNone: 'None',
    skyClearSky: 'Clear Sky',
    skySunset: 'Sunset',
    skyStudio: 'Studio',
    skyNeutral: 'Neutral',
    // Clip axis
    clipAxisLabel: 'Clip Axis',
    // 3MF export
    export3mf: 'Export 3MF',
    cmdExport3MF: 'Export 3MF',
    // Device lost
    deviceLost: 'GPU Device Lost',
    deviceLostMsg: 'GPU connection was lost. Try to reinitialize.',
    reinitialize: 'Reinitialize',
    reinitSuccess: 'WebGPU reinitialized',
    reinitFailed: 'Failed to reinitialize WebGPU',
    // Performance panel
    perfPanel: 'Performance',
    perfParseTime: 'Parse',
    perfMeshGenTime: 'Mesh Gen',
    perfGpuUploadTime: 'GPU Upload',
    perfTriangles: 'Triangles',
    perfVertices: 'Vertices',
    perfVertexBuffer: 'Vertex Buffer',
    perfIndexBuffer: 'Index Buffer',
    perfGpuAdapter: 'GPU Adapter',
    perfGpuVendor: 'Vendor',
    perfMaxBuffer: 'Max Buffer',
    perfMaxTexture: 'Max Texture',
    cmdPerfPanel: 'Performance Panel',
    // Shortcuts categories
    shortcatEditor: 'Editor',
    shortcatNavigation: 'Navigation',
    shortcatView: 'View',
    shortcatFile: 'File',
    printShortcuts: 'Print',
    // Undo/redo
    sc_undo_custom: 'Undo',
    sc_redo: 'Redo',
    // Diff mode
    showChanges: 'Show Changes',
    savedState: 'Saved State',
    linesAdded: 'Lines added',
    linesRemoved: 'Lines removed',
    linesModified: 'Lines modified',
    noChanges: 'No changes',
    unsaved: 'Unsaved',
    cmdToggleDiff: 'Toggle Changes',
    // Split editor
    splitEditor: 'Split Editor',
    splitEditorTip: 'Split editor into two panes',
    rightPaneTab: 'Right pane tab',
    // Version history
    history: 'History',
    historyTitle: 'Version History',
    historyEmpty: 'No saved versions',
    historyRestore: 'Restore',
    historyPreview: 'Preview',
    historyEntry: '{n} chars',
    cmdToggleHistory: 'Toggle History',
    ariaHistory: 'Version history',
    // Progressive disclosure
    simpleMode: 'Simple',
    advancedMode: 'Advanced',
    modeToggle: 'UI Mode',
    // Shortcut presets
    shortcutPreset: 'Key Preset',
    presetDefault: 'Default',
    presetVSCode: 'VS Code',
    presetSublime: 'Sublime',
    presetEmacs: 'Emacs',
    // Example gallery
    exampleGallery: 'Example Gallery',
    exampleGallerySearch: 'Search examples...',
    loadExample: 'Load',
    cmdExampleGallery: 'Example Gallery',
    ariaExampleGallery: 'Example gallery',
    // Snippets
    snippets: 'Snippets',
    cmdInsertSnippet: 'Insert Snippet',
    snippetCenteredCube: 'Centered Cube',
    snippetCylinderHole: 'Cylinder with Hole',
    snippetRoundedBox: 'Rounded Box',
    snippetArrayPattern: 'Array Pattern',
    snippetParametricModule: 'Parametric Module',
    snippetThreadedInsert: 'Threaded Insert',
    // Code statistics
    codeStats: 'Code Stats',
    codeLines: 'Lines',
    codeChars: 'Chars',
    codeWords: 'Words',
    codeModules: 'Modules',
    codeForLoops: 'For loops',
    codeNestingDepth: 'Nesting depth',
    // Camera info
    cameraInfo: 'Camera',
    cameraYaw: 'Yaw',
    cameraPitch: 'Pitch',
    cameraDist: 'Distance',
    cameraTarget: 'Target',
    showCameraInfo: 'Show Camera',
    // Export dropdown
    exportDropdown: 'Export',
    exportScad: 'Export .scad',
    // Responsive
    resetView: 'Reset View',
    // OpenSCAD Reference
    scadReference: 'OpenSCAD Reference',
    cmdScadReference: 'OpenSCAD Reference',
    ariaScadReference: 'OpenSCAD reference',
    refPrimitives3D: '3D Primitives',
    refPrimitives2D: '2D Primitives',
    refTransforms: 'Transforms',
    refCSG: 'Boolean Operations (CSG)',
    refExtrusions: 'Extrusions',
    refMathFunctions: 'Math Functions',
    refSpecialVars: 'Special Variables',
    refControlFlow: 'Control Flow',
    refOther: 'Other',
    // Measurement tool
    measureMode: 'Measure',
    measureClear: 'Clear',
    measureDistance: 'Distance',
    cmdToggleMeasure: 'Toggle Measure',
    measureHint: 'Click two points to measure distance',
    // Ghost comparison
    ghostCompare: 'Ghost',
    ghostTab: 'Ghost Tab',
    cmdToggleGhost: 'Toggle Ghost',
    noOtherTabs: 'No other tabs',
    // What's New
    whatsNew: "What's New",
    whatsNewTitle: "What's New",
    // Print Code
    printCode: 'Print Code',
    cmdPrintCode: 'Print Code',
    // 3D Annotations
    annotations: 'Annotations',
    annotationsHint: 'annotation("text", [x,y,z])',
    // Bookmark thumbnails
    bookmarkThumbnail: 'Preview',
    // More snippets
    snippetHexGrid: 'Hexagonal Grid',
    snippetBevel: 'Beveled Edge',
    snippetSpring: 'Spring / Helix',
    snippetBoxWithLid: 'Box with Lid',
    snippetGearWheel: 'Gear Wheel',
    // Watermark
    watermark: 'Watermark',
    watermarkText: 'Text',
    watermarkPosition: 'Position',
    watermarkOpacity: 'Opacity',
    wmTopLeft: 'Top-left',
    wmTopRight: 'Top-right',
    wmBottomLeft: 'Bottom-left',
    wmBottomRight: 'Bottom-right',
    // Session recovery
    sessionRestore: 'Restore session?',
    sessionRestoreMsg: 'A previous session was found. Restore it?',
    sessionRestoreYes: 'Restore',
    sessionRestoreNo: 'Dismiss',
    // Batch export
    exportAllTabs: 'Export All Tabs (ZIP)',
    cmdExportAllTabs: 'Export All Tabs',
    colorPalette: 'Color Palette',
    perfWarning: 'Complex model ({n} triangles). Reduce $fn for better performance.',
    slowRenderHint: 'Render took {ms}ms. Try reducing $fn.',
    fastPreview: 'Fast Preview',
    fastPreviewOn: 'Fast preview ON ($fn halved)',
    fastPreviewOff: 'Fast preview OFF',
    cmdToggleFastPreview: 'Toggle Fast Preview',
    // Ghost comparison stats
    ghostStats: 'Model Comparison',
    ghostDeltaTriangles: 'Δ Triangles',
    ghostDeltaVolume: 'Δ Volume',
    ghostDeltaSize: 'Δ Size',
    ghostOverlayMode: 'Overlay Mode',
    ghostTransparent: 'Transparent',
    ghostSideBySide: 'Side by Side',
    ghostDifference: 'Difference',
    historyTimeline: 'Timeline',
    historyCurrent: 'Current',
    // Presets
    presets: 'Presets',
    savePreset: 'Save Preset',
    loadPreset: 'Load',
    deletePreset: 'Delete',
    presetName: 'Preset Name',
    presetSaved: 'Preset saved',
    presetLoaded: 'Preset loaded',
    presetDeleted: 'Preset deleted',
    builtInPresets: 'Built-in',
    userPresets: 'User Presets',
    presetCadPro: 'CAD Professional',
    presetPrintPreview: '3D Print Preview',
    presetPresentation: 'Presentation',
    // Welcome Tour
    tourStart: 'Take a Tour',
    tourSkip: 'Skip',
    tourNext: 'Next',
    tourPrev: 'Back',
    tourFinish: 'Finish',
    tourStep1: 'Editor — write your OpenSCAD code here',
    tourStep2: 'Render button — click to update the 3D view',
    tourStep3: 'Examples — load pre-made models',
    tourStep4: '3D Viewport — rotate, zoom, and pan the model',
    tourStep5: 'View controls — render modes and settings',
    tourStep6: 'Export — save your model as STL, OBJ, or 3MF',
    // Numpad navigation
    sc_numpad: 'Numpad 1-9: Camera views',
    sc_fitAll: 'F / Numpad.: Fit all',
    sc_homeView: 'Home: Reset view',
    // Undo/Redo visualization
    undoBtn: 'Undo',
    redoBtn: 'Redo',
    undoSteps: '{n} undo steps',
    redoSteps: '{n} redo steps',
    // Gutter decorations
    gutterDecorations: 'Gutter Decorations',
    // Snapshot gallery
    snapshotBtn: 'Snapshot',
    snapshotGallery: 'Snapshot Gallery',
    snapshotSaved: 'Snapshot saved',
    snapshotDeleted: 'Snapshot deleted',
    noSnapshots: 'No saved snapshots',
    deleteSnapshot: 'Delete',
    cmdSnapshotGallery: 'Snapshot Gallery',
    ariaSnapshotGallery: 'Snapshot gallery',
    // Code profiling
    profilePanel: 'Profiling',
    profileNode: 'Node',
    profileLine: 'Line',
    profileTime: 'Time',
    cmdToggleProfile: 'Toggle Profiling',
    noProfileData: 'No profile data',
    // Batch 34: editor & rendering improvements
    toonShading: 'Toon Shading',
    cmdToggleToon: 'Toggle Toon Shading',
    explodedView: 'Exploded View',
    explodeFactor: 'Explode Factor',
    vignette: 'Vignette',
    cmdToggleVignette: 'Toggle Vignette',
    fovSlider: 'Field of View',
    orbitInertia: 'Orbit Inertia',
    cmdToggleInertia: 'Toggle Orbit Inertia',
    sortLinesAsc: 'Sort Lines Ascending',
    sortLinesDesc: 'Sort Lines Descending',
    toggleUpperCase: 'UPPERCASE',
    toggleLowerCase: 'lowercase',
    joinLines: 'Join Lines',
    sc_joinLines: 'Join Lines',
    sc_upperCase: 'UPPERCASE',
    sc_lowerCase: 'lowercase',
    goochShading: 'Gooch Shading',
    cmdToggleGooch: 'Toggle Gooch Shading',
    modeHiddenLine: 'Hidden Line',
    groundShadow: 'Ground Shadow',
    cmdToggleGroundShadow: 'Toggle Ground Shadow',
    exportTurntable: 'Export Turntable (ZIP)',
    turntableProgress: 'Exporting frames...',
    turntableComplete: 'Turntable exported',
    zenMode: 'Zen Mode',
    cmdToggleZen: 'Toggle Zen Mode',
    exitZenMode: 'Exit Zen Mode',
    stickyScroll: 'Code Scope',
    tipOfDay: 'Tip of the Day',
    dontShowTips: 'Don\'t show again',
    meshVisible: 'Visible',
    meshHidden: 'Hidden',
    // Batch 36
    flyCamera: 'Fly Camera',
    cmdToggleFly: 'Toggle Fly Camera',
    bloom: 'Bloom',
    cmdToggleBloom: 'Toggle Bloom',
    sectionBox: 'Section Box',
    cmdToggleSectionBox: 'Toggle Section Box',
    colorOverride: 'Object Color',
    resetColor: 'Reset Color',
    orbitMode: 'Orbit',
    flyModeLabel: 'Fly',
    // Batch 38
    multiReplace: 'Multi-Replace',
    multiReplaceActive: 'Replacing {n} occurrences',
    multiReplaceHint: 'Ctrl+D: select word / replace all occurrences',
    sc_multiReplace: 'Select All Occurrences',
    duplicateTab: 'Duplicate Tab',
    copyOf: 'Copy of {name}',
    tabColor: 'Tab Color',
    tabColorNone: 'None',
    tabColorRed: 'Red',
    tabColorGreen: 'Green',
    tabColorBlue: 'Blue',
    tabColorYellow: 'Yellow',
    tabColorPurple: 'Purple',
    quickSwitcher: 'Quick Switcher',
    quickSwitcherPlaceholder: 'Search tabs...',
    noTabsFound: 'No tabs found',
    notificationCenter: 'Notification Center',
    clearAllNotifications: 'Clear All',
    noNotifications: 'No notifications',
    embedMode: 'Embed Mode',
    specSheet: 'Spec Sheet',
    cmdSpecSheet: 'Generate Spec Sheet',
    specModelName: 'Model Name',
    specDimensions: 'Dimensions',
    specVolume: 'Volume',
    specSurfaceArea: 'Surface Area',
    specTriangles: 'Triangles',
    specMaterialEst: 'Material Estimate',
    specSourceCode: 'Source Code',
    specGenerated: 'Generated',
    specDensity: 'Density (g/cm3)',
    specWeight: 'Weight (g)',
    uiDensity: 'UI Density',
    densityCompact: 'Compact',
    densityNormal: 'Normal',
    densityComfortable: 'Comfortable',
    sc_quickSwitcher: 'Quick tab switcher',
    renderAllTabs: 'Render All Tabs',
    batchRendering: 'Batch rendering...',
    batchProgress: 'Progress: {n}/{total}',
    batchResults: 'Batch Render Results',
    batchTabName: 'Tab',
    batchTriangles: 'Triangles',
    batchTime: 'Time (ms)',
    batchTotal: 'Total',
    cmdRenderAll: 'Render All Tabs',
    customizeShortcuts: 'Customize Shortcuts',
    shortcutEditorTitle: 'Shortcut Editor',
    shortcutAction: 'Action',
    shortcutBinding: 'Binding',
    shortcutDefault: 'Default',
    shortcutEdit: 'Edit',
    shortcutReset: 'Reset',
    shortcutResetAll: 'Reset All',
    shortcutCapturing: 'Press key combination...',
    shortcutCancel: 'Cancel',
    screenshotMetadata: 'Screenshot Metadata',
    metaModelName: 'Model Name',
    metaDimensions: 'Dimensions',
    metaTriangles: 'Triangles',
    metaDate: 'Date',
    // Batch 40 — New primitive examples
    latticeEx: 'Lattice',
    latticeTip: 'Cubic lattice structure',
    slotCrossEx: 'Slot & Cross',
    slotCrossTip: 'Stadium slot and cross shape',
    mazeEx: 'Maze',
    mazeTip: 'Random maze generator',
    fibSphereEx: 'Fibonacci Sphere',
    fibSphereTip: 'Fibonacci spiral point distribution',
    // Batch 40 — Theme Editor
    customTheme: 'Custom Theme',
    customThemeName: 'Theme Name',
    themeBackground: 'Background',
    themeSurface: 'Surface',
    themeBorder: 'Border',
    themeText: 'Text',
    themeAccent: 'Accent',
    themeSave: 'Save Theme',
    themeDelete: 'Delete',
    themeNamePlaceholder: 'My Theme',
    themeSaved: 'Theme saved',
    themeDeleted: 'Theme deleted',
    // Batch 40 — Playground
    playground: 'Playground',
    playgroundTitle: 'Interactive Playground',
    playgroundChallenge: 'Challenge {n}',
    playgroundGoal: 'Goal',
    playgroundHint: 'Hint',
    playgroundCheck: 'Check',
    playgroundSuccess: 'Great job! Challenge passed!',
    playgroundFail: 'Not quite right. Try again.',
    playgroundNext: 'Next Challenge',
    playgroundComplete: 'All challenges completed! Congratulations!',
    playgroundReset: 'Start Over',
    cmdPlayground: 'Playground (tutorial)',
    // Batch 40 — Error Explanation
    errorExplanation: 'Explanation',
    errorFixHint: 'Possible fix',
    showErrorHelp: 'Help',
    // Batch 42 — Weight/Cost Estimator
    material: 'Material',
    weight: 'Weight',
    cost: 'Cost',
    materialPLA: 'PLA',
    materialABS: 'ABS',
    materialPETG: 'PETG',
    materialNylon: 'Nylon',
    materialResin: 'Resin',
    costPerKg: 'Cost per kg',
    currency: 'Currency',
    // Batch 42 — About Dialog
    about: 'About',
    aboutTitle: 'About',
    aboutVersion: 'Version',
    aboutBuiltWith: 'Built with Vue 3, WebGPU, TypeScript',
    aboutLicense: 'License: MIT',
    // Batch 42 — Screenshot Comparison
    compareScreenshots: 'Compare Screenshots',
    saveReference: 'Save Reference',
    compareWithReference: 'Compare with Reference',
    referenceNotSaved: 'Save a reference screenshot first',
    overlay: 'Overlay',
    sideBySide: 'Side by Side',
    difference: 'Difference',
    cmdAbout: 'About',
  },
  zh: {
    title: 'OpenSCAD 3D 查看器',
    subtitle: '基于WebGPU的OpenSCAD编辑器',
    render: '渲染', auto: '自动', examples: '示例',
    basic: '基础', csg: 'CSG', house: '房屋', tower: '塔',
    meshes: '网格', triangles: '三角形',
    hint: '左键:旋转 · Ctrl+左键:15°吸附 · 右键/Shift:平移 · 滚轮:缩放 · Ctrl+Enter:渲染',
    noGpu: 'WebGPU不受支持。请使用Chrome 113+ / Edge 113+ / Firefox Nightly。',
    theme: '主题',
    wireframe: '线框', grid: '网格', fullscreen: '全屏',
    autoRotate: '自动旋转',
    shortcuts: '快捷键',
    shortcutsTitle: '快捷键',
    close: '关闭',
    preferences: '设置',
    exportStl: '导出STL',
    save: '保存',
    open: '打开',
    find: '查找',
    replace: '替换',
    replaceAll: '全部替换',
    next: '下一个',
    prev: '上一个',
    console: '控制台',
    consoleClear: '清空',
    parameters: '参数',
    lighting: '照明',
    screenshot: '截图',
    format: '格式化',
    themeSelector: '主题选择',
    fontSize: '字体大小',
    tabSize: 'Tab大小',
    newTab: '新建标签',
    closeTab: '关闭标签',
    minimap: '小地图',
    exportObj: '导出OBJ',
    export3mf: '导出3MF',
    exportPng: '导出PNG',
    share: '分享',
    copied: '已复制!',
    shareInvalid: '无效的分享链接',
    undo: '撤销',
    commandPalette: '命令面板',
    wordWrap: '自动换行',
    objectTree: '对象树',
    renderTime: '渲染',
    top: '俯', front: '前', right: '右', iso: '等轴', reset: '重置',
    persp: '透视', ortho: '正交',
    zoomIn: '放大', zoomOut: '缩小',
    gear: '齿轮', vase: '花瓶', chess: '棋子', mechanical: '机械',
    staircase: '楼梯', paramVase: '参数花瓶', paramGear: '参数齿轮',
    honeycomb: '蜂窝',
    honeycombTip: '六角网格',
    springCoil: '弹簧',
    springCoilTip: '螺旋弹簧',
    knurledCylinder: '滚花',
    knurledCylinderTip: '滚花圆柱',
    chamferedBox: '倒角',
    chamferedBoxTip: '倒角立方体',
    loftShape: '放样',
    loftShapeTip: '截面放样',
    statistics: '统计', history: '历史',
    historyTitle: '版本历史',
    historyEmpty: '没有保存的版本',
    historyRestore: '恢复',
    historyEntry: '{n}个字符',
    ghostCompare: '对比',
    ghostTab: '对比标签',
    ghostStats: '模型对比',
    ghostDeltaTriangles: 'Δ 三角形',
    ghostDeltaVolume: 'Δ 体积',
    ghostDeltaSize: 'Δ 尺寸',
    ghostOverlayMode: '对比模式',
    ghostTransparent: '透明',
    ghostSideBySide: '左右对比',
    ghostDifference: '差异',
    historyTimeline: '时间线',
    historyCurrent: '当前',
    justNow: '刚刚',
    minutesAgo: '{n}分钟前',
    hoursAgo: '{n}小时前',
    daysAgo: '{n}天前',
    menu: '菜单',
    simpleMode: '简单',
    advancedMode: '高级',
    clipPlane: '截面',
    fog: '雾',
    reflection: '反射',
    scadReference: 'OpenSCAD参考',
    whatsNew: '新功能',
    measureMode: '测量',
    buildPlate: '打印平台',
    exportDropdown: '导出',
    menuView: '视图', menuRender: '渲染', menuExport: '导出',
    // Batch 38
    duplicateTab: '复制标签',
    tabColor: '标签颜色',
    tabColorNone: '无',
    quickSwitcher: '快速切换',
    notificationCenter: '通知中心',
    clearAllNotifications: '清除所有',
    noNotifications: '没有通知',
    specSheet: '规格表',
    uiDensity: '界面密度',
    densityCompact: '紧凑',
    densityNormal: '正常',
    densityComfortable: '舒适',
    // Batch 40
    latticeEx: '晶格',
    latticeTip: '立方晶格结构',
    slotCrossEx: '槽和十字',
    slotCrossTip: '体育场槽和十字形',
    mazeEx: '迷宫',
    mazeTip: '随机迷宫生成器',
    fibSphereEx: '斐波那契球',
    fibSphereTip: '斐波那契螺旋点分布',
    customTheme: '自定义主题',
    customThemeName: '主题名称',
    themeBackground: '背景',
    themeSurface: '表面',
    themeBorder: '边框',
    themeText: '文字',
    themeAccent: '强调',
    themeSave: '保存主题',
    themeDelete: '删除',
    themeNamePlaceholder: '我的主题',
    themeSaved: '主题已保存',
    themeDeleted: '主题已删除',
    playground: '练习场',
    playgroundTitle: '交互式练习场',
    playgroundChallenge: '挑战 {n}',
    playgroundGoal: '目标',
    playgroundHint: '提示',
    playgroundCheck: '检查',
    playgroundSuccess: '太棒了！挑战通过！',
    playgroundFail: '还不太对，再试试。',
    playgroundNext: '下一个挑战',
    playgroundComplete: '所有挑战完成！恭喜！',
    playgroundReset: '重新开始',
    cmdPlayground: '练习场（教程）',
    errorExplanation: '说明',
    errorFixHint: '可能的修复',
    showErrorHelp: '帮助',
    // Batch 42
    material: '材料',
    weight: '重量',
    cost: '成本',
    materialPLA: 'PLA',
    materialABS: 'ABS',
    materialPETG: 'PETG',
    materialNylon: '尼龙',
    materialResin: '树脂',
    costPerKg: '每公斤价格',
    currency: '货币',
    about: '关于',
    aboutTitle: '关于',
    aboutVersion: '版本',
    aboutBuiltWith: '使用 Vue 3, WebGPU, TypeScript 构建',
    aboutLicense: '许可证: MIT',
    compareScreenshots: '截图对比',
    saveReference: '保存参考',
    compareWithReference: '与参考对比',
    referenceNotSaved: '请先保存参考截图',
    overlay: '叠加',
    sideBySide: '并排',
    difference: '差异',
    cmdAbout: '关于',
  },
}

const t = (k: string) => L[lang.value]?.[k] ?? L.en?.[k] ?? k
const toggleLang = () => { const cycle: Array<'ru'|'en'|'de'|'zh'> = ['ru', 'en', 'de', 'zh']; const idx = cycle.indexOf(lang.value); lang.value = cycle[(idx + 1) % cycle.length]; localStorage.setItem('scad-lang', lang.value) }

onMounted(() => {
  const savedThemeId = localStorage.getItem('scad-editor-theme')
  if (savedThemeId) {
    activeThemeId.value = savedThemeId
  } else {
    const saved = localStorage.getItem('scad-theme')
    activeThemeId.value = saved === 'light' ? 'default-light' : 'default-dark'
  }
  applyTheme()
  // Trigger the first-load fade-in on the next frame.
  requestAnimationFrame(() => { appLoaded.value = true })
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
  pinned?: boolean
  savedCode?: string
  colorTag?: string  // 'red' | 'green' | 'blue' | 'yellow' | 'purple' | ''
}

function generateTabId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
}

function loadTabsFromStorage(): EditorTab[] {
  const parsed = safeParse<EditorTab[]>(localStorage.getItem('scad-tabs'), [])
  if (Array.isArray(parsed) && parsed.length > 0) return parsed
  // Migrate from old single-code storage
  const oldCode = localStorage.getItem('scad-code')
  const initialCode = oldCode || EXAMPLES.basic
  return [{ id: generateTabId(), name: t('untitled') + ' 1', code: initialCode, savedCode: initialCode }]
}

const tabs = ref<EditorTab[]>(loadTabsFromStorage())
// Ensure all tabs have savedCode initialized (handles old localStorage format)
for (const tb of tabs.value) {
  if (tb.savedCode === undefined) tb.savedCode = tb.code
}
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
  const tab: EditorTab = { id: generateTabId(), name: `${t('untitled')} ${idx}`, code: '', savedCode: '' }
  tabs.value.push(tab)
  activeTabId.value = tab.id
  saveTabs()
}

function closeTab(id: string) {
  if (tabs.value.length <= 1) return
  const target = tabs.value.find(tb => tb.id === id)
  if (target?.pinned) return // pinned tabs cannot be closed
  const idx = tabs.value.findIndex(tb => tb.id === id)
  if (idx === -1) return
  // Snapshot the closed tab (id, name, code, position) so it can be restored.
  const closedTab: EditorTab = { ...tabs.value[idx] }
  const closedIndex = idx
  tabs.value.splice(idx, 1)
  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id
  }
  saveTabs()
  // Offer an Undo action via the toast/snackbar (auto-dismiss ~5s).
  addToast(t('tabClosed') + ': ' + closedTab.name, 'info', {
    actionLabel: t('undo'),
    duration: 5000,
    action: () => restoreClosedTab(closedTab, closedIndex),
  })
}

function restoreClosedTab(tab: EditorTab, index: number) {
  // Avoid id collision if a new tab happened to reuse the slot.
  if (tabs.value.some(tb => tb.id === tab.id)) {
    tab = { ...tab, id: generateTabId() }
  }
  const insertAt = Math.max(0, Math.min(index, tabs.value.length))
  tabs.value.splice(insertAt, 0, tab)
  activeTabId.value = tab.id
  saveTabs()
  addToast(t('tabRestored') + ': ' + tab.name, 'success')
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

/* ── Sorted tabs (pinned first) ── */
const sortedTabs = computed(() => {
  const pinned = tabs.value.filter(tb => tb.pinned)
  const unpinned = tabs.value.filter(tb => !tb.pinned)
  return [...pinned, ...unpinned]
})

/* ── Tab context menu ── */
const showTabContextMenu = ref(false)
const tabContextMenuX = ref(0)
const tabContextMenuY = ref(0)
const tabContextMenuId = ref('')

function onTabContextMenu(e: MouseEvent, tabId: string) {
  e.preventDefault()
  tabContextMenuId.value = tabId
  tabContextMenuX.value = e.clientX
  tabContextMenuY.value = e.clientY
  showTabContextMenu.value = true
}

function closeTabContextMenu() {
  showTabContextMenu.value = false
}

function togglePinTab(id: string) {
  const tab = tabs.value.find(tb => tb.id === id)
  if (tab) {
    tab.pinned = !tab.pinned
    saveTabs()
  }
  closeTabContextMenu()
}

function closeOtherTabs(id: string) {
  const keep = tabs.value.filter(tb => tb.id === id || tb.pinned)
  if (keep.length === 0) return
  tabs.value = keep
  if (!tabs.value.find(tb => tb.id === activeTabId.value)) {
    activeTabId.value = tabs.value[0].id
  }
  saveTabs()
  closeTabContextMenu()
}

/* ── Keyboard tab navigation ── */
function switchToNextTab() {
  const sorted = sortedTabs.value
  const idx = sorted.findIndex(tb => tb.id === activeTabId.value)
  const next = sorted[(idx + 1) % sorted.length]
  if (next) switchTab(next.id)
}

function switchToPrevTab() {
  const sorted = sortedTabs.value
  const idx = sorted.findIndex(tb => tb.id === activeTabId.value)
  const prev = sorted[(idx - 1 + sorted.length) % sorted.length]
  if (prev) switchTab(prev.id)
}

function closeCurrentTab() {
  const tab = tabs.value.find(tb => tb.id === activeTabId.value)
  if (tab && tab.pinned) return
  closeTab(activeTabId.value)
}

/* ── Editor + Renderer ── */

const canvasRef = ref<HTMLCanvasElement | null>(null)
const error = ref('')
const errorLine = ref(-1)
const errorCharPos = ref(-1)
const meshCount = ref(0)
const triCount = ref(0)
// Guard so the heavy-model warning toast fires once per crossing into the
// heavy range, instead of on every render. Reset when back under threshold.
const heavyWarned = ref(false)
const gpuOk = ref(true)
const autoRender = ref(true)
const renderTime = ref(0)
const isFullscreen = ref(false)
const showWireframe = ref(false)
const showGrid = ref(true)
const isAutoRotate = ref(false)
const isOrthographic = ref(false)
const activeRenderMode = ref<string>('solid')
const flatShadingEnabled = ref(false)
const ssaoEnabled = ref(false)
const outlineEnabled = ref(false)
const smoothNormalsEnabled = ref(false)
const colorGrading = ref('none')
const skyPreset = ref('none')
const toonShadingEnabled = ref(false)
const explodeFactorVal = ref(0)
const vignetteEnabled = ref(false)
const fovDeg = ref(45)
const inertiaEnabled = ref(localStorage.getItem('scad-orbit-inertia') === 'true')

const COLOR_GRADING_FILTERS: Record<string, string> = {
  none: '',
  warm: 'sepia(0.15) saturate(1.2)',
  cool: 'hue-rotate(10deg) saturate(0.9) brightness(1.05)',
  vintage: 'sepia(0.3) contrast(1.1) brightness(0.95)',
  noir: 'grayscale(1) contrast(1.3)',
  vivid: 'saturate(1.6) contrast(1.1)',
}

const canvasFilter = computed(() => {
  let f = COLOR_GRADING_FILTERS[colorGrading.value] || ''
  if (bloomEnabled.value) {
    f = (f ? f + ' ' : '') + 'contrast(1.1) brightness(1.05)'
  }
  return f
})

/* ── Code Folding ── */
const foldedLines = ref<Set<number>>(new Set())
const foldRanges = ref<Map<number, number>>(new Map()) // startLine -> endLine

/* ── Color Picker ── */
const colorPickerVisible = ref(false)
const colorPickerX = ref(0)
const colorPickerY = ref(0)
const colorPickerValue = ref('#ffffff')
let colorPickerMatch: { start: number; end: number } | null = null

const fastPreviewMode = ref(false)

/* ── Batch 35 features ── */
const goochShadingEnabled = ref(false)
const groundShadowEnabled = ref(false)
const zenModeActive = ref(false)
const showTipOfDay = ref(false)
const tipOfDayDismissed = ref(localStorage.getItem('scad-no-tips') === 'true')
const turntableExporting = ref(false)

let zenChordPending = false
let zenChordTimer: ReturnType<typeof setTimeout> | null = null

/* ── Batch 36 features ── */
const flyCameraMode = ref(false)
const bloomEnabled = ref(false)
const sectionBoxEnabled = ref(false)
const sectionBoxX = ref(0)
const sectionBoxY = ref(0)
const sectionBoxZ = ref(0)
const meshColorOverrides = ref<Map<number, string>>(new Map())
const meshVisibilityMap = ref<Map<number, boolean>>(new Map())

let renderer: WebGPURenderer | null = null
let debounce: ReturnType<typeof setTimeout> | null = null
let lastParsedMeshes: MeshData[] = []

/* ── WebGPU initialization / loading state ── */
const rendererReady = ref(false)
const initFailed = ref(false)
const deviceLost = ref(false)

/* ── Custom Undo/Redo Stack (Feature 4) ── */
const UNDO_MAX = 100
const undoStack = ref<string[]>([])
const redoStack = ref<string[]>([])
let undoDebounceTimer: ReturnType<typeof setTimeout> | null = null

function pushUndoSnapshot(snapshot?: string) {
  const snap = snapshot ?? code.value
  const top = undoStack.value.length > 0 ? undoStack.value[undoStack.value.length - 1] : null
  if (snap === top) return // no duplicate
  undoStack.value.push(snap)
  if (undoStack.value.length > UNDO_MAX) undoStack.value.shift()
  redoStack.value = [] // clear redo on new edit
}

function pushUndoDebounced() {
  if (undoDebounceTimer) clearTimeout(undoDebounceTimer)
  undoDebounceTimer = setTimeout(() => {
    pushUndoSnapshot()
  }, 500)
}

function customUndo() {
  if (!undoStack.value.length) return
  // Push current state onto redo before reverting
  redoStack.value.push(code.value)
  const prev = undoStack.value.pop()!
  // Temporarily disable the watcher-driven undo push
  suppressUndoPush = true
  code.value = prev
  nextTick(() => { suppressUndoPush = false })
}

function customRedo() {
  if (!redoStack.value.length) return
  // Push current state onto undo before redoing
  undoStack.value.push(code.value)
  const next = redoStack.value.pop()!
  suppressUndoPush = true
  code.value = next
  nextTick(() => { suppressUndoPush = false })
}

let suppressUndoPush = false

/* ── Performance Panel (Feature 5) ── */
const showPerfPanel = ref(false)
const perfParseTime = ref(0)
const perfMeshGenTime = ref(0)
const perfGpuUploadTime = ref(0)

const perfDeviceInfo = computed(() => {
  if (!renderer) return { adapter: '', description: '', vendor: '', architecture: '', maxBufferSize: 0, maxTextureSize: 0 }
  return renderer.getDeviceInfo()
})

const perfBufferStats = computed(() => {
  // Touch reactive refs to ensure recomputation when meshes change
  void triCount.value
  void meshCount.value
  if (!renderer) return { totalVertexBytes: 0, totalIndexBytes: 0, meshCount: 0, triangleCount: 0 }
  return renderer.getBufferStats()
})

/* ── WebGPU Reinitialize (Device Lost Recovery) ── */
async function reinitializeWebGPU() {
  if (!canvasRef.value) return
  // Destroy old renderer
  try { renderer?.destroy() } catch { /* ignore cleanup errors */ }
  renderer = null

  renderer = new WebGPURenderer()
  let ok = false
  try {
    ok = await renderer.init(canvasRef.value)
  } catch {
    ok = false
  }
  if (!ok) {
    addToast(t('reinitFailed'), 'error')
    addConsoleEntry('error', t('reinitFailed'))
    return
  }
  deviceLost.value = false

  // Re-attach device lost handler
  renderer.onDeviceLost = (reason: string) => {
    deviceLost.value = true
    addConsoleEntry('error', t('deviceLost') + ': ' + reason)
    addToast(t('deviceLostMsg'), 'error', {
      actionLabel: t('reinitialize'),
      duration: 10000,
      action: () => reinitializeWebGPU(),
    })
  }

  applyBuildPlate()
  doRender()
  addToast(t('reinitSuccess'), 'success')
  addConsoleEntry('info', t('reinitSuccess'))
}

/* ── App first-load fade-in ── */
const appLoaded = ref(false)

/* ── Feature: First-run Welcome / Onboarding ── */
const showWelcome = ref(!localStorage.getItem('scad-onboarded'))

function dismissWelcome() {
  showWelcome.value = false
  localStorage.setItem('scad-onboarded', '1')
}

function openWelcome() {
  showWelcome.value = true
}

/* ── Welcome Tour (Interactive) ── */
const tourActive = ref(false)
const tourStep = ref(0)
const TOUR_STEPS = [
  { target: '.editor-panel', key: 'tourStep1' },
  { target: '.btn-primary', key: 'tourStep2' },
  { target: '.btn-sm', key: 'tourStep3' },
  { target: '.canvas-panel', key: 'tourStep4' },
  { target: '.vp-toolbar', key: 'tourStep5' },
  { target: '.export-dropdown-wrapper', key: 'tourStep6' },
]

const tourTooltipStyle = ref<Record<string, string>>({})

function startTour() {
  showWelcome.value = false
  localStorage.setItem('scad-onboarded', '1')
  tourActive.value = true
  tourStep.value = 0
  nextTick(positionTourTooltip)
}

function nextTourStep() {
  if (tourStep.value < TOUR_STEPS.length - 1) {
    tourStep.value++
    nextTick(positionTourTooltip)
  } else {
    endTour()
  }
}

function prevTourStep() {
  if (tourStep.value > 0) {
    tourStep.value--
    nextTick(positionTourTooltip)
  }
}

function endTour() {
  tourActive.value = false
  tourStep.value = 0
}

function positionTourTooltip() {
  const step = TOUR_STEPS[tourStep.value]
  if (!step) return
  const el = document.querySelector(step.target)
  if (!el) return
  const rect = el.getBoundingClientRect()
  const top = rect.bottom + 12
  const left = Math.max(10, rect.left + rect.width / 2 - 150)
  tourTooltipStyle.value = {
    position: 'fixed',
    top: Math.min(top, window.innerHeight - 140) + 'px',
    left: Math.min(left, window.innerWidth - 320) + 'px',
    zIndex: '10001',
  }
}

const tourSpotlightStyle = computed(() => {
  if (!tourActive.value) return {} as Record<string, string>
  const step = TOUR_STEPS[tourStep.value]
  if (!step) return {} as Record<string, string>
  const el = document.querySelector(step.target)
  if (!el) return {} as Record<string, string>
  const rect = el.getBoundingClientRect()
  return {
    position: 'fixed',
    top: (rect.top - 6) + 'px',
    left: (rect.left - 6) + 'px',
    width: (rect.width + 12) + 'px',
    height: (rect.height + 12) + 'px',
    borderRadius: '8px',
  }
})

/* ── Drag & drop state ── */
const isDragOver = ref(false)
let dragCounter = 0

/* ── Stats polling ── */
const fpsVal = ref(0)
const vertexCount = ref(0)
const boundsSize = ref<[number, number, number]>([0, 0, 0])
let statsInterval: ReturnType<typeof setInterval> | null = null

/* ── Preferences ── */
const PREF_DEFAULTS = {
  fontSize: 13,
  tabSize: 4,
  autoRenderDelay: 400,
  showMinimap: true,
  showLineNumbers: true,
}
const showPreferences = ref(false)
const prefFontSize = ref(parseInt(localStorage.getItem('scad-pref-fontSize') || String(PREF_DEFAULTS.fontSize)))
const prefTabSize = ref(parseInt(localStorage.getItem('scad-pref-tabSize') || String(PREF_DEFAULTS.tabSize)))
const prefAutoRenderDelay = ref(parseInt(localStorage.getItem('scad-pref-autoRenderDelay') || String(PREF_DEFAULTS.autoRenderDelay)))
const prefShowMinimap = ref(localStorage.getItem('scad-pref-showMinimap') !== 'false')
const prefShowLineNumbers = ref(localStorage.getItem('scad-pref-showLineNumbers') !== 'false')

const FONT_OPTIONS = [
  'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'SF Mono', 'Consolas', 'Monaco', 'monospace',
]
const prefFontFamily = ref(localStorage.getItem('scad-pref-fontFamily') || 'JetBrains Mono')

function savePref(key: string, val: string) {
  localStorage.setItem(`scad-pref-${key}`, val)
}

function resetPreferences() {
  prefFontSize.value = PREF_DEFAULTS.fontSize
  prefTabSize.value = PREF_DEFAULTS.tabSize
  prefAutoRenderDelay.value = PREF_DEFAULTS.autoRenderDelay
  prefShowMinimap.value = PREF_DEFAULTS.showMinimap
  prefShowLineNumbers.value = PREF_DEFAULTS.showLineNumbers
  prefFontFamily.value = 'JetBrains Mono'
  shortcutPreset.value = 'default'
  simpleMode.value = false
  // Clear the related localStorage keys (the watchers above will re-persist the defaults)
  for (const key of ['fontSize', 'tabSize', 'autoRenderDelay', 'showMinimap', 'showLineNumbers', 'fontFamily']) {
    localStorage.removeItem(`scad-pref-${key}`)
  }
  localStorage.removeItem('scad-shortcut-preset')
  localStorage.removeItem('scad-simple-mode')
  // Reset watermark
  wmEnabled.value = false
  wmText.value = ''
  wmPosition.value = 'bottom-right'
  wmOpacity.value = 0.15
  localStorage.removeItem('scad-wm-enabled')
  localStorage.removeItem('scad-wm-text')
  localStorage.removeItem('scad-wm-position')
  localStorage.removeItem('scad-wm-opacity')
  addToast(t('prefsReset'), 'success')
}

/* ── Export Preset Configurations ── */
interface ViewerPreset {
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

const BUILT_IN_PRESETS: Record<string, ViewerPreset> = {
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

function loadUserPresets(): Record<string, ViewerPreset> {
  const parsed = safeParse<Record<string, ViewerPreset>>(localStorage.getItem('scad-user-presets'), {})
  return (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) ? parsed : {}
}

const userPresets = ref<Record<string, ViewerPreset>>(loadUserPresets())
const presetNameInput = ref('')

function saveUserPresetsToStorage() {
  localStorage.setItem('scad-user-presets', JSON.stringify(userPresets.value))
}

function captureCurrentPreset(): ViewerPreset {
  return {
    name: presetNameInput.value || 'Untitled',
    themeId: activeThemeId.value,
    lighting: activeLighting.value,
    renderMode: activeRenderMode.value,
    skyPreset: skyPreset.value,
    colorGrading: colorGrading.value,
    showGrid: showGrid.value,
    flatShading: flatShadingEnabled.value,
    ssao: ssaoEnabled.value,
    outline: outlineEnabled.value,
    smoothNormals: smoothNormalsEnabled.value,
    fontSize: prefFontSize.value,
    tabSize: prefTabSize.value,
  }
}

function savePreset() {
  const name = presetNameInput.value.trim()
  if (!name) return
  userPresets.value[name] = captureCurrentPreset()
  saveUserPresetsToStorage()
  presetNameInput.value = ''
  addToast(t('presetSaved'), 'success')
}

function applyPreset(preset: ViewerPreset) {
  selectTheme(preset.themeId)
  activeLighting.value = preset.lighting
  renderer?.setLighting(preset.lighting)
  setRenderMode(preset.renderMode)
  skyPreset.value = preset.skyPreset
  colorGrading.value = preset.colorGrading
  showGrid.value = preset.showGrid
  flatShadingEnabled.value = preset.flatShading
  ssaoEnabled.value = preset.ssao
  outlineEnabled.value = preset.outline
  smoothNormalsEnabled.value = preset.smoothNormals
  prefFontSize.value = preset.fontSize
  prefTabSize.value = preset.tabSize
  if (renderer) {
    renderer.showGrid = preset.showGrid
    renderer.setRenderMode(preset.renderMode)
    renderer.requestRender()
  }
  addToast(t('presetLoaded'), 'success')
}

function deleteUserPreset(name: string) {
  delete userPresets.value[name]
  saveUserPresetsToStorage()
  addToast(t('presetDeleted'), 'info')
}

watch(prefFontSize, v => { savePref('fontSize', String(v)) })
watch(prefTabSize, v => { savePref('tabSize', String(v)) })
watch(prefAutoRenderDelay, v => { savePref('autoRenderDelay', String(v)) })
watch(prefShowMinimap, v => {
  savePref('showMinimap', String(v))
  showMinimap.value = v
  if (v) nextTick(renderMinimap)
})
watch(prefShowLineNumbers, v => { savePref('showLineNumbers', String(v)) })
watch(prefFontFamily, v => { savePref('fontFamily', v) })

/* ── Progressive Disclosure (Simple/Advanced Mode) ── */
const simpleMode = ref(localStorage.getItem('scad-simple-mode') === 'true')
watch(simpleMode, v => { localStorage.setItem('scad-simple-mode', String(v)) })

/* ── Split Editor ── */
const splitMode = ref(false)
const rightTabId = ref('')
const rightScrollTop = ref(0)

function toggleSplitMode() {
  splitMode.value = !splitMode.value
  if (splitMode.value && !rightTabId.value) {
    // Default right pane to a different tab if available, else same
    const otherTab = tabs.value.find(tb => tb.id !== activeTabId.value)
    rightTabId.value = otherTab ? otherTab.id : activeTabId.value
  }
}

const rightTab = computed(() => {
  if (!rightTabId.value) return null
  return tabs.value.find(tb => tb.id === rightTabId.value) || null
})

const rightCode = computed(() => rightTab.value?.code ?? '')

function onRightPaneInput(e: Event) {
  const target = e.target as HTMLTextAreaElement
  if (rightTab.value) {
    rightTab.value.code = target.value
    saveTabs()
  }
}

function onRightPaneScroll(e: Event) {
  const target = e.target as HTMLTextAreaElement
  rightScrollTop.value = target.scrollTop
}

/* ── Version History ── */
interface TabHistoryEntry {
  timestamp: number
  code: string
}

const showHistory = ref(false)
const MAX_HISTORY_ENTRIES = 20
const HISTORY_STORAGE_KEY = 'scad-tab-history'

function loadHistories(): Record<string, TabHistoryEntry[]> {
  const parsed = safeParse<Record<string, TabHistoryEntry[]>>(localStorage.getItem(HISTORY_STORAGE_KEY), {})
  return (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) ? parsed : {}
}

function saveHistories(h: Record<string, TabHistoryEntry[]>) {
  try {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(h))
  } catch {
    // Quota exceeded: trim the oldest entries (halve the total across all tabs,
    // dropping oldest first) and retry the save once.
    try {
      const total = Object.values(h).reduce((s, e) => s + e.length, 0)
      let toDrop = Math.ceil(total / 2)
      while (toDrop > 0) {
        // Find the tab whose oldest entry has the smallest timestamp.
        let oldestTab: string | null = null
        let oldestTs = Infinity
        for (const [tabId, entries] of Object.entries(h)) {
          if (entries.length > 0 && entries[0].timestamp < oldestTs) {
            oldestTs = entries[0].timestamp
            oldestTab = tabId
          }
        }
        if (!oldestTab) break
        h[oldestTab].shift()
        if (h[oldestTab].length === 0) delete h[oldestTab]
        toDrop--
      }
      localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(h))
    } catch (e) {
      console.warn('saveHistories: storage quota exceeded, history not saved', e)
    }
  }
}

function addHistorySnapshot(tabId: string, codeVal: string) {
  const all = loadHistories()
  const entries = all[tabId] || []
  // Don't save if same as last entry
  if (entries.length > 0 && entries[entries.length - 1].code === codeVal) return
  entries.push({ timestamp: Date.now(), code: codeVal })
  // Keep max entries
  while (entries.length > MAX_HISTORY_ENTRIES) entries.shift()
  all[tabId] = entries
  saveHistories(all)
}

const currentTabHistory = computed(() => {
  const all = loadHistories()
  return (all[activeTabId.value] || []).slice().reverse()
})

function restoreHistoryEntry(entry: TabHistoryEntry) {
  code.value = entry.code
  showHistory.value = false
}

function formatHistoryTime(ts: number): string {
  const diff = Date.now() - ts
  if (diff < 60000) return t('justNow')
  if (diff < 3600000) return t('minutesAgo').replace('{n}', String(Math.floor(diff / 60000)))
  if (diff < 86400000) return t('hoursAgo').replace('{n}', String(Math.floor(diff / 3600000)))
  return t('daysAgo').replace('{n}', String(Math.floor(diff / 86400000)))
}

// Debounced history save
let historyDebounce: ReturnType<typeof setTimeout> | null = null

/* ── Shortcut Presets ── */
type ShortcutPreset = 'default' | 'vscode' | 'sublime' | 'emacs'
const shortcutPreset = ref<ShortcutPreset>((localStorage.getItem('scad-shortcut-preset') as ShortcutPreset) || 'default')
watch(shortcutPreset, v => { localStorage.setItem('scad-shortcut-preset', v) })

interface KeyBinding {
  render: string
  format: string
  fullscreen: string
  commandPalette: string
}

const SHORTCUT_PRESETS: Record<ShortcutPreset, KeyBinding> = {
  default: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  vscode: { render: 'Ctrl+Enter', format: 'Shift+Alt+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  sublime: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Ctrl+Shift+P' },
  emacs: { render: 'Ctrl+Enter', format: 'Ctrl+Shift+F', fullscreen: 'F11', commandPalette: 'Alt+X' },
}

function matchesBinding(e: KeyboardEvent, binding: string): boolean {
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

/* ── Console / Log Panel ── */
interface ConsoleEntry {
  id: number
  type: 'info' | 'warn' | 'error'
  message: string
  timestamp: Date
}
let consoleIdCounter = 0
const showConsole = ref(false)
const consoleEntries = ref<ConsoleEntry[]>([])
const consoleRef = ref<HTMLElement | null>(null)
const consolePanelHeight = ref(parseInt(localStorage.getItem('scad-console-height') || '150'))
const consoleDragging = ref(false)

function addConsoleEntry(type: 'info' | 'warn' | 'error', message: string) {
  consoleEntries.value.push({ id: consoleIdCounter++, type, message, timestamp: new Date() })
  if (consoleEntries.value.length > 50) {
    consoleEntries.value = consoleEntries.value.slice(-50)
  }
  nextTick(() => {
    if (consoleRef.value) {
      consoleRef.value.scrollTop = consoleRef.value.scrollHeight
    }
  })
}

function clearConsole() {
  consoleEntries.value = []
}

function onConsoleDragStart(e: MouseEvent) {
  e.preventDefault()
  consoleDragging.value = true
  const startY = e.clientY
  const startH = consolePanelHeight.value
  function onMove(ev: MouseEvent) {
    consolePanelHeight.value = Math.max(80, Math.min(400, startH - (ev.clientY - startY)))
  }
  function onUp() {
    consoleDragging.value = false
    localStorage.setItem('scad-console-height', String(consolePanelHeight.value))
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

function formatConsoleTime(d: Date): string {
  return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

/* ── Copy as Image ── */
const showCopiedImage = ref(false)

async function copyCanvasToClipboard() {
  if (!renderer) return
  const ok = await renderer.copyToClipboard()
  if (ok) {
    showCopiedImage.value = true
    setTimeout(() => { showCopiedImage.value = false }, 1500)
    addToast(t('copiedImage'), 'success')
  } else {
    // Fallback to download
    takeScreenshot()
  }
}

/* ── Object Tree Panel ── */
const showObjectTree = ref(false)
const astNodes = ref<ASTNode[]>([])
const expandedNodes = ref<Set<number>>(new Set())

function toggleTreeNode(pos: number) {
  if (expandedNodes.value.has(pos)) {
    expandedNodes.value.delete(pos)
  } else {
    expandedNodes.value.add(pos)
  }
  // Force reactivity
  expandedNodes.value = new Set(expandedNodes.value)
}

function scrollEditorToLine(pos: number) {
  const el = textareaRef.value
  if (!el) return
  const textBefore = code.value.substring(0, pos)
  const lineNum = textBefore.split('\n').length
  const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20
  el.scrollTo({ top: Math.max(0, (lineNum - 3) * lineHeight), behavior: 'smooth' })
  // Also place cursor there
  const lineStart = code.value.lastIndexOf('\n', pos - 1) + 1
  const lineEnd = code.value.indexOf('\n', pos)
  el.focus()
  el.setSelectionRange(lineStart, lineEnd === -1 ? code.value.length : lineEnd)
  syncScroll()
}

function getNodeSummary(node: ASTNode): string {
  const a = node.args
  switch (node.name) {
    case 'cube': {
      const size = a.size ?? a._0 ?? 1
      if (Array.isArray(size)) return `[${size.join(',')}]`
      return `${size}`
    }
    case 'sphere': {
      const r = a.r ?? a._0 ?? a.d ? `d=${a.d}` : '1'
      return `r=${r}`
    }
    case 'cylinder': {
      const h = a.h ?? a._0 ?? 1
      return `h=${h}`
    }
    case 'translate': {
      const v = a.v ?? a._0 ?? [0,0,0]
      if (Array.isArray(v)) return `[${v.join(',')}]`
      return ''
    }
    case 'rotate': {
      const v = a.a ?? a._0 ?? 0
      if (Array.isArray(v)) return `[${v.join(',')}]`
      return `${v}`
    }
    case 'scale': {
      const v = a.v ?? a._0 ?? [1,1,1]
      if (Array.isArray(v)) return `[${v.join(',')}]`
      return `${v}`
    }
    case 'color': {
      const c = a.c ?? a._0 ?? ''
      if (Array.isArray(c)) return `[${c.map((x: number) => typeof x === 'number' ? x.toFixed(2) : x).join(',')}]`
      return String(c)
    }
    default:
      return ''
  }
}

function getNodeIcon(name: string): string {
  const icons: Record<string, string> = {
    cube: '□', sphere: '○', cylinder: '▭', polygon: '△',
    translate: '→', rotate: '↻', scale: '⤢',
    color: '●', mirror: '↔', difference: '−',
    union: '∪', intersection: '∩',
  }
  return icons[name] || '▸'
}

interface FlatTreeNode {
  node: ASTNode
  depth: number
  hasChildren: boolean
  meshIndex: number // -1 if not a geometry leaf
}

const GEOMETRY_NODES = new Set(['cube', 'sphere', 'cylinder', 'polyhedron', 'circle', 'square', 'polygon', 'text', 'import', 'surface'])

const flatTree = computed(() => {
  const result: FlatTreeNode[] = []
  let meshIdx = 0
  function walk(nodes: ASTNode[], depth: number) {
    for (const n of nodes) {
      if (n.name === '__assign') continue
      const hasChildren = n.children.length > 0
      const isGeom = GEOMETRY_NODES.has(n.name) && !hasChildren
      result.push({ node: n, depth, hasChildren, meshIndex: isGeom ? meshIdx : -1 })
      if (isGeom) meshIdx++
      if (hasChildren && expandedNodes.value.has(n.pos)) {
        walk(n.children, depth + 1)
      }
    }
  }
  walk(astNodes.value, 0)
  return result
})

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
interface BgPreset {
  name: string
  nameKey?: string
  hex: string
  r: number; g: number; b: number
  gradient?: string
}
const bgColors: BgPreset[] = [
  { name: 'Dark',  hex: '#18181c', r: 0.09, g: 0.09, b: 0.11 },
  { name: 'Light', hex: '#e8e8ec', r: 0.91, g: 0.91, b: 0.93 },
  { name: 'Blue',  hex: '#1a2332', r: 0.10, g: 0.14, b: 0.20 },
  { name: 'Green', hex: '#1a2a1e', r: 0.10, g: 0.16, b: 0.12 },
  { name: 'Dark Grad', nameKey: 'bgGradDark', hex: 'linear-gradient(to top, #0a0a0e, #1e1e28)', r: 0.06, g: 0.06, b: 0.07, gradient: 'linear-gradient(to top, #0a0a0e, #1e1e28)' },
  { name: 'Blue Grad', nameKey: 'bgGradBlue', hex: 'linear-gradient(to top, #0a1628, #2a4a6e)', r: 0.04, g: 0.09, b: 0.16, gradient: 'linear-gradient(to top, #0a1628, #2a4a6e)' },
  { name: 'Sunset', nameKey: 'bgGradSunset', hex: 'linear-gradient(to top, #1a1040, #d46830)', r: 0.10, g: 0.06, b: 0.25, gradient: 'linear-gradient(to top, #1a1040, #d46830)' },
]
const activeBg = ref(0)
const canvasGradient = ref('')

function setBgColor(index: number) {
  activeBg.value = index
  const c = bgColors[index]
  if (c.gradient) {
    renderer?.setClearColor(0, 0, 0, 0)
    canvasGradient.value = c.gradient
  } else {
    renderer?.setClearColor(c.r, c.g, c.b, 1)
    canvasGradient.value = ''
  }
}

/* ── Toast Notifications ── */
interface Toast {
  id: number
  message: string
  type: 'success' | 'info' | 'error'
  actionLabel?: string
  action?: () => void
}
let toastIdCounter = 0
const toasts = ref<Toast[]>([])

function dismissToast(id: number) {
  toasts.value = toasts.value.filter(tst => tst.id !== id)
}

function addToast(
  message: string,
  type: 'success' | 'info' | 'error' = 'info',
  options?: { actionLabel?: string; action?: () => void; duration?: number },
) {
  const id = toastIdCounter++
  toasts.value.push({ id, message, type, actionLabel: options?.actionLabel, action: options?.action })
  const duration = options?.duration ?? 3000
  setTimeout(() => {
    dismissToast(id)
  }, duration)
}

function runToastAction(toast: Toast) {
  toast.action?.()
  dismissToast(toast.id)
}

/* ── STL Export ── */
function doExportSTL() {
  if (!lastParsedMeshes.length) return
  const tabName = activeTab.value.name.replace(/[^a-zA-Z0-9_-]/g, '_') || 'model'
  exportSTL(lastParsedMeshes, `${tabName}.stl`)
  addToast(t('exportStl') + ': ' + tabName + '.stl', 'success')
}

/* ── OBJ Export ── */
function doExportOBJ() {
  if (!lastParsedMeshes.length) return
  const tabName = activeTab.value.name.replace(/[^a-zA-Z0-9_-]/g, '_') || 'model'
  exportOBJ(lastParsedMeshes, `${tabName}.obj`)
  addToast(t('exportObj') + ': ' + tabName + '.obj', 'success')
}

/* ── 3MF Export ── */
function doExport3MF() {
  if (!lastParsedMeshes.length) return
  const tabName = activeTab.value.name.replace(/[^a-zA-Z0-9_-]/g, '_') || 'model'
  export3MF(lastParsedMeshes, `${tabName}.3mf`)
  addToast(t('export3mf') + ': ' + tabName + '.3mf', 'success')
}

/* ── Feature: Measurement Tool ── */
const measureMode = ref(false)
const measurePoint1 = ref<{x:number;y:number;z:number}|null>(null)
const measurePoint2 = ref<{x:number;y:number;z:number}|null>(null)

const measureDistance = computed(() => {
  if (!measurePoint1.value || !measurePoint2.value) return null
  const dx = measurePoint2.value.x - measurePoint1.value.x
  const dy = measurePoint2.value.y - measurePoint1.value.y
  const dz = measurePoint2.value.z - measurePoint1.value.z
  return Math.sqrt(dx*dx + dy*dy + dz*dz)
})

const measureLabelPos = computed(() => {
  if (!measurePoint1.value || !measurePoint2.value || !renderer) return null
  // Midpoint in 3D - we just center the label on canvas
  return { x: '50%', y: '50%' }
})

function toggleMeasureMode() {
  measureMode.value = !measureMode.value
  if (!measureMode.value) {
    measurePoint1.value = null
    measurePoint2.value = null
  }
}

function clearMeasurement() {
  measurePoint1.value = null
  measurePoint2.value = null
}

function onMeasureClick(e: PointerEvent) {
  if (!measureMode.value || !renderer || !canvasRef.value) return
  // Only handle left-click that wasn't a drag
  if (canvasDidDrag) return

  const rect = canvasRef.value.getBoundingClientRect()
  const sx = (e.clientX - rect.left) * window.devicePixelRatio
  const sy = (e.clientY - rect.top) * window.devicePixelRatio

  const ray = renderer.unproject(sx, sy)
  // Intersect with Y=0 plane (ground plane)
  // ray.origin + t * ray.direction, solve for y=0: t = -origin.y / direction.y
  if (Math.abs(ray.direction[1]) < 1e-8) return // parallel to ground
  const tHit = -ray.origin[1] / ray.direction[1]
  if (tHit < 0) return // behind camera
  const hitX = ray.origin[0] + tHit * ray.direction[0]
  const hitY = 0
  const hitZ = ray.origin[2] + tHit * ray.direction[2]

  if (!measurePoint1.value) {
    measurePoint1.value = { x: hitX, y: hitY, z: hitZ }
  } else if (!measurePoint2.value) {
    measurePoint2.value = { x: hitX, y: hitY, z: hitZ }
  } else {
    // Reset: start new measurement
    measurePoint1.value = { x: hitX, y: hitY, z: hitZ }
    measurePoint2.value = null
  }
}

/* ── Feature: Ghost Comparison ── */
const ghostMode = ref(false)
const ghostTabId = ref('')
const ghostOverlay = ref<'transparent'|'side-by-side'|'difference'>('transparent')
const ghostTriCount = ref(0)
const ghostVolume = ref(0)
const ghostBoundsSize = ref<[number, number, number]>([0, 0, 0])

function toggleGhostMode() {
  ghostMode.value = !ghostMode.value
  if (ghostMode.value) {
    // Select first other tab as default ghost
    const otherTab = tabs.value.find(tb => tb.id !== activeTabId.value)
    if (otherTab) {
      ghostTabId.value = otherTab.id
      updateGhostMeshes()
    } else {
      ghostMode.value = false
      addToast(t('noOtherTabs'), 'info')
    }
  } else {
    // Remove ghost meshes
    if (renderer) {
      doRender() // re-render without ghost
    }
  }
}

function updateGhostMeshes() {
  if (!ghostMode.value || !renderer || !ghostTabId.value) return
  const ghostTab = tabs.value.find(tb => tb.id === ghostTabId.value)
  if (!ghostTab) return

  // Parse ghost tab
  const ghostResult = parseOpenSCADWithAST(ghostTab.code, (name: string) => {
    const baseName = name.replace(/\.scad$/, '')
    const tab = tabs.value.find(tb => {
      const tabBase = tb.name.replace(/\.scad$/, '')
      return tabBase === baseName || tabBase === name || tb.name === name
    })
    return tab ? tab.code : null
  })

  // Tint ghost meshes: set alpha=0.2 and a blue tint
  const ghostMeshes = ghostResult.meshes.map(m => ({
    ...m,
    color: [0.3, 0.5, 1.0, 0.2] as [number, number, number, number],
  }))

  // Merge current meshes with ghost meshes
  const currentMeshes = [...lastParsedMeshes, ...ghostMeshes]
  renderer.setMeshes(currentMeshes)
  // Compute comparison statistics
  computeGhostStats(ghostResult.meshes)
}

function computeGhostStats(meshes: MeshData[]) {
  // Tri count
  ghostTriCount.value = meshes.reduce((s, m) => s + m.indices.length / 3, 0)
  // Volume (same algorithm as computeStatistics)
  let vol = 0
  let minX = Infinity, minY = Infinity, minZ = Infinity
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
  for (const m of meshes) {
    if (m.color[3] < 0.99) continue
    const v = m.vertices
    const idx = m.indices
    const tf = m.transform
    const wx = (vi: number) => {
      const x = v[vi * 6], y = v[vi * 6 + 1], z = v[vi * 6 + 2]
      return [
        tf[0] * x + tf[1] * y + tf[2] * z + tf[3],
        tf[4] * x + tf[5] * y + tf[6] * z + tf[7],
        tf[8] * x + tf[9] * y + tf[10] * z + tf[11],
      ] as [number, number, number]
    }
    let meshVol = 0
    for (let i = 0; i < idx.length; i += 3) {
      const v0 = wx(idx[i]), v1 = wx(idx[i + 1]), v2 = wx(idx[i + 2])
      const cx = v1[1] * v2[2] - v1[2] * v2[1]
      const cy = v1[2] * v2[0] - v1[0] * v2[2]
      const cz = v1[0] * v2[1] - v1[1] * v2[0]
      meshVol += (v0[0] * cx + v0[1] * cy + v0[2] * cz) / 6
    }
    vol += Math.abs(meshVol)
    // Bounding box
    for (let i = 0; i < v.length; i += 6) {
      const px = tf[0] * v[i] + tf[1] * v[i+1] + tf[2] * v[i+2] + tf[3]
      const py = tf[4] * v[i] + tf[5] * v[i+1] + tf[6] * v[i+2] + tf[7]
      const pz = tf[8] * v[i] + tf[9] * v[i+1] + tf[10] * v[i+2] + tf[11]
      if (px < minX) minX = px; if (px > maxX) maxX = px
      if (py < minY) minY = py; if (py > maxY) maxY = py
      if (pz < minZ) minZ = pz; if (pz > maxZ) maxZ = pz
    }
  }
  ghostVolume.value = vol
  ghostBoundsSize.value = [
    maxX > minX ? maxX - minX : 0,
    maxY > minY ? maxY - minY : 0,
    maxZ > minZ ? maxZ - minZ : 0,
  ]
  // Also compute current model stats for comparison
  computeStatistics()
}

/* ── Feature: Notification Badge for Updates ── */
const APP_VERSION = 9 // Increment when adding major features
const hasNewFeatures = ref(false)

function checkVersionBadge() {
  const lastSeen = parseInt(localStorage.getItem('scad-last-version') || '0')
  hasNewFeatures.value = lastSeen < APP_VERSION
}

function dismissNewFeatures() {
  localStorage.setItem('scad-last-version', String(APP_VERSION))
  hasNewFeatures.value = false
}

// Check on load
checkVersionBadge()

const WHATS_NEW_ITEMS = [
  { version: 9, items: {
    ru: ['Примитивы lattice(), slot(), cross(), maze(), fibonacci_sphere()', 'Редактор тем оформления', 'Площадка с интерактивными уроками', 'Подсказки по ошибкам парсера'],
    en: ['lattice(), slot(), cross(), maze(), fibonacci_sphere() primitives', 'Custom theme editor', 'Interactive playground with tutorials', 'Error explanation panel'],
  }},
  { version: 8, items: {
    ru: ['Примитив star()', 'Примитив thread()', 'Сравнение моделей (призрак)', 'Пресеты конфигурации', 'Интерактивный тур', 'Навигация с клавиатуры (Numpad)'],
    en: ['star() primitive', 'thread() primitive', 'Model comparison stats (ghost)', 'Configuration presets', 'Interactive tour', 'Keyboard viewport navigation (Numpad)'],
  }},
  { version: 7, items: {
    ru: ['Инструмент измерения расстояний', 'Режим призрака для сравнения моделей', 'Примитив torus()', 'Примитив helix()', 'Печать кода'],
    en: ['Distance measurement tool', 'Ghost mode for model comparison', 'torus() primitive', 'helix() primitive', 'Print code'],
  }},
  { version: 6, items: {
    ru: ['Галерея примеров', 'Сниппеты', 'Статистика кода'],
    en: ['Example gallery', 'Snippets', 'Code statistics'],
  }},
]

/* ── Feature: Print Code ── */
function printCode() {
  const codeText = code.value
  const tabName = activeTab.value.name
  const now = new Date().toLocaleString()
  // Syntax highlight: simple token-based coloring
  const highlighted = codeText
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/(\/\/.*)$/gm, '<span style="color:#6a6a7a;font-style:italic">$1</span>')
    .replace(/\b(module|function|if|else|for|let|use|include|union|difference|intersection|linear_extrude|rotate_extrude|hull|minkowski|render|echo|assert|projection|import)\b/g, '<span style="color:#1a6dd4;font-weight:600">$1</span>')
    .replace(/\b(cube|sphere|cylinder|polygon|circle|square|text|polyhedron|surface|offset|torus|helix|color|translate|rotate|scale|mirror|resize|multmatrix)\b/g, '<span style="color:#1a8a99;font-weight:600">$1</span>')
    .replace(/\b(\d+\.?\d*([eE][+-]?\d+)?)\b/g, '<span style="color:#c5600a">$1</span>')
    .replace(/\b(true|false)\b/g, '<span style="color:#9040b0">$1</span>')
    .replace(/"([^"]*)"/g, '<span style="color:#2a8c3a">"$1"</span>')

  const lines = highlighted.split('\n')
  const lineNumbered = lines.map((line, i) =>
    `<tr><td style="color:#999;text-align:right;padding-right:12px;user-select:none;min-width:30px">${i + 1}</td><td style="white-space:pre-wrap;word-break:break-all">${line || ' '}</td></tr>`
  ).join('\n')

  const html = `<!DOCTYPE html>
<html>
<head>
  <title>${escapeHtml(tabName)} - OpenSCAD</title>
  <style>
    body { font-family: 'Courier New', monospace; font-size: 11pt; margin: 20px; }
    h2 { font-family: sans-serif; margin-bottom: 4px; }
    .meta { color: #777; font-family: sans-serif; font-size: 9pt; margin-bottom: 16px; }
    table { border-collapse: collapse; width: 100%; }
    td { vertical-align: top; padding: 1px 4px; font-size: 10pt; line-height: 1.4; }
    @media print {
      body { margin: 10mm; }
    }
  </style>
</head>
<body>
  <h2>${escapeHtml(tabName)}</h2>
  <div class="meta">${escapeHtml(now)}</div>
  <table>${lineNumbered}</table>
  <script>window.print();<\/script>
</body>
</html>`

  const w = window.open('', '_blank')
  if (w) {
    w.document.write(html)
    w.document.close()
  }
}

/* ── Share Link ── */
const showCopied = ref(false)

function shareLink() {
  const encoded = btoa(encodeURIComponent(code.value))
  const url = window.location.origin + window.location.pathname + '#code=' + encoded
  navigator.clipboard.writeText(url).then(() => {
    showCopied.value = true
    setTimeout(() => { showCopied.value = false }, 1500)
    addToast(t('copied'), 'success')
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
    addToast(t('copied'), 'success')
  })
}

function loadFromHash() {
  const hash = window.location.hash
  if (hash.startsWith('#code=')) {
    let decoded: string
    try {
      const encoded = hash.slice(6)
      decoded = decodeURIComponent(atob(encoded))
    } catch {
      addToast(t('shareInvalid'), 'error')
      history.replaceState(null, '', window.location.pathname)
      return
    }
    if (decoded.length > 1_000_000) {
      addToast(t('shareInvalid'), 'error')
      history.replaceState(null, '', window.location.pathname)
      return
    }
    // Only assign after successful validation
    code.value = decoded
    // Clear hash after loading
    history.replaceState(null, '', window.location.pathname)
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
  const parsed = safeParse<RecentEntry[]>(localStorage.getItem('scad-recent'), [])
  recentFiles.value = Array.isArray(parsed) ? parsed : []
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
const showMinimap = ref(localStorage.getItem('scad-pref-showMinimap') !== 'false')
const minimapCanvasRef = ref<HTMLCanvasElement | null>(null)
let minimapDebounce: ReturnType<typeof setTimeout> | null = null
const minimapDragging = ref(false)

const MINIMAP_KEYWORDS = new Set([
  'cube','sphere','cylinder','polygon','translate','rotate','scale','color',
  'difference','union','intersection','mirror','module','function',
  'if','else','for','let','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','each','echo','assert',
  'use','include',
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
  prefShowMinimap.value = showMinimap.value
  if (showMinimap.value) nextTick(renderMinimap)
}

/* ── Autocomplete ── */
const AUTOCOMPLETE_KEYWORDS = [
  'cube','sphere','cylinder','polygon','translate','rotate','scale','mirror','color',
  'difference','union','intersection','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','module','function','for','if','else',
  'let','each','echo','assert','use','include',
  '$fn','$fa','$fs','true','false','undef','PI',
  'prism','cone','capsule','gear','radial_array','linear_array',
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

/* ── Hover Documentation Tooltips ── */
interface DocEntry {
  sig: string
  desc: { ru: string; en: string }
}

const OPENSCAD_DOCS: Record<string, DocEntry> = {
  cube: { sig: 'cube(size, center)', desc: { ru: 'Прямоугольный параллелепипед', en: 'Creates a rectangular box' } },
  sphere: { sig: 'sphere(r|d, $fn)', desc: { ru: 'Сфера', en: 'Creates a sphere' } },
  cylinder: { sig: 'cylinder(h, r|r1/r2, center, $fn)', desc: { ru: 'Цилиндр или конус', en: 'Creates a cylinder or cone' } },
  translate: { sig: 'translate([x, y, z])', desc: { ru: 'Перемещение объекта', en: 'Moves child objects' } },
  rotate: { sig: 'rotate([x, y, z]) or rotate(a, v)', desc: { ru: 'Поворот объекта', en: 'Rotates child objects' } },
  scale: { sig: 'scale([x, y, z])', desc: { ru: 'Масштабирование объекта', en: 'Scales child objects' } },
  color: { sig: 'color(c, alpha) or color([r,g,b,a])', desc: { ru: 'Цвет объекта', en: 'Sets the color of child objects' } },
  difference: { sig: 'difference() { ... }', desc: { ru: 'Вычитание: первый минус остальные', en: 'Subtracts subsequent children from the first' } },
  union: { sig: 'union() { ... }', desc: { ru: 'Объединение объектов', en: 'Combines all child objects' } },
  intersection: { sig: 'intersection() { ... }', desc: { ru: 'Пересечение объектов', en: 'Keeps only the overlap of children' } },
  linear_extrude: { sig: 'linear_extrude(height, twist, slices, $fn)', desc: { ru: 'Линейная экструзия 2D-формы', en: 'Extrudes a 2D shape along Z axis' } },
  polygon: { sig: 'polygon(points, paths)', desc: { ru: '2D-многоугольник', en: 'Creates a 2D polygon' } },
  mirror: { sig: 'mirror([x, y, z])', desc: { ru: 'Зеркальное отражение', en: 'Mirrors child objects' } },
  hull: { sig: 'hull() { ... }', desc: { ru: 'Выпуклая оболочка', en: 'Creates convex hull of children' } },
  minkowski: { sig: 'minkowski() { ... }', desc: { ru: 'Сумма Минковского', en: 'Minkowski sum of children' } },
  rotate_extrude: { sig: 'rotate_extrude(angle, $fn)', desc: { ru: 'Вращательная экструзия', en: 'Rotates a 2D shape around Z axis' } },
  assert: { sig: 'assert(condition, message)', desc: { ru: 'Проверка условия, ошибка если ложь', en: 'Checks condition, logs error if false' } },
  use: { sig: 'use <filename>', desc: { ru: 'Импорт модулей из другого файла/вкладки', en: 'Import modules from another file/tab' } },
  include: { sig: 'include <filename>', desc: { ru: 'Включение кода из другого файла/вкладки', en: 'Include code from another file/tab' } },
  polyhedron: { sig: 'polyhedron(points, faces)', desc: { ru: 'Многогранник из точек и граней', en: 'Creates a polyhedron from points and faces' } },
  offset: { sig: 'offset(r|delta, chamfer)', desc: { ru: 'Смещение 2D-контура', en: 'Offsets a 2D outline inward or outward' } },
  projection: { sig: 'projection(cut)', desc: { ru: 'Проекция 3D на плоскость XY', en: 'Projects 3D geometry onto XY plane' } },
  prism: { sig: 'prism(sides, r, h, center)', desc: { ru: 'Правильная призма с N сторонами', en: 'Regular N-sided prism' } },
  cone: { sig: 'cone(r, h, center, $fn)', desc: { ru: 'Конус (цилиндр с r2=0)', en: 'Cone (cylinder with r2=0)' } },
  capsule: { sig: 'capsule(r, h, center, $fn)', desc: { ru: 'Капсула (цилиндр с полусферами)', en: 'Capsule (cylinder with hemisphere caps)' } },
  gear: { sig: 'gear(teeth, mod, thickness, $fn)', desc: { ru: 'Упрощённая прямозубая шестерня', en: 'Simplified spur gear' } },
  radial_array: { sig: 'radial_array(count, r) { ... }', desc: { ru: 'Круговой массив дочерних объектов', en: 'Radial array of children around a circle' } },
  linear_array: { sig: 'linear_array(count, spacing) { ... }', desc: { ru: 'Линейный массив дочерних объектов', en: 'Linear array of children along a direction' } },
}

const hoverDocVisible = ref(false)
const hoverDocContent = ref({ sig: '', desc: '' })
const hoverDocX = ref(0)
const hoverDocY = ref(0)
let hoverDocTimeout: ReturnType<typeof setTimeout> | null = null

function onEditorMouseMove(e: MouseEvent) {
  const el = textareaRef.value
  if (!el) return

  // Clear any previous auto-hide
  if (hoverDocTimeout) clearTimeout(hoverDocTimeout)

  // Calculate approximate character position from mouse coordinates
  const rect = el.getBoundingClientRect()
  const style = getComputedStyle(el)
  const lineHeight = parseFloat(style.lineHeight) || 20
  const fontSize = parseFloat(style.fontSize) || 13
  const charWidth = fontSize * 0.6 // approximate monospace char width
  const paddingTop = parseFloat(style.paddingTop) || 12
  const paddingLeft = parseFloat(style.paddingLeft) || 12

  const relX = e.clientX - rect.left - paddingLeft + el.scrollLeft
  const relY = e.clientY - rect.top - paddingTop + el.scrollTop

  const lineIdx = Math.floor(relY / lineHeight)
  const colIdx = Math.floor(relX / charWidth)

  const lines = code.value.split('\n')
  if (lineIdx < 0 || lineIdx >= lines.length) { hoverDocVisible.value = false; return }
  const line = lines[lineIdx]
  if (colIdx < 0 || colIdx >= line.length) { hoverDocVisible.value = false; return }

  // Extract word at position
  let wordStart = colIdx
  let wordEnd = colIdx
  while (wordStart > 0 && /[a-zA-Z_]/.test(line[wordStart - 1])) wordStart--
  while (wordEnd < line.length && /[a-zA-Z_]/.test(line[wordEnd])) wordEnd++
  const word = line.substring(wordStart, wordEnd)

  const doc = OPENSCAD_DOCS[word]
  if (!doc) { hoverDocVisible.value = false; return }

  hoverDocContent.value = { sig: doc.sig, desc: (doc.desc as any)[lang.value] || doc.desc.en }
  const codeArea = el.closest('.code-area')
  const codeAreaRect = codeArea ? codeArea.getBoundingClientRect() : rect
  hoverDocX.value = e.clientX - codeAreaRect.left + 8
  hoverDocY.value = e.clientY - codeAreaRect.top - 40
  hoverDocVisible.value = true

  // Auto-hide after 3 seconds
  hoverDocTimeout = setTimeout(() => { hoverDocVisible.value = false }, 3000)
}

function onEditorMouseLeave() {
  hoverDocVisible.value = false
  if (hoverDocTimeout) { clearTimeout(hoverDocTimeout); hoverDocTimeout = null }
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
  'cube','sphere','cylinder','polygon','translate','rotate','scale','color',
  'difference','union','intersection','mirror','module','function',
  'if','else','for','let','linear_extrude','rotate_extrude',
  'hull','minkowski','multmatrix','each','echo','assert',
  'use','include',
])
const BOOLEANS = new Set(['true','false','undef'])
const SPECIALS = new Set(['$fn','$fa','$fs'])

function highlightCode(src: string, bmA: number, bmB: number, fMatches: { start: number; end: number }[], fActiveIdx: number, errPos: number, occMatches: { start: number; end: number }[] = []): string {
  const esc = (s: string) => s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')

  // Build a set of find-match ranges for quick lookup
  const findSet = new Map<number, { end: number; active: boolean }>()
  for (let fi = 0; fi < fMatches.length; fi++) {
    findSet.set(fMatches[fi].start, { end: fMatches[fi].end, active: fi === fActiveIdx })
  }

  // Build a set of occurrence-highlight ranges
  const occSet = new Map<number, number>()
  for (const om of occMatches) {
    occSet.set(om.start, om.end)
  }

  const result: string[] = []
  let i = 0
  const len = src.length

  const isBracketMatch = (pos: number) => pos === bmA || pos === bmB

  // Track if we are inside a find-match highlight span
  let inFindMatch = false
  let findMatchEnd = 0
  let findMatchActive = false

  // Track occurrence highlights
  let inOccMatch = false
  let occMatchEnd = 0

  while (i < len) {
    // Check if we enter a find-match range
    const fm = findSet.get(i)
    if (fm && !inFindMatch) {
      inFindMatch = true
      findMatchEnd = fm.end
      findMatchActive = fm.active
      result.push(`<span class="${findMatchActive ? 'find-match-active' : 'find-match'}">`)
    }

    // Check if we enter an occurrence-match range
    const om = occSet.get(i)
    if (om && !inOccMatch && !inFindMatch) {
      inOccMatch = true
      occMatchEnd = om
      result.push('<span class="occ-match">')
    }

    // Close find-match if we passed the end
    if (inFindMatch && i >= findMatchEnd) {
      result.push('</span>')
      inFindMatch = false
    }

    // Close occurrence-match if we passed the end
    if (inOccMatch && i >= occMatchEnd) {
      result.push('</span>')
      inOccMatch = false
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
        // Inline color swatch for color(...) calls
        if (word === 'color') {
          // Look ahead past optional whitespace for the opening paren.
          let k = j
          while (k < len && (src[k] === ' ' || src[k] === '\t')) k++
          if (src[k] === '(') {
            let a = k + 1
            while (a < len && (src[a] === ' ' || src[a] === '\t')) a++
            const css = colorArgToCss(src.slice(a, Math.min(a + 60, len)))
            if (css) {
              // Emit the whitespace + paren we consumed, then the swatch.
              result.push(esc(src.slice(j, k + 1)))
              result.push(`<span class="color-swatch" style="background:${css}"></span>`)
              i = k + 1
              continue
            }
          }
        }
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
  // Close any dangling find-match or occurrence-match span
  if (inFindMatch) result.push('</span>')
  if (inOccMatch) result.push('</span>')

  // Insert error underline at error position
  if (errPos >= 0 && errPos < len) {
    // Find the word or ~10 chars around the error position
    let errEnd = errPos
    if (/[a-zA-Z_0-9]/.test(src[errPos])) {
      while (errEnd < len && /[a-zA-Z_0-9]/.test(src[errEnd])) errEnd++
    } else {
      errEnd = Math.min(errPos + 10, len)
    }
    // We need to place the underline in the result. Count characters in src
    // to find where in the HTML result the error position falls.
    // Build a simple char-to-result-index map.
    let html = result.join('')
    let srcIdx = 0
    let htmlIdx = 0
    const htmlLen = html.length

    // Walk the HTML, skipping tags, to find where srcIdx == errPos
    let insertStart = -1
    let insertEnd = -1
    while (htmlIdx < htmlLen && srcIdx <= errEnd) {
      if (srcIdx === errPos && insertStart === -1) insertStart = htmlIdx
      if (srcIdx === errEnd) { insertEnd = htmlIdx; break }
      if (html[htmlIdx] === '<') {
        // Skip HTML tag
        while (htmlIdx < htmlLen && html[htmlIdx] !== '>') htmlIdx++
        htmlIdx++ // skip '>'
      } else if (html[htmlIdx] === '&') {
        // HTML entity counts as 1 src char
        while (htmlIdx < htmlLen && html[htmlIdx] !== ';') htmlIdx++
        htmlIdx++
        srcIdx++
      } else {
        htmlIdx++
        srcIdx++
      }
    }
    if (insertEnd === -1) insertEnd = htmlIdx
    if (insertStart >= 0 && insertEnd > insertStart) {
      const before = html.substring(0, insertStart)
      const errContent = html.substring(insertStart, insertEnd)
      const after = html.substring(insertEnd)
      html = before + '<span class="error-squiggly">' + errContent + '</span>' + after
    }
    return html + '\n'
  }

  return result.join('') + '\n'
}

function addIndentGuides(html: string, tabSize: number): string {
  const lines = html.split('\n')
  const result: string[] = []
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    // Count leading spaces in the raw text (ignoring HTML tags)
    const plain = line.replace(/<[^>]*>/g, '')
    let spaces = 0
    for (let j = 0; j < plain.length; j++) {
      if (plain[j] === ' ') spaces++
      else break
    }
    if (spaces >= tabSize) {
      const guideCount = Math.floor(spaces / tabSize)
      let guideHtml = ''
      for (let g = 0; g < guideCount; g++) {
        guideHtml += `<span class="indent-guide" style="left:${g * tabSize}ch"></span>`
      }
      result.push(`<span class="indent-line">${guideHtml}</span>${line}`)
    } else {
      result.push(line)
    }
  }
  return result.join('\n')
}

const highlightedCode = computed(() => {
  const raw = highlightCode(code.value, bracketMatchA.value, bracketMatchB.value, findMatches.value, findMatchIndex.value, errorCharPos.value, occurrencePositions.value)
  const withIndent = addIndentGuides(raw, prefTabSize.value)
  // Apply code folding: hide folded lines, add placeholder
  if (foldedLines.value.size === 0) return withIndent
  const lines = withIndent.split('\n')
  const result: string[] = []
  for (let i = 0; i < lines.length; i++) {
    if (isLineHidden(i)) continue
    result.push(lines[i])
    if (foldedLines.value.has(i)) {
      // Append fold placeholder on same line
      result[result.length - 1] += '<span class="fold-placeholder"> ... </span>'
    }
  }
  return result.join('\n')
})

/* ── Line numbers ── */
const lineCount = computed(() => code.value.split('\n').length)
const lineNumbers = computed(() => {
  const n = lineCount.value
  const errL = errorLine.value
  const nums: string[] = []
  const diffs = showDiff.value ? diffLines.value : null
  const codeLines = gutterDecorationsEnabled.value ? code.value.split('\n') : null
  for (let i = 1; i <= n; i++) {
    if (isLineHidden(i - 1)) continue
    const foldable = isFoldable(i - 1)
    const folded = foldedLines.value.has(i - 1)
    let prefix = ''
    if (foldable) {
      prefix = `<span class="fold-marker" data-line="${i - 1}">${folded ? '▶' : '▼'}</span>`
    }
    // Diff indicator
    let diffPrefix = ''
    if (diffs) {
      const dl = diffs.find(d => d.lineNum === i)
      if (dl) {
        if (dl.type === 'added') diffPrefix = '<span class="diff-indicator diff-ind-added"></span>'
        else if (dl.type === 'modified') diffPrefix = '<span class="diff-indicator diff-ind-modified"></span>'
      }
    }
    // Gutter decoration icon
    let gutterIcon = ''
    if (codeLines && i - 1 < codeLines.length) {
      gutterIcon = getGutterIcon(codeLines[i - 1])
    }
    if (i === errL) {
      nums.push(`${diffPrefix}${prefix}${gutterIcon}<span class="line-error">${i}</span>`)
    } else {
      nums.push(`${diffPrefix}${prefix}${gutterIcon}${i}`)
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

/* ── Code Folding functions ── */
function computeFolds() {
  const src = code.value
  const lines = src.split('\n')
  const ranges = new Map<number, number>()
  const stack: number[] = []
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    let inStr = false
    for (let j = 0; j < line.length; j++) {
      const ch = line[j]
      if (ch === '"' && (j === 0 || line[j - 1] !== '\\')) { inStr = !inStr; continue }
      if (inStr) continue
      if (ch === '/' && j + 1 < line.length && line[j + 1] === '/') break // rest is comment
      if (ch === '{') {
        stack.push(i)
      } else if (ch === '}') {
        if (stack.length > 0) {
          const start = stack.pop()!
          if (i > start) {
            ranges.set(start, i)
          }
        }
      }
    }
  }
  foldRanges.value = ranges
  // Remove stale folds
  const newFolded = new Set<number>()
  for (const s of foldedLines.value) {
    if (ranges.has(s)) newFolded.add(s)
  }
  foldedLines.value = newFolded
}

function toggleFold(lineIdx: number) {
  const newSet = new Set(foldedLines.value)
  if (newSet.has(lineIdx)) {
    newSet.delete(lineIdx)
  } else {
    newSet.add(lineIdx)
  }
  foldedLines.value = newSet
}

function isFoldable(lineIdx: number): boolean {
  return foldRanges.value.has(lineIdx)
}

function isLineHidden(lineIdx: number): boolean {
  for (const startLine of foldedLines.value) {
    const endLine = foldRanges.value.get(startLine)
    if (endLine !== undefined && lineIdx > startLine && lineIdx <= endLine) {
      return true
    }
  }
  return false
}

function onLineNumClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (target.classList.contains('fold-marker')) {
    const lineIdx = parseInt(target.dataset.line || '-1')
    if (lineIdx >= 0) {
      toggleFold(lineIdx)
    }
  }
}

/* ── Color Picker functions ── */
function onHighlightClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  const swatchEl = target.classList.contains('color-swatch') ? target : target.closest('.color-swatch') as HTMLElement | null
  if (!swatchEl) return
  e.preventDefault()
  e.stopPropagation()
  const src = code.value
  const colorCalls = findColorCalls(src)
  if (colorCalls.length === 0) return
  const codeArea = swatchEl.closest('.code-area')
  const rect = codeArea ? codeArea.getBoundingClientRect() : { left: 0, top: 0 }
  colorPickerX.value = e.clientX - rect.left + 10
  colorPickerY.value = e.clientY - rect.top + 10
  const bg = swatchEl.style.background
  colorPickerValue.value = cssToHex(bg) || '#808080'
  const allSwatches = (swatchEl.closest('.highlight-layer') || document).querySelectorAll('.color-swatch')
  let swatchIndex = -1
  allSwatches.forEach((s, i) => { if (s === swatchEl) swatchIndex = i })
  if (swatchIndex >= 0 && swatchIndex < colorCalls.length) {
    colorPickerMatch = colorCalls[swatchIndex]
  } else if (colorCalls.length === 1) {
    colorPickerMatch = colorCalls[0]
  } else {
    colorPickerMatch = null
  }
  colorPickerVisible.value = true
}

function findColorCalls(src: string): { start: number; end: number }[] {
  const results: { start: number; end: number }[] = []
  const re = /\bcolor\s*\(/g
  let m: RegExpExecArray | null
  while ((m = re.exec(src)) !== null) {
    const parenStart = m.index + m[0].length
    let depth = 1
    let i = parenStart
    while (i < src.length && depth > 0) {
      if (src[i] === '(') depth++
      if (src[i] === ')') depth--
      i++
    }
    const argStart = parenStart
    let argEnd = i - 1
    let d = 0
    for (let j = argStart; j < argEnd; j++) {
      if (src[j] === '(' || src[j] === '[') d++
      if (src[j] === ')' || src[j] === ']') d--
      if (src[j] === ',' && d === 0) { argEnd = j; break }
    }
    results.push({ start: argStart, end: argEnd })
  }
  return results
}

function cssToHex(css: string): string | null {
  if (!css) return null
  const rgbaMatch = css.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
  if (rgbaMatch) {
    const r = parseInt(rgbaMatch[1]).toString(16).padStart(2, '0')
    const g = parseInt(rgbaMatch[2]).toString(16).padStart(2, '0')
    const b = parseInt(rgbaMatch[3]).toString(16).padStart(2, '0')
    return '#' + r + g + b
  }
  if (css.startsWith('#')) return css
  const name = css.trim().toLowerCase()
  return COLOR_NAMES[name] || null
}

function onColorPickerChange(e: Event) {
  const newColor = (e.target as HTMLInputElement).value
  if (!colorPickerMatch) return
  const src = code.value
  const oldArg = src.substring(colorPickerMatch.start, colorPickerMatch.end).trim()
  let replacement: string
  if (oldArg.startsWith('[')) {
    const r = parseInt(newColor.substring(1, 3), 16) / 255
    const g = parseInt(newColor.substring(3, 5), 16) / 255
    const b = parseInt(newColor.substring(5, 7), 16) / 255
    replacement = `[${r.toFixed(2)}, ${g.toFixed(2)}, ${b.toFixed(2)}]`
  } else {
    replacement = '"' + newColor + '"'
  }
  code.value = src.substring(0, colorPickerMatch.start) + replacement + src.substring(colorPickerMatch.end)
  colorPickerMatch = { start: colorPickerMatch.start, end: colorPickerMatch.start + replacement.length }
}

function closeColorPicker() {
  colorPickerVisible.value = false
  colorPickerMatch = null
}

const colorPaletteItems = computed(() => {
  // Subset of most-used colors for the palette
  const subset: Record<string, string> = {}
  const keys = ['red', 'green', 'blue', 'yellow', 'cyan', 'magenta', 'white', 'black',
    'orange', 'gray', 'pink', 'purple', 'brown', 'lime', 'navy', 'teal',
    'gold', 'indigo', 'violet', 'salmon', 'coral', 'turquoise', 'crimson', 'chocolate']
  for (const k of keys) {
    if (COLOR_NAMES[k]) subset[k] = COLOR_NAMES[k]
  }
  return subset
})

function insertColorName(name: string) {
  if (!colorPickerMatch) return
  const src = code.value
  const replacement = '"' + name + '"'
  code.value = src.substring(0, colorPickerMatch.start) + replacement + src.substring(colorPickerMatch.end)
  colorPickerMatch = { start: colorPickerMatch.start, end: colorPickerMatch.start + replacement.length }
  // Update the native picker value too
  if (COLOR_NAMES[name]) {
    colorPickerValue.value = COLOR_NAMES[name]
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
      pushUndoSnapshot() // snapshot before file open
      const content = reader.result as string
      code.value = content
      activeTab.value.savedCode = content
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
  activeTab.value.savedCode = code.value
  addToRecent(activeTab.value.name, code.value)
  addToast(t('save') + ': ' + (activeTab.value.name || 'model') + '.scad', 'success')
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
  if (!file) return
  if (file.name.toLowerCase().endsWith('.stl')) {
    importSTLFile(file)
    return
  }
  if (!file.name.endsWith('.scad')) return
  const reader = new FileReader()
  reader.onload = () => {
    const content = reader.result as string
    code.value = content
    activeTab.value.savedCode = content
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
  el.scrollTo({ top: Math.max(0, (lineNum - 3) * lineHeight), behavior: 'smooth' })
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
  const meta = screenshotMeta.value
  const anyMeta = meta.modelName || meta.dimensions || meta.triangles || meta.date
  if (anyMeta) {
    screenshotWithMeta(1)
  } else {
    renderer.screenshot()
  }
  addToast(t('screenshot'), 'success')
}

/* ── Toggle wireframe ── */
function toggleWireframe() {
  if (!renderer) return
  // Use render mode system: toggle between 'solid' and 'wireframe'
  if (activeRenderMode.value === 'wireframe') {
    setRenderMode('solid')
  } else {
    setRenderMode('wireframe')
  }
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
  pushUndoSnapshot() // snapshot before format
  const src = code.value
  const lines = src.split('\n')
  const result: string[] = []
  let indent = 0
  const INDENT = ' '.repeat(prefTabSize.value)

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

/* ── Orientation Cube / Navigation Gizmo ── */
interface GizmoAxis {
  id: string        // '+x' | '-x' | '+y' | '-y' | '+z' | '-z'
  x: number         // projected screen x within the gizmo box
  y: number         // projected screen y within the gizmo box
  z: number         // camera-space depth (for painter sort)
  color: string
  label: string
  positive: boolean
}

const GIZMO_SIZE = 78               // px box for the gizmo
const GIZMO_R = 26                  // axis projection radius
const gizmoAxes = ref<GizmoAxis[]>([])
let gizmoRAF = 0
let gizmoPollAccum = 0
let gizmoLastTime = 0

// Static axis definitions in world space (unit vectors)
const GIZMO_DEFS: { id: string; v: [number, number, number]; color: string; label: string; positive: boolean }[] = [
  { id: '+x', v: [1, 0, 0],  color: '#e05555', label: 'X', positive: true },
  { id: '-x', v: [-1, 0, 0], color: '#e05555', label: '', positive: false },
  { id: '+y', v: [0, 1, 0],  color: '#44cc55', label: 'Y', positive: true },
  { id: '-y', v: [0, -1, 0], color: '#44cc55', label: '', positive: false },
  { id: '+z', v: [0, 0, 1],  color: '#4488ee', label: 'Z', positive: true },
  { id: '-z', v: [0, 0, -1], color: '#4488ee', label: '', positive: false },
]

function computeGizmo(yaw: number, pitch: number) {
  // Camera basis derived from the renderer's spherical orbit.
  // Eye direction (from target to eye):
  const ex = Math.cos(pitch) * Math.sin(yaw)
  const ey = Math.sin(pitch)
  const ez = Math.cos(pitch) * Math.cos(yaw)
  // Camera forward = -eye (looking toward origin)
  const fx = -ex, fy = -ey, fz = -ez
  // World up
  const upx = 0, upy = 1, upz = 0
  // right = normalize(cross(forward, up))  (screen X)
  let rx = fy * upz - fz * upy
  let ry = fz * upx - fx * upz
  let rz = fx * upy - fy * upx
  let rl = Math.hypot(rx, ry, rz) || 1
  rx /= rl; ry /= rl; rz /= rl
  // trueUp = cross(right, forward)  (screen Y, pointing up)
  const ux = ry * fz - rz * fy
  const uy = rz * fx - rx * fz
  const uz = rx * fy - ry * fx

  const cx = GIZMO_SIZE / 2
  const cy = GIZMO_SIZE / 2
  const result: GizmoAxis[] = []
  for (const def of GIZMO_DEFS) {
    const [vx, vy, vz] = def.v
    // Project onto camera basis. screenX uses right, screenY uses up (inverted for CSS),
    // depth uses forward (larger = farther away from camera).
    const sx = vx * rx + vy * ry + vz * rz
    const sy = vx * ux + vy * uy + vz * uz
    const depth = vx * fx + vy * fy + vz * fz
    result.push({
      id: def.id,
      x: cx + sx * GIZMO_R,
      y: cy - sy * GIZMO_R,
      z: depth,
      color: def.color,
      label: def.label,
      positive: def.positive,
    })
  }
  // Painter's sort: farther first so near dots render on top.
  result.sort((a, b) => a.z - b.z)
  gizmoAxes.value = result
}

function updateGizmo() {
  if (!renderer) return
  const now = performance.now()
  if (gizmoLastTime === 0) gizmoLastTime = now
  gizmoPollAccum += now - gizmoLastTime
  gizmoLastTime = now
  // Poll roughly every 50ms to keep it in sync without thrashing reactivity.
  if (gizmoPollAccum >= 50) {
    gizmoPollAccum = 0
    const o = renderer.getOrientation()
    computeGizmo(o.yaw, o.pitch)
  }
  gizmoRAF = requestAnimationFrame(updateGizmo)
}

function snapGizmoAxis(axis: string) {
  if (!renderer) return
  renderer.snapToAxis(axis)
}

/* ── Right-click Context Menu in Viewport ── */
const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
// Track whether a pan/orbit drag occurred between pointerdown and contextmenu
let canvasPointerDownX = 0
let canvasPointerDownY = 0
let canvasDidDrag = false

function onCanvasPointerDown(e: PointerEvent) {
  canvasPointerDownX = e.clientX
  canvasPointerDownY = e.clientY
  canvasDidDrag = false
}

function onCanvasPointerMove(e: PointerEvent) {
  if (e.buttons === 0) return
  const dx = e.clientX - canvasPointerDownX
  const dy = e.clientY - canvasPointerDownY
  // Treat anything beyond a small threshold as a drag (pan/orbit)
  if (dx * dx + dy * dy > 25) canvasDidDrag = true
}

function onCanvasPointerUp(e: PointerEvent) {
  if (measureMode.value && !canvasDidDrag && e.button === 0) {
    onMeasureClick(e)
  }
}

function onCanvasContextMenu(e: MouseEvent) {
  e.preventDefault()
  // Suppress the menu if the user was panning/orbiting (drag with right button)
  if (canvasDidDrag) { canvasDidDrag = false; return }
  const panel = (e.currentTarget as HTMLElement).closest('.canvas-panel') as HTMLElement | null
  const rect = panel ? panel.getBoundingClientRect() : (e.currentTarget as HTMLElement).getBoundingClientRect()
  // Position relative to the canvas panel, clamped so it stays in view.
  const menuW = 200, menuH = 280
  let mx = e.clientX - rect.left
  let my = e.clientY - rect.top
  if (mx + menuW > rect.width) mx = rect.width - menuW - 4
  if (my + menuH > rect.height) my = Math.max(4, rect.height - menuH - 4)
  contextMenuX.value = Math.max(4, mx)
  contextMenuY.value = Math.max(4, my)
  showContextMenu.value = true
}

function onCloseContextMenu() {
  showContextMenu.value = false
}

interface ContextMenuItem {
  id: string
  label: () => string
  icon: string   // path data for a 24x24 stroke SVG
  action: () => void
}

const contextMenuItems: ContextMenuItem[] = [
  { id: 'reset', label: () => t('ctxResetView'), icon: 'M3 12a9 9 0 1 0 9-9 9 9 0 0 0-6.4 2.6L3 8 M3 3v5h5', action: () => setView('reset') },
  { id: 'fit', label: () => t('ctxFitScreen'), icon: 'M15 3h6v6 M9 21H3v-6 M21 3l-7 7 M3 21l7-7', action: () => { renderer?.autoFitAll() } },
  { id: 'screenshot', label: () => t('ctxScreenshot'), icon: 'M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z M12 17a4 4 0 1 0 0-8 4 4 0 0 0 0 8z', action: () => takeScreenshot() },
  { id: 'copyImage', label: () => t('ctxCopyImage'), icon: 'M9 9h13v13H9z M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1', action: () => copyCanvasToClipboard() },
  { id: 'wireframe', label: () => t('ctxToggleWireframe'), icon: 'M12 2 2 7l10 5 10-5-10-5z M2 17l10 5 10-5 M2 12l10 5 10-5', action: () => toggleWireframe() },
  { id: 'grid', label: () => t('ctxToggleGrid'), icon: 'M3 3h18v18H3z M3 9h18 M3 15h18 M9 3v18 M15 3v18', action: () => toggleGrid() },
  { id: 'exportStl', label: () => t('ctxExportStl'), icon: 'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4 M7 10l5 5 5-5 M12 15V3', action: () => doExportSTL() },
]

function runContextMenuItem(item: ContextMenuItem) {
  showContextMenu.value = false
  nextTick(() => item.action())
}

/* ── Inline Color Swatches in Editor ── */
// Map of named CSS colors → hex (subset commonly used in OpenSCAD)
const COLOR_NAMES: Record<string, string> = {
  red: '#ff0000', green: '#008000', blue: '#0000ff', yellow: '#ffff00',
  cyan: '#00ffff', magenta: '#ff00ff', white: '#ffffff', black: '#000000',
  gray: '#808080', grey: '#808080', orange: '#ffa500', purple: '#800080',
  pink: '#ffc0cb', brown: '#a52a2a', lime: '#00ff00', navy: '#000080',
  teal: '#008080', olive: '#808000', maroon: '#800000', silver: '#c0c0c0',
  gold: '#ffd700', indigo: '#4b0082', violet: '#ee82ee', salmon: '#fa8072',
  khaki: '#f0e68c', coral: '#ff7f50', turquoise: '#40e0d0', tan: '#d2b48c',
  beige: '#f5f5dc', ivory: '#fffff0', crimson: '#dc143c', chocolate: '#d2691e',
  tomato: '#ff6347', orchid: '#da70d6', plum: '#dda0dd', azure: '#f0ffff',
  aqua: '#00ffff', fuchsia: '#ff00ff', transparent: '#00000000',
}

function clamp01(n: number): number {
  return Math.max(0, Math.min(1, n))
}

// Convert an OpenSCAD color() argument to a CSS color string, or null if unknown.
function colorArgToCss(arg: string): string | null {
  arg = arg.trim()
  // Named color in quotes: "red"
  const nameMatch = arg.match(/^"([^"]+)"/)
  if (nameMatch) {
    const name = nameMatch[1].toLowerCase()
    // Hex string like "#ff0000"
    if (/^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(name)) return name
    return COLOR_NAMES[name] ?? null
  }
  // RGB(A) array: [r, g, b] or [r, g, b, a] with 0..1 floats
  const arrMatch = arg.match(/^\[\s*([0-9.]+)\s*,\s*([0-9.]+)\s*,\s*([0-9.]+)\s*(?:,\s*([0-9.]+)\s*)?\]/)
  if (arrMatch) {
    const r = Math.round(clamp01(parseFloat(arrMatch[1])) * 255)
    const g = Math.round(clamp01(parseFloat(arrMatch[2])) * 255)
    const b = Math.round(clamp01(parseFloat(arrMatch[3])) * 255)
    const a = arrMatch[4] !== undefined ? clamp01(parseFloat(arrMatch[4])) : 1
    return `rgba(${r}, ${g}, ${b}, ${a})`
  }
  return null
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
  // Close viewport menus on outside click
  if ((viewMenuOpen.value || renderMenuOpen.value || exportMenuOpen.value) && !target.closest('.vp-menu-wrapper')) {
    closeAllMenus()
  }
  // Close tab context menu on outside click
  if (showTabContextMenu.value && !target.closest('.tab-ctx-menu')) {
    closeTabContextMenu()
  }
  // Close hamburger menu on outside click
  if (hamburgerOpen.value && !target.closest('.hamburger-wrapper')) {
    hamburgerOpen.value = false
  }
  // Close color picker on outside click
  if (colorPickerVisible.value && !target.closest('.color-picker-popup') && !target.closest('.color-swatch')) {
    closeColorPicker()
  }
  // Close export dropdown on outside click
  if (showExportDropdown.value && !target.closest('.export-dropdown-wrapper')) {
    closeExportDropdown()
  }
  // Close snippet panel on outside click
  if (showSnippetPanel.value && !target.closest('.snippet-panel-wrapper')) {
    showSnippetPanel.value = false
  }
}

/* ── Global keyboard handler ── */
function onGlobalKeydown(e: KeyboardEvent) {
  // Custom shortcut capture mode
  if (capturingShortcutId.value) {
    onCaptureKey(e)
    return
  }
  const bindings = SHORTCUT_PRESETS[shortcutPreset.value]
  // Ctrl+Shift+P or F1 or preset command palette: Command Palette
  if (e.key === 'F1' || ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'P') || matchesBinding(e, bindings.commandPalette)) {
    e.preventDefault()
    if (showCommandPalette.value) closeCommandPalette()
    else openCommandPalette()
    return
  }
  // Ctrl+K Z chord: Zen Mode (VS Code style)
  if (zenChordPending && (e.key === 'z' || e.key === 'Z')) {
    e.preventDefault()
    zenChordPending = false
    if (zenChordTimer) { clearTimeout(zenChordTimer); zenChordTimer = null }
    toggleZenMode()
    return
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K') && !e.shiftKey && !e.altKey) {
    e.preventDefault()
    zenChordPending = true
    if (zenChordTimer) clearTimeout(zenChordTimer)
    zenChordTimer = setTimeout(() => { zenChordPending = false }, 1500)
    return
  }
  zenChordPending = false

  // Ctrl+\ : Toggle split editor
  if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
    e.preventDefault()
    toggleSplitMode()
    return
  }
  // Ctrl+G: Go to Line
  if ((e.ctrlKey || e.metaKey) && e.key === 'g') {
    e.preventDefault()
    if (showGoToLine.value) closeGoToLine()
    else openGoToLine()
    return
  }
  // Alt+Z: Toggle Word Wrap
  if (e.altKey && e.key === 'z') {
    e.preventDefault()
    toggleWordWrap()
    return
  }
  // Ctrl+Tab / Ctrl+Shift+Tab: switch tabs
  if ((e.ctrlKey || e.metaKey) && e.key === 'Tab') {
    e.preventDefault()
    if (e.shiftKey) switchToPrevTab()
    else switchToNextTab()
    return
  }
  // Ctrl+W: close current tab
  if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
    e.preventDefault()
    closeCurrentTab()
    return
  }
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
  // Escape to close modal or exit fullscreen or zen mode
  if (e.key === 'Escape') {
    if (zenModeActive.value) { zenModeActive.value = false; return }
    if (viewMenuOpen.value || renderMenuOpen.value || exportMenuOpen.value) { closeAllMenus(); return }
    if (showContextMenu.value) { showContextMenu.value = false; return }
    if (showCommandPalette.value) { closeCommandPalette(); return }
    if (showGoToLine.value) { closeGoToLine(); return }
    if (showHistory.value) { showHistory.value = false; return }
    if (showPreferences.value) { showPreferences.value = false; return }
    if (showFind.value) { closeFindReplace(); return }
    if (showShortcuts.value) { showShortcuts.value = false; return }
    if (showScadReference.value) { showScadReference.value = false; return }
    if (showExampleGallery.value) { closeExampleGallery(); return }
    if (showSnippetPanel.value) { showSnippetPanel.value = false; return }
    if (acVisible.value) { acVisible.value = false; return }
    if (showRecent.value) { showRecent.value = false; return }
    if (showExportDropdown.value) { showExportDropdown.value = false; return }
    if (showBatchResults.value) { showBatchResults.value = false; return }
    if (showShortcutEditor.value) { showShortcutEditor.value = false; return }
    if (measureMode.value) { measureMode.value = false; clearMeasurement(); return }
    if (isFullscreen.value) { isFullscreen.value = false }
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onGlobalKeydown)
  document.addEventListener('click', onDocClick)
  document.addEventListener('click', onCloseContextMenu)
  loadRecentFiles()
  loadFromHash()
  computeFolds()

  // Show tip of the day if not dismissed
  if (!tipOfDayDismissed.value) {
    showTipOfDay.value = true
  }

  // Dynamic favicon: isometric 3D cube SVG
  {
    const svgFavicon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
      <polygon points="16,2 30,10 30,22 16,30 2,22 2,10" fill="#1e1e22" stroke="#4a9eff" stroke-width="1.5"/>
      <polygon points="16,2 30,10 16,18 2,10" fill="#4a9eff" opacity="0.35"/>
      <polygon points="16,18 30,10 30,22 16,30" fill="#4a9eff" opacity="0.2"/>
      <polygon points="16,18 2,10 2,22 16,30" fill="#4a9eff" opacity="0.5"/>
    </svg>`
    const faviconUrl = 'data:image/svg+xml,' + encodeURIComponent(svgFavicon)
    let link = document.querySelector("link[rel='icon']") as HTMLLinkElement | null
    if (!link) {
      link = document.createElement('link')
      link.rel = 'icon'
      document.head.appendChild(link)
    }
    link.type = 'image/svg+xml'
    link.href = faviconUrl
  }

  if (!canvasRef.value) return
  renderer = new WebGPURenderer()
  let ok = false
  try {
    ok = await renderer.init(canvasRef.value)
  } catch {
    ok = false
  }
  if (!ok) { gpuOk.value = false; initFailed.value = true; rendererReady.value = true; return }
  rendererReady.value = true
  deviceLost.value = false

  // Handle device lost
  renderer.onDeviceLost = (reason: string) => {
    deviceLost.value = true
    addConsoleEntry('error', t('deviceLost') + ': ' + reason)
    addToast(t('deviceLostMsg'), 'error', {
      actionLabel: t('reinitialize'),
      duration: 10000,
      action: () => reinitializeWebGPU(),
    })
  }

  // Restore persisted build-plate setting.
  applyBuildPlate()
  // Restore orbit inertia setting
  renderer.setInertia(inertiaEnabled.value)
  // Initialize undo stack with current code
  pushUndoSnapshot(code.value)
  doRender()

  // Start axis label updates
  updateAxisLabels()

  // Start 3D annotation updates
  updateAnnotations()

  // Start orientation gizmo polling
  updateGizmo()

  // Stats polling
  statsInterval = setInterval(() => {
    if (!renderer) return
    fpsVal.value = renderer.getFPS()
    vertexCount.value = renderer.getVertexCount()
    const bounds = renderer.getBounds()
    boundsSize.value = bounds.size
  }, 500)

  // Session auto-save every 30 seconds
  sessionAutoSaveInterval = setInterval(saveSessionBackup, 30000)

  // Check for session to restore (only if tabs match old storage - i.e. user might have lost data)
  checkSessionRestore()
})

onUnmounted(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  document.removeEventListener('click', onDocClick)
  if (debounce) clearTimeout(debounce)
  if (minimapDebounce) clearTimeout(minimapDebounce)
  if (statsInterval) clearInterval(statsInterval)
  if (axisLabelRAF) cancelAnimationFrame(axisLabelRAF)
  if (annotationRAF) cancelAnimationFrame(annotationRAF)
  if (gizmoRAF) cancelAnimationFrame(gizmoRAF)
  if (sessionAutoSaveInterval) clearInterval(sessionAutoSaveInterval)
  if (undoDebounceTimer) clearTimeout(undoDebounceTimer)
  document.removeEventListener('click', onCloseContextMenu)
  renderer?.destroy(); renderer = null
})

watch(code, (v) => {
  // Save tabs
  saveTabs()
  // Recompute fold ranges
  computeFolds()
  // Also keep legacy key for backwards compat
  localStorage.setItem('scad-code', v)
  // Push to undo stack (debounced) on code change from typing
  if (!suppressUndoPush) {
    pushUndoDebounced()
  }
  if (!autoRender.value) return
  if (debounce) clearTimeout(debounce)
  debounce = setTimeout(doRender, prefAutoRenderDelay.value)
  // Debounce minimap render
  if (showMinimap.value) {
    if (minimapDebounce) clearTimeout(minimapDebounce)
    minimapDebounce = setTimeout(renderMinimap, 300)
  }
  // Debounce history snapshot
  if (historyDebounce) clearTimeout(historyDebounce)
  historyDebounce = setTimeout(() => {
    addHistorySnapshot(activeTabId.value, v)
  }, 3000)
})

// Replace standalone `$t` tokens with a value, skipping occurrences inside
// string literals ("..."), line comments (//...) and block comments (/* ... */).
// Pragmatic, single-pass scanner: it is not a full OpenSCAD lexer but correctly
// handles the common cases (escaped quotes, nested-looking comments).
function replaceAnimT(src: string, value: string): string {
  let out = ''
  let i = 0
  const n = src.length
  while (i < n) {
    const c = src[i]
    // Line comment
    if (c === '/' && src[i + 1] === '/') {
      const end = src.indexOf('\n', i)
      const stop = end === -1 ? n : end
      out += src.slice(i, stop)
      i = stop
      continue
    }
    // Block comment
    if (c === '/' && src[i + 1] === '*') {
      const end = src.indexOf('*/', i + 2)
      const stop = end === -1 ? n : end + 2
      out += src.slice(i, stop)
      i = stop
      continue
    }
    // String literal
    if (c === '"') {
      out += c
      i++
      while (i < n) {
        const sc = src[i]
        out += sc
        i++
        if (sc === '\\' && i < n) { out += src[i]; i++; continue }
        if (sc === '"') break
      }
      continue
    }
    // Standalone $t token (followed by a non-word char or end)
    if (c === '$' && src[i + 1] === 't' && !/[A-Za-z0-9_]/.test(src[i + 2] || '')) {
      out += value
      i += 2
      continue
    }
    out += c
    i++
  }
  return out
}

function doRender() {
  if (!renderer) return
  error.value = ''
  errorLine.value = -1
  errorCharPos.value = -1
  try {
    const t0 = performance.now()
    // Inject $t animation variable
    let codeWithT = replaceAnimT(code.value, String(animT.value))
    // Fast preview: halve all $fn values
    if (fastPreviewMode.value) {
      codeWithT = codeWithT.replace(/\$fn\s*=\s*(\d+)/g, (_m: string, n: string) => {
        return '$fn=' + Math.max(6, Math.floor(parseInt(n) / 2))
      })
    }
    const result = parseOpenSCADWithAST(codeWithT, (name: string) => {
      const baseName = name.replace(/\.scad$/, '')
      const tab = tabs.value.find(tb => {
        const tabBase = tb.name.replace(/\.scad$/, '')
        return tabBase === baseName || tabBase === name || tb.name === name
      })
      return tab ? tab.code : null
    })
    const meshes = result.meshes
    astNodes.value = result.ast
    // Collect 3D annotations from parse result
    parsedAnnotations.value = result.annotations || []
    // Collect profiling data
    if (result.profileEntries) {
      profileEntries.value = result.profileEntries
    }
    const tParsed = performance.now()
    perfParseTime.value = Math.round(tParsed - t0)
    meshCount.value = meshes.length
    triCount.value = meshes.reduce((s: number, m: MeshData) => s + m.indices.length / 3, 0)
    lastParsedMeshes = meshes
    const tMeshStart = performance.now()
    renderer.setMeshes(meshes)
    const tMeshEnd = performance.now()
    // Mesh generation (post-parse CPU work preparing meshes, before GPU upload).
    perfMeshGenTime.value = Math.round(tMeshStart - tParsed)
    perfGpuUploadTime.value = Math.round(tMeshEnd - tMeshStart)
    const t1 = performance.now()
    renderTime.value = Math.round(t1 - t0)
    // Adaptive quality warnings (warn once per crossing into the heavy range)
    if (triCount.value > 100000) {
      if (!heavyWarned.value) {
        addToast(t('perfWarning').replace('{n}', String(triCount.value)), 'info', { duration: 5000 })
        heavyWarned.value = true
      }
    } else {
      heavyWarned.value = false
    }
    if (renderTime.value > 500 && !fastPreviewMode.value) {
      addToast(t('slowRenderHint').replace('{ms}', String(renderTime.value)), 'info', {
        actionLabel: t('fastPreview'),
        action: () => { toggleFastPreview() },
        duration: 6000,
      })
    }
    // Console log entries
    const nodeCount = countASTNodes(result.ast)
    addConsoleEntry('info', t('parsedNodes').replace('{n}', String(nodeCount)))
    addConsoleEntry('info', t('generatedMeshes')
      .replace('{m}', String(meshes.length))
      .replace('{t}', String(triCount.value))
      .replace('{ms}', String(renderTime.value)))
    // Log echo messages
    for (const echoMsg of result.echos) {
      addConsoleEntry('info', echoMsg)
    }
    // Show partial errors from error recovery
    for (const errMsg of result.errors) {
      let msg = errMsg
      const posMatch = msg.match(/@(\d+)/)
      if (posMatch) {
        const pos = parseInt(posMatch[1])
        const prefix = code.value.substring(0, pos)
        const line = prefix.split('\n').length
        errorLine.value = line
        errorCharPos.value = pos
        msg = `${t('errorAtLine')} ${line}: ${msg}`
      }
      error.value = msg
      addConsoleEntry('error', msg)
    }
    // Update breadcrumbs after parse
    updateBreadcrumbs()
    // Update ghost meshes if ghost mode is active
    if (ghostMode.value) {
      nextTick(() => updateGhostMeshes())
    }
  } catch (e: any) {
    let msg = e.message || String(e)
    const posMatch = msg.match(/@(\d+)/)
    if (posMatch) {
      const pos = parseInt(posMatch[1])
      const prefix = code.value.substring(0, pos)
      const line = prefix.split('\n').length
      errorLine.value = line
      errorCharPos.value = pos
      msg = `${t('errorAtLine')} ${line}: ${msg}`
    }
    error.value = msg
    addConsoleEntry('error', msg)
  }
}

function toggleFastPreview() {
  fastPreviewMode.value = !fastPreviewMode.value
  addToast(fastPreviewMode.value ? t('fastPreviewOn') : t('fastPreviewOff'), 'info')
  doRender()
}

function countASTNodes(nodes: ASTNode[]): number {
  let count = 0
  for (const n of nodes) {
    count++
    count += countASTNodes(n.children)
  }
  return count
}

function loadExample(name: string) {
  if (EXAMPLES[name]) {
    pushUndoSnapshot() // snapshot before example load
    code.value = EXAMPLES[name]
    activeTab.value.savedCode = EXAMPLES[name]
    addToRecent(name, EXAMPLES[name])
  }
}

/* ── Editor text-edit helpers (preserve native undo where possible) ──
 * insertEditorText replaces the current selection (or inserts at the caret)
 * with `text`, then places the caret `selOffset` chars from the insertion
 * start (defaults to end of inserted text). It prefers
 * document.execCommand('insertText') because that keeps the textarea's native
 * undo stack working; if that fails it falls back to a manual value splice. */
function insertEditorText(el: HTMLTextAreaElement, text: string, selOffset?: number) {
  const start = el.selectionStart
  let ok = false
  try {
    ok = document.execCommand('insertText', false, text)
  } catch {
    ok = false
  }
  if (!ok) {
    // Manual splice fallback (loses native undo for this edit only).
    const end = el.selectionEnd
    el.value = el.value.substring(0, start) + text + el.value.substring(end)
  }
  // Keep the Vue model in sync with the DOM value.
  code.value = el.value
  const caret = start + (selOffset !== undefined ? selOffset : text.length)
  nextTick(() => { el.selectionStart = el.selectionEnd = caret })
}

// Wrap the current selection with open/close, leaving the selection intact.
function wrapEditorSelection(el: HTMLTextAreaElement, open: string, close: string) {
  const start = el.selectionStart
  const end = el.selectionEnd
  const selected = el.value.substring(start, end)
  let ok = false
  try {
    ok = document.execCommand('insertText', false, open + selected + close)
  } catch {
    ok = false
  }
  if (!ok) {
    el.value = el.value.substring(0, start) + open + selected + close + el.value.substring(end)
  }
  code.value = el.value
  // Re-select the original content (now offset by one for the opening char).
  nextTick(() => {
    el.selectionStart = start + open.length
    el.selectionEnd = start + open.length + selected.length
  })
}

const AUTO_CLOSE_PAIRS: Record<string, string> = { '(': ')', '[': ']', '{': '}' }
const AUTO_CLOSE_CLOSERS = new Set([')', ']', '}'])

/* Feature 1: auto-close brackets & quotes. Returns true if the key was handled. */
function handleAutoClose(e: KeyboardEvent, el: HTMLTextAreaElement): boolean {
  const start = el.selectionStart
  const end = el.selectionEnd
  const hasSel = start !== end
  const src = el.value
  const key = e.key

  // Opening bracket: insert matching pair (or wrap selection).
  if (AUTO_CLOSE_PAIRS[key]) {
    e.preventDefault()
    if (hasSel) {
      wrapEditorSelection(el, key, AUTO_CLOSE_PAIRS[key])
    } else {
      insertEditorText(el, key + AUTO_CLOSE_PAIRS[key], 1)
    }
    return true
  }

  // Quote: wrap selection, or insert a matching pair, or skip over an existing one.
  if (key === '"') {
    e.preventDefault()
    if (hasSel) {
      wrapEditorSelection(el, '"', '"')
    } else if (src[start] === '"') {
      // Cursor sits just before a closing quote → step over it.
      nextTick(() => { el.selectionStart = el.selectionEnd = start + 1 })
    } else {
      insertEditorText(el, '""', 1)
    }
    return true
  }

  // Closing bracket: if the same closer is already next, just step over it.
  if (AUTO_CLOSE_CLOSERS.has(key) && !hasSel && src[start] === key) {
    e.preventDefault()
    nextTick(() => { el.selectionStart = el.selectionEnd = start + 1 })
    return true
  }

  // Backspace: delete a matched empty pair (e.g. cursor between "()" or "\"\"").
  if (key === 'Backspace' && !hasSel && start > 0) {
    const prev = src[start - 1]
    const next = src[start]
    const isEmptyPair =
      (AUTO_CLOSE_PAIRS[prev] && AUTO_CLOSE_PAIRS[prev] === next) ||
      (prev === '"' && next === '"')
    if (isEmptyPair) {
      e.preventDefault()
      // Select both chars then delete to keep native undo coherent.
      el.selectionStart = start - 1
      el.selectionEnd = start + 1
      insertEditorText(el, '', 0)
      return true
    }
  }

  return false
}

/* Feature 2: auto-indent on Enter (and brace expansion). */
function handleAutoIndent(e: KeyboardEvent, el: HTMLTextAreaElement): boolean {
  if (e.key !== 'Enter' || e.shiftKey || e.ctrlKey || e.metaKey || e.altKey) return false
  const start = el.selectionStart
  const end = el.selectionEnd
  const src = el.value
  // Leading whitespace of the current line.
  const lineStart = src.lastIndexOf('\n', start - 1) + 1
  const curLine = src.substring(lineStart, start)
  const indentMatch = curLine.match(/^[ \t]*/)
  const baseIndent = indentMatch ? indentMatch[0] : ''
  const unit = ' '.repeat(prefTabSize.value)
  const prevChar = start > 0 ? src[start - 1] : ''
  const nextChar = src[end] ?? ''

  if (prevChar === '{' && nextChar === '}') {
    // Expand braces: blank indented line in the middle, "}" back to base indent.
    e.preventDefault()
    insertEditorText(el, '\n' + baseIndent + unit + '\n' + baseIndent,
      1 + baseIndent.length + unit.length)
    return true
  }
  if (prevChar === '{') {
    // Open brace with nothing after → add one indent level.
    e.preventDefault()
    insertEditorText(el, '\n' + baseIndent + unit)
    return true
  }
  if (baseIndent.length > 0) {
    // Plain newline that preserves the current indentation.
    e.preventDefault()
    insertEditorText(el, '\n' + baseIndent)
    return true
  }
  return false
}

/* Feature 3: smart Home key — toggle between first non-whitespace and column 0. */
function handleSmartHome(e: KeyboardEvent, el: HTMLTextAreaElement): boolean {
  if (e.key !== 'Home' || e.ctrlKey || e.metaKey || e.altKey) return false
  e.preventDefault()
  const pos = el.selectionStart
  const src = el.value
  const lineStart = src.lastIndexOf('\n', pos - 1) + 1
  let firstNonWs = lineStart
  while (firstNonWs < src.length && (src[firstNonWs] === ' ' || src[firstNonWs] === '\t')) {
    firstNonWs++
  }
  // If a newline starts the line (empty line) keep firstNonWs at lineStart.
  if (src[lineStart] === '\n') firstNonWs = lineStart
  const target = pos === firstNonWs ? lineStart : firstNonWs
  if (e.shiftKey) {
    // Extend selection from the existing anchor (selectionEnd is the moving edge
    // for forward selections; use a direction-aware anchor).
    const anchor = el.selectionStart === pos ? el.selectionEnd : el.selectionStart
    el.selectionStart = Math.min(anchor, target)
    el.selectionEnd = Math.max(anchor, target)
  } else {
    el.selectionStart = el.selectionEnd = target
  }
  return true
}

/* ── Feature: Duplicate Line (Ctrl+D) ── */
function duplicateLine(el: HTMLTextAreaElement) {
  const src = el.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd

  if (selStart === selEnd) {
    // No selection: duplicate the current line
    const lineStart = src.lastIndexOf('\n', selStart - 1) + 1
    let lineEnd = src.indexOf('\n', selStart)
    if (lineEnd === -1) lineEnd = src.length
    const lineText = src.substring(lineStart, lineEnd)
    // Insert a newline + copy of the line after the current line end
    el.selectionStart = el.selectionEnd = lineEnd
    insertEditorText(el, '\n' + lineText)
  } else {
    // With selection: duplicate selected text right after the selection
    const selected = src.substring(selStart, selEnd)
    el.selectionStart = el.selectionEnd = selEnd
    insertEditorText(el, selected)
  }
}

/* ── Feature: Move Line Up/Down (Alt+Up / Alt+Down) ── */
function moveLineUp(el: HTMLTextAreaElement) {
  const src = el.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd

  // Find the range of selected lines
  const firstLineStart = src.lastIndexOf('\n', selStart - 1) + 1
  let lastLineEnd = src.indexOf('\n', selEnd - (selEnd > selStart && src[selEnd - 1] === '\n' ? 1 : 0))
  if (lastLineEnd === -1) lastLineEnd = src.length

  // Can't move up if already at the first line
  if (firstLineStart === 0) return

  // The line above
  const prevLineStart = src.lastIndexOf('\n', firstLineStart - 2) + 1
  const prevLine = src.substring(prevLineStart, firstLineStart - 1)
  const selectedBlock = src.substring(firstLineStart, lastLineEnd)

  // Build new content: selectedBlock \n prevLine
  const newContent = selectedBlock + '\n' + prevLine
  el.value = src.substring(0, prevLineStart) + newContent + src.substring(lastLineEnd)
  code.value = el.value

  // Restore cursor position: shift up by (prevLine.length + 1)
  const shift = prevLine.length + 1
  nextTick(() => {
    el.selectionStart = selStart - shift
    el.selectionEnd = selEnd - shift
  })
}

function moveLineDown(el: HTMLTextAreaElement) {
  const src = el.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd

  // Find the range of selected lines
  const firstLineStart = src.lastIndexOf('\n', selStart - 1) + 1
  let lastLineEnd = src.indexOf('\n', selEnd - (selEnd > selStart && src[selEnd - 1] === '\n' ? 1 : 0))
  if (lastLineEnd === -1) lastLineEnd = src.length

  // Can't move down if already at the last line
  if (lastLineEnd >= src.length) return

  // The line below
  let nextLineEnd = src.indexOf('\n', lastLineEnd + 1)
  if (nextLineEnd === -1) nextLineEnd = src.length
  const nextLine = src.substring(lastLineEnd + 1, nextLineEnd)
  const selectedBlock = src.substring(firstLineStart, lastLineEnd)

  // Build new content: nextLine \n selectedBlock
  const newContent = nextLine + '\n' + selectedBlock
  el.value = src.substring(0, firstLineStart) + newContent + src.substring(nextLineEnd)
  code.value = el.value

  // Restore cursor position: shift down by (nextLine.length + 1)
  const shift = nextLine.length + 1
  nextTick(() => {
    el.selectionStart = selStart + shift
    el.selectionEnd = selEnd + shift
  })
}

function handleKey(e: KeyboardEvent) {
  // Custom Undo: Ctrl+Z
  if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
    e.preventDefault()
    customUndo()
    return
  }
  // Custom Redo: Ctrl+Shift+Z or Ctrl+Y
  if ((e.ctrlKey || e.metaKey) && ((e.key === 'z' && e.shiftKey) || e.key === 'y')) {
    e.preventDefault()
    customRedo()
    return
  }

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

  // Comment toggle: Ctrl+/
  if ((e.ctrlKey || e.metaKey) && e.key === '/') {
    e.preventDefault()
    toggleComment()
    return
  }

  // Join Lines: Ctrl+J
  if ((e.ctrlKey || e.metaKey) && e.key === 'j' && !e.shiftKey && !e.altKey) {
    e.preventDefault()
    joinLinesCmd()
    return
  }

  // Toggle UPPERCASE: Ctrl+Shift+U
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'U') {
    e.preventDefault()
    toggleCase('upper')
    return
  }

  // Toggle lowercase: Ctrl+Shift+L
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'L') {
    e.preventDefault()
    toggleCase('lower')
    return
  }

  const editorEl = e.target as HTMLTextAreaElement

  // Multi-replace / Select all occurrences: Ctrl+D
  if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
    e.preventDefault()
    activateMultiReplace(editorEl)
    return
  }

  // Duplicate line: Ctrl+Shift+D (moved from Ctrl+D)
  if ((e.ctrlKey || e.metaKey) && e.key === 'D' && e.shiftKey) {
    e.preventDefault()
    duplicateLine(editorEl)
    return
  }

  // Move line up/down: Alt+Up / Alt+Down
  if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      moveLineUp(editorEl)
      return
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      moveLineDown(editorEl)
      return
    }
  }

  // Feature 3: Smart Home key (must run before any indentation handlers).
  if (handleSmartHome(e, editorEl)) return

  // Feature 1: Auto-close brackets/quotes & smart backspace.
  // Skip when a modifier (besides Shift) is held so shortcuts keep working.
  if (!e.ctrlKey && !e.metaKey && !e.altKey) {
    if (handleAutoClose(e, editorEl)) return
  }

  // Feature 2: Auto-indent on Enter.
  if (handleAutoIndent(e, editorEl)) return

  if (e.key === 'Tab' && !acVisible.value) {
    e.preventDefault()
    const el = e.target as HTMLTextAreaElement
    const s = el.selectionStart, end = el.selectionEnd
    const indent = ' '.repeat(prefTabSize.value)
    code.value = code.value.substring(0, s) + indent + code.value.substring(end)
    requestAnimationFrame(() => { el.selectionStart = el.selectionEnd = s + prefTabSize.value })
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); doRender() }
}

function toggleComment() {
  const el = textareaRef.value
  if (!el) return
  const src = code.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd

  // Find the lines covered by selection
  const lineStartIdx = src.lastIndexOf('\n', selStart - 1) + 1
  let lineEndIdx = src.indexOf('\n', selEnd)
  if (lineEndIdx === -1) lineEndIdx = src.length

  const block = src.substring(lineStartIdx, lineEndIdx)
  const lines = block.split('\n')

  // Determine if we should comment or uncomment
  // If all non-empty lines start with //, uncomment; otherwise comment
  const nonEmpty = lines.filter(l => l.trim().length > 0)
  const allCommented = nonEmpty.length > 0 && nonEmpty.every(l => l.trimStart().startsWith('//'))

  let newLines: string[]
  let cursorDelta = 0

  if (allCommented) {
    // Uncomment: remove first occurrence of '// ' or '//'
    newLines = lines.map(l => {
      if (l.trimStart().startsWith('// ')) {
        const idx = l.indexOf('// ')
        return l.substring(0, idx) + l.substring(idx + 3)
      } else if (l.trimStart().startsWith('//')) {
        const idx = l.indexOf('//')
        return l.substring(0, idx) + l.substring(idx + 2)
      }
      return l
    })
    // Cursor delta for first line
    const firstLine = lines[0]
    if (firstLine.trimStart().startsWith('// ')) cursorDelta = -3
    else if (firstLine.trimStart().startsWith('//')) cursorDelta = -2
  } else {
    // Comment: add '// ' at the beginning of each line
    newLines = lines.map(l => {
      if (l.trim().length === 0) return l // keep empty lines as-is
      return '// ' + l
    })
    cursorDelta = lines[0].trim().length === 0 ? 0 : 3
  }

  const newBlock = newLines.join('\n')
  code.value = src.substring(0, lineStartIdx) + newBlock + src.substring(lineEndIdx)

  // Restore cursor / selection
  nextTick(() => {
    const newSelStart = Math.max(lineStartIdx, selStart + cursorDelta)
    const lengthDiff = newBlock.length - block.length
    const newSelEnd = selEnd + lengthDiff
    el.selectionStart = newSelStart
    el.selectionEnd = Math.max(newSelStart, newSelEnd)
    el.focus()
  })
}

/* ── WASD Camera Controls ── */
function handleCanvasKeydown(e: KeyboardEvent) {
  if (!renderer) return
  const key = e.key.toLowerCase()
  const shift = e.shiftKey
  const step = 0.05
  const distStep = renderer.dist * 0.05
  const panStep = renderer.dist * 0.02

  if (flyCameraMode.value) {
    // Fly camera: WASD moves camera position in look direction, Q/E for altitude
    const flySpeed = renderer.dist * 0.04
    const cy = Math.cos(renderer.yaw), sy = Math.sin(renderer.yaw)
    const cp = Math.cos(renderer.pitch), sp = Math.sin(renderer.pitch)
    if (key === 'w' || key === 'ц') {
      e.preventDefault()
      renderer.tx += sy * cp * flySpeed
      renderer.ty += sp * flySpeed
      renderer.tz += cy * cp * flySpeed
    } else if (key === 's' || key === 'ы') {
      e.preventDefault()
      renderer.tx -= sy * cp * flySpeed
      renderer.ty -= sp * flySpeed
      renderer.tz -= cy * cp * flySpeed
    } else if (key === 'a' || key === 'ф') {
      e.preventDefault()
      renderer.tx += cy * flySpeed
      renderer.tz -= sy * flySpeed
    } else if (key === 'd' || key === 'в') {
      e.preventDefault()
      renderer.tx -= cy * flySpeed
      renderer.tz += sy * flySpeed
    } else if (key === 'q' || key === 'й') {
      e.preventDefault()
      renderer.ty += flySpeed
    } else if (key === 'e' || key === 'у') {
      e.preventDefault()
      renderer.ty -= flySpeed
    }
  } else if (key === 'w' || key === 'ц') {
    e.preventDefault()
    if (shift) {
      // Pan forward (into screen along view direction)
      const cy = Math.cos(renderer.yaw), sy = Math.sin(renderer.yaw)
      renderer.tx += sy * panStep
      renderer.tz += cy * panStep
    } else {
      renderer.dist = Math.max(1, renderer.dist - distStep)
    }
  } else if (key === 's' || key === 'ы') {
    e.preventDefault()
    if (shift) {
      const cy = Math.cos(renderer.yaw), sy = Math.sin(renderer.yaw)
      renderer.tx -= sy * panStep
      renderer.tz -= cy * panStep
    } else {
      renderer.dist = Math.min(50000, renderer.dist + distStep)
    }
  } else if (key === 'a' || key === 'ф') {
    e.preventDefault()
    if (shift) {
      const cy = Math.cos(renderer.yaw), sy = Math.sin(renderer.yaw)
      renderer.tx += cy * panStep
      renderer.tz -= sy * panStep
    } else {
      renderer.yaw += step
    }
  } else if (key === 'd' || key === 'в') {
    e.preventDefault()
    if (shift) {
      const cy = Math.cos(renderer.yaw), sy = Math.sin(renderer.yaw)
      renderer.tx -= cy * panStep
      renderer.tz += sy * panStep
    } else {
      renderer.yaw -= step
    }
  } else if (key === 'q' || key === 'й') {
    e.preventDefault()
    renderer.pitch = Math.min(1.5, renderer.pitch + step)
  } else if (key === 'e' || key === 'у') {
    e.preventDefault()
    renderer.pitch = Math.max(-1.5, renderer.pitch - step)
  } else if (key === '1' && e.location === 3) {
    e.preventDefault(); setView('front'); return
  } else if (key === '3' && e.location === 3) {
    e.preventDefault(); setView('right'); return
  } else if (key === '7' && e.location === 3) {
    e.preventDefault(); setView('top'); return
  } else if (key === '5' && e.location === 3) {
    e.preventDefault(); toggleProjection(); return
  } else if (key === '0' && e.location === 3) {
    e.preventDefault(); setView('reset'); return
  } else if (key === '9' && e.location === 3) {
    e.preventDefault(); renderer.animateTo(Math.PI, 0); return
  } else if (key === '4' && e.location === 3) {
    e.preventDefault(); renderer.animateTo(-Math.PI / 2, 0); return
  } else if (key === '6' && e.location === 3) {
    e.preventDefault(); renderer.animateTo(Math.PI / 2, 0); return
  } else if (key === '8' && e.location === 3) {
    e.preventDefault(); renderer.animateTo(0, -Math.PI / 2); return
  } else if (key === '2' && e.location === 3) {
    e.preventDefault(); renderer.animateTo(Math.PI, 0); return
  } else if (key === '.' && e.location === 3) {
    e.preventDefault(); renderer.autoFitAll(); return
  } else if (key === 'f' || key === 'а') {
    e.preventDefault(); renderer.autoFitAll(); return
  } else if (key === 'home') {
    e.preventDefault(); setView('reset'); return
  } else {
    return // No camera key was pressed, skip requestRender
  }
  renderer.requestRender()
}

function handleKeyUp() {
  updateAutocomplete()
  updateBracketMatch()
  updateBreadcrumbs()
}

function handleClick() {
  updateBracketMatch()
  dismissAutocomplete()
  updateBreadcrumbs()
}

/* ── Feature: Word Wrap Toggle ── */
const wordWrap = ref(localStorage.getItem('scad-word-wrap') === 'true')
watch(wordWrap, v => localStorage.setItem('scad-word-wrap', String(v)))

function toggleWordWrap() {
  wordWrap.value = !wordWrap.value
}

/* ── Print Shortcuts ── */
function printShortcuts() {
  const el = document.querySelector('.shortcuts-scroll')
  if (!el) return
  const win = window.open('', '_blank', 'width=600,height=800')
  if (!win) return
  win.document.write(`<!DOCTYPE html><html><head><title>${escapeHtml(t('shortcutsTitle'))}</title><style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 24px; color: #222; }
    h1 { font-size: 1.3rem; margin-bottom: 16px; }
    .shortcut-category { font-weight: 700; font-size: 0.9rem; margin-top: 16px; margin-bottom: 6px; color: #555; text-transform: uppercase; letter-spacing: 0.04em; }
    .shortcut-row { display: flex; justify-content: space-between; padding: 3px 0; font-size: 0.85rem; border-bottom: 1px solid #eee; }
    kbd { background: #f0f0f0; border: 1px solid #ccc; border-radius: 3px; padding: 1px 6px; font-size: 0.78rem; font-family: monospace; }
    @media print { body { padding: 0; } }
  </style></head><body><h1>${escapeHtml(t('shortcutsTitle'))}</h1>${el.innerHTML}</body></html>`)
  win.document.close()
  win.focus()
  setTimeout(() => { win.print() }, 300)
}

/* ── Feature: Selection Info ── */
const selectionInfo = ref('')

function updateSelectionInfo() {
  const el = textareaRef.value
  if (!el) { selectionInfo.value = ''; return }
  const s = el.selectionStart
  const e = el.selectionEnd
  if (s === e) { selectionInfo.value = ''; return }
  const selected = code.value.substring(s, e)
  const chars = selected.length
  const lines = selected.split('\n').length
  selectionInfo.value = t('selChars').replace('{x}', String(chars)).replace('{y}', String(lines))
}

/* ── Feature: Editor Breadcrumbs ── */
interface BreadcrumbItem {
  label: string
  pos: number
  endPos: number
}
const breadcrumbs = ref<BreadcrumbItem[]>([])

function updateBreadcrumbs() {
  const el = textareaRef.value
  if (!el) { breadcrumbs.value = []; return }
  const cursorPos = el.selectionStart
  const ast = astNodes.value
  if (!ast || ast.length === 0) { breadcrumbs.value = []; return }
  const path: BreadcrumbItem[] = []
  findBreadcrumbPath(ast, cursorPos, path)
  breadcrumbs.value = path
}

function findBreadcrumbPath(nodes: ASTNode[], cursorPos: number, path: BreadcrumbItem[]): boolean {
  for (const node of nodes) {
    if (cursorPos >= node.pos && cursorPos <= node.endPos) {
      // Skip internal-only node types
      if (node.name === '__assign' || node.name === 'function') {
        continue
      }
      let label = node.name
      if (node.name === 'module') {
        label = 'module ' + (node.args.__name || '')
      } else {
        // Add key args hint
        const hints: string[] = []
        for (const [k, v] of Object.entries(node.args)) {
          if (k.startsWith('_')) continue
          if (typeof v === 'number') hints.push(`${k}=${v}`)
          if (hints.length >= 2) break
        }
        if (hints.length) label += '(' + hints.join(', ') + ')'
        else label += '()'
      }
      path.push({ label, pos: node.pos, endPos: node.endPos })
      if (node.children.length > 0) {
        findBreadcrumbPath(node.children, cursorPos, path)
      }
      return true
    }
  }
  return false
}

function onBreadcrumbClick(item: BreadcrumbItem) {
  const el = textareaRef.value
  if (!el) return
  el.focus()
  el.selectionStart = item.pos
  el.selectionEnd = item.endPos
  // Scroll the textarea so the selection is visible
  const textBefore = code.value.substring(0, item.pos)
  const lineNum = textBefore.split('\n').length
  const lineHeight = prefFontSize.value * 1.5
  el.scrollTop = Math.max(0, (lineNum - 3) * lineHeight)
}

/* ── Feature: Occurrence Highlighting ── */
const occurrenceWord = ref('')
const occurrencePositions = ref<{ start: number; end: number }[]>([])

function updateOccurrenceHighlight() {
  const el = textareaRef.value
  if (!el) { occurrenceWord.value = ''; occurrencePositions.value = []; return }
  const s = el.selectionStart
  const e = el.selectionEnd
  const src = code.value

  // Only highlight when exactly a word is selected (or double-click selects a word)
  if (s === e) {
    occurrenceWord.value = ''
    occurrencePositions.value = []
    return
  }

  const selected = src.substring(s, e)
  // Must be a single "word" (alphanumeric + underscores, no spaces)
  if (!/^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(selected)) {
    occurrenceWord.value = ''
    occurrencePositions.value = []
    return
  }

  // Check word boundaries
  if (s > 0 && /[a-zA-Z0-9_$]/.test(src[s - 1])) {
    occurrenceWord.value = ''; occurrencePositions.value = []; return
  }
  if (e < src.length && /[a-zA-Z0-9_$]/.test(src[e])) {
    occurrenceWord.value = ''; occurrencePositions.value = []; return
  }

  occurrenceWord.value = selected

  // Find all occurrences with word boundary
  const positions: { start: number; end: number }[] = []
  const wordLen = selected.length
  let idx = 0
  while (idx <= src.length - wordLen) {
    const pos = src.indexOf(selected, idx)
    if (pos === -1) break
    // Check word boundaries
    const before = pos > 0 ? src[pos - 1] : ' '
    const after = pos + wordLen < src.length ? src[pos + wordLen] : ' '
    if (!/[a-zA-Z0-9_$]/.test(before) && !/[a-zA-Z0-9_$]/.test(after)) {
      // Skip the occurrence that is currently selected
      if (pos !== s) {
        positions.push({ start: pos, end: pos + wordLen })
      }
    }
    idx = pos + 1
  }
  occurrencePositions.value = positions
}

function onSelectionChange() {
  updateSelectionInfo()
  updateOccurrenceHighlight()
}

onMounted(() => {
  document.addEventListener('selectionchange', onSelectionChange)
})
onUnmounted(() => {
  document.removeEventListener('selectionchange', onSelectionChange)
})

/* ── Feature: Multi-Replace Mode (Ctrl+D) ── */
const multiReplaceActive = ref(false)
const multiReplaceWord = ref('')
const multiReplaceInput = ref('')
const multiReplaceCount = ref(0)

function activateMultiReplace(el: HTMLTextAreaElement) {
  const src = code.value
  const s = el.selectionStart
  const e = el.selectionEnd

  // If no selection, select the word under cursor first
  if (s === e) {
    const wordChars = /[a-zA-Z0-9_$]/
    let wStart = s, wEnd = s
    while (wStart > 0 && wordChars.test(src[wStart - 1])) wStart--
    while (wEnd < src.length && wordChars.test(src[wEnd])) wEnd++
    if (wStart === wEnd) return
    el.setSelectionRange(wStart, wEnd)
    updateOccurrenceHighlight()
    return
  }

  const selected = src.substring(s, e)
  if (!/^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(selected)) return

  // Count all occurrences
  let count = 0
  let idx = 0
  while (idx <= src.length - selected.length) {
    const pos = src.indexOf(selected, idx)
    if (pos === -1) break
    const before = pos > 0 ? src[pos - 1] : ' '
    const after = pos + selected.length < src.length ? src[pos + selected.length] : ' '
    if (!/[a-zA-Z0-9_$]/.test(before) && !/[a-zA-Z0-9_$]/.test(after)) {
      count++
    }
    idx = pos + 1
  }
  if (count < 2) return

  multiReplaceActive.value = true
  multiReplaceWord.value = selected
  multiReplaceInput.value = selected
  multiReplaceCount.value = count
}

function executeMultiReplace() {
  if (!multiReplaceWord.value) return
  const src = code.value
  const oldWord = multiReplaceWord.value
  const newWord = multiReplaceInput.value
  if (oldWord === newWord) { cancelMultiReplace(); return }

  // Replace all occurrences with word boundary check
  let result = ''
  let lastIdx = 0
  let idx = 0
  while (idx <= src.length - oldWord.length) {
    const pos = src.indexOf(oldWord, idx)
    if (pos === -1) break
    const before = pos > 0 ? src[pos - 1] : ' '
    const after = pos + oldWord.length < src.length ? src[pos + oldWord.length] : ' '
    if (!/[a-zA-Z0-9_$]/.test(before) && !/[a-zA-Z0-9_$]/.test(after)) {
      result += src.substring(lastIdx, pos) + newWord
      lastIdx = pos + oldWord.length
      idx = pos + oldWord.length
    } else {
      idx = pos + 1
    }
  }
  result += src.substring(lastIdx)
  code.value = result
  multiReplaceActive.value = false
  multiReplaceWord.value = ''
  addToast(t('multiReplaceActive').replace('{n}', String(multiReplaceCount.value)), 'success')
}

function cancelMultiReplace() {
  multiReplaceActive.value = false
  multiReplaceWord.value = ''
}

/* ── Feature: Duplicate Tab ── */
function duplicateTab(id: string) {
  const source = tabs.value.find(tb => tb.id === id)
  if (!source) return
  const newName = t('copyOf').replace('{name}', source.name)
  const newTab: EditorTab = {
    id: generateTabId(),
    name: newName,
    code: source.code,
    savedCode: source.code,
    colorTag: source.colorTag,
  }
  tabs.value.push(newTab)
  activeTabId.value = newTab.id
  saveTabs()
  closeTabContextMenu()
}

/* ── Feature: Tab Color Tags ── */
const TAB_COLORS: { key: string; color: string }[] = [
  { key: 'tabColorNone', color: '' },
  { key: 'tabColorRed', color: '#e53935' },
  { key: 'tabColorGreen', color: '#43a047' },
  { key: 'tabColorBlue', color: '#1e88e5' },
  { key: 'tabColorYellow', color: '#fdd835' },
  { key: 'tabColorPurple', color: '#8e24aa' },
]
const showTabColorMenu = ref(false)

function setTabColor(id: string, color: string) {
  const tab = tabs.value.find(tb => tb.id === id)
  if (tab) {
    tab.colorTag = color || undefined
    saveTabs()
  }
  showTabColorMenu.value = false
  closeTabContextMenu()
}

/* ── Feature: Quick Switcher (Ctrl+P) ── */
const showQuickSwitcher = ref(false)
const quickSwitcherSearch = ref('')
const quickSwitcherIndex = ref(0)
const quickSwitcherInputRef = ref<HTMLInputElement | null>(null)

const filteredTabs = computed(() => {
  const q = quickSwitcherSearch.value.trim().toLowerCase()
  const all = sortedTabs.value
  if (!q) return all
  return all.filter(tb => tb.name.toLowerCase().includes(q))
})

function openQuickSwitcher() {
  showQuickSwitcher.value = true
  quickSwitcherSearch.value = ''
  quickSwitcherIndex.value = 0
  nextTick(() => quickSwitcherInputRef.value?.focus())
}

function closeQuickSwitcher() {
  showQuickSwitcher.value = false
  quickSwitcherSearch.value = ''
}

function executeQuickSwitch() {
  const tabs_f = filteredTabs.value
  if (tabs_f.length > 0) {
    switchTab(tabs_f[quickSwitcherIndex.value].id)
  }
  closeQuickSwitcher()
}

function handleQuickSwitcherKeydown(e: KeyboardEvent) {
  const items = filteredTabs.value
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    quickSwitcherIndex.value = (quickSwitcherIndex.value + 1) % items.length
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    quickSwitcherIndex.value = (quickSwitcherIndex.value - 1 + items.length) % items.length
  } else if (e.key === 'Enter') {
    e.preventDefault()
    executeQuickSwitch()
  } else if (e.key === 'Escape') {
    e.preventDefault()
    closeQuickSwitcher()
  }
}

watch(quickSwitcherSearch, () => {
  quickSwitcherIndex.value = 0
})

/* ── Feature: Notification Center ── */
interface NotificationEntry {
  id: number
  message: string
  type: 'success' | 'info' | 'error'
  timestamp: Date
  read: boolean
}

const MAX_NOTIFICATIONS = 50
const notifications = ref<NotificationEntry[]>([])
const showNotificationCenter = ref(false)
let notifIdCounter = 0

const unreadNotificationCount = computed(() => notifications.value.filter(n => !n.read).length)

function addNotification(message: string, type: 'success' | 'info' | 'error') {
  notifications.value.unshift({
    id: notifIdCounter++,
    message,
    type,
    timestamp: new Date(),
    read: false,
  })
  if (notifications.value.length > MAX_NOTIFICATIONS) {
    notifications.value = notifications.value.slice(0, MAX_NOTIFICATIONS)
  }
}

function toggleNotificationCenter() {
  showNotificationCenter.value = !showNotificationCenter.value
  if (showNotificationCenter.value) {
    // Mark all as read
    for (const n of notifications.value) n.read = true
  }
}

function clearNotifications() {
  notifications.value = []
}

function formatNotifTime(d: Date): string {
  return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

/* ── Feature: Embed Mode ── */
const embedMode = ref(false)

/* ── Feature: Spec Sheet ── */
function generateSpecSheet() {
  if (!renderer || !lastParsedMeshes.length) return
  computeStatistics()
  const modelName = activeTab.value.name || t('untitled')
  const dims = boundsSize.value
  const vol = statsVolume.value
  const area = statsSurfaceArea.value
  const tris = triCount.value
  const density = 1.24  // PLA default g/cm3
  const volumeCm3 = vol / 1000  // mm3 to cm3
  const weight = volumeCm3 * density
  const codeText = code.value
  const now = new Date().toLocaleString()

  // Capture screenshot as data URL
  let screenshotDataUrl = ''
  try {
    const canvas = canvasRef.value
    if (canvas) {
      screenshotDataUrl = canvas.toDataURL('image/png')
    }
  } catch { /* cross-origin, skip */ }

  const win = window.open('', '_blank', 'width=800,height=900')
  if (!win) return
  win.document.write(`<!DOCTYPE html><html><head><title>${escapeHtml(t('specSheet'))} - ${escapeHtml(modelName)}</title>
<style>
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 32px; color: #222; max-width: 800px; margin: 0 auto; }
  h1 { font-size: 1.5rem; border-bottom: 2px solid #333; padding-bottom: 8px; margin-bottom: 16px; }
  .spec-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px 24px; margin-bottom: 24px; }
  .spec-item { display: flex; flex-direction: column; }
  .spec-label { font-size: 0.75rem; text-transform: uppercase; color: #666; font-weight: 600; letter-spacing: 0.05em; }
  .spec-value { font-size: 1rem; font-weight: 500; }
  .spec-screenshot { max-width: 100%; border: 1px solid #ddd; border-radius: 8px; margin-bottom: 24px; }
  .spec-code { background: #f5f5f5; border: 1px solid #ddd; border-radius: 6px; padding: 12px; font-family: monospace; font-size: 11px; white-space: pre-wrap; max-height: 300px; overflow: auto; margin-top: 16px; }
  .spec-footer { margin-top: 24px; font-size: 0.75rem; color: #999; border-top: 1px solid #eee; padding-top: 8px; }
  @media print { body { padding: 16px; } .spec-code { max-height: none; } }
</style></head><body>
<h1>${escapeHtml(modelName)}</h1>
${screenshotDataUrl ? `<img class="spec-screenshot" src="${escapeAttr(screenshotDataUrl)}" alt="3D Model" />` : ''}
<div class="spec-grid">
  <div class="spec-item"><span class="spec-label">${t('specDimensions')}</span><span class="spec-value">${formatNumber(dims[0])} x ${formatNumber(dims[1])} x ${formatNumber(dims[2])} mm</span></div>
  <div class="spec-item"><span class="spec-label">${t('specTriangles')}</span><span class="spec-value">${fmtInt(tris)}</span></div>
  <div class="spec-item"><span class="spec-label">${t('specVolume')}</span><span class="spec-value">${formatNumber(vol)} mm3</span></div>
  <div class="spec-item"><span class="spec-label">${t('specSurfaceArea')}</span><span class="spec-value">${formatNumber(area)} mm2</span></div>
  <div class="spec-item"><span class="spec-label">${t('specDensity')}</span><span class="spec-value">${density}</span></div>
  <div class="spec-item"><span class="spec-label">${t('specWeight')}</span><span class="spec-value">${formatNumber(weight)}</span></div>
</div>
<h3>${t('specSourceCode')}</h3>
<div class="spec-code">${escapeHtml(codeText)}</div>
<div class="spec-footer">${escapeHtml(t('specGenerated'))}: ${escapeHtml(now)}</div>
</body></html>`)
  win.document.close()
  win.focus()
  setTimeout(() => { win.print() }, 500)
}

/* ── Feature: UI Density ── */
type UIDensity = 'compact' | 'normal' | 'comfortable'
const uiDensity = ref<UIDensity>((localStorage.getItem('scad-ui-density') as UIDensity) || 'normal')

const UI_DENSITY_SCALES: Record<UIDensity, number> = {
  compact: 0.8,
  normal: 1.0,
  comfortable: 1.2,
}

watch(uiDensity, (v) => {
  localStorage.setItem('scad-ui-density', v)
  applyUIDensity()
})

function applyUIDensity() {
  const scale = UI_DENSITY_SCALES[uiDensity.value]
  document.documentElement.style.setProperty('--density-scale', String(scale))
  document.documentElement.style.setProperty('--density-padding', `${Math.round(8 * scale)}px`)
  document.documentElement.style.setProperty('--density-gap', `${Math.round(6 * scale)}px`)
  document.documentElement.style.setProperty('--density-font-size', `${Math.round(13 * scale)}px`)
}

/* ── Feature: Go to Line (Ctrl+G) ── */
const showGoToLine = ref(false)
const goToLineText = ref('')
const goToLineInputRef = ref<HTMLInputElement | null>(null)

function openGoToLine() {
  showGoToLine.value = true
  goToLineText.value = ''
  nextTick(() => goToLineInputRef.value?.focus())
}

function closeGoToLine() {
  showGoToLine.value = false
  goToLineText.value = ''
}

function executeGoToLine() {
  const lineNum = parseInt(goToLineText.value)
  if (isNaN(lineNum) || lineNum < 1) { closeGoToLine(); return }
  const el = textareaRef.value
  if (!el) { closeGoToLine(); return }
  const lines = code.value.split('\n')
  const targetLine = Math.min(lineNum, lines.length)
  // Calculate character offset for the target line
  let charOffset = 0
  for (let i = 0; i < targetLine - 1; i++) {
    charOffset += lines[i].length + 1 // +1 for \n
  }
  const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20
  el.scrollTo({ top: Math.max(0, (targetLine - 3) * lineHeight), behavior: 'smooth' })
  el.focus()
  el.setSelectionRange(charOffset, charOffset + (lines[targetLine - 1]?.length || 0))
  syncScroll()
  closeGoToLine()
}

function handleGoToLineKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') { e.preventDefault(); executeGoToLine() }
  if (e.key === 'Escape') { e.preventDefault(); closeGoToLine() }
}

/* ── Feature: Custom Theme Editor ── */
interface CustomTheme {
  name: string
  bg: string
  surface: string
  border: string
  text: string
  accent: string
}
const customThemes = ref<CustomTheme[]>((() => { const v = safeParse<CustomTheme[]>(localStorage.getItem('scad-custom-themes'), []); return Array.isArray(v) ? v : [] })())
const customThemeName = ref('')
const customThemeBg = ref('#1a1a2e')
const customThemeSurface = ref('#16213e')
const customThemeBorder = ref('#0f3460')
const customThemeText = ref('#e4e4e8')
const customThemeAccent = ref('#e94560')

function saveCustomTheme() {
  const name = customThemeName.value.trim()
  if (!name) return
  const existing = customThemes.value.findIndex(ct => ct.name === name)
  const themeData: CustomTheme = {
    name,
    bg: customThemeBg.value,
    surface: customThemeSurface.value,
    border: customThemeBorder.value,
    text: customThemeText.value,
    accent: customThemeAccent.value,
  }
  if (existing >= 0) {
    customThemes.value[existing] = themeData
  } else {
    customThemes.value.push(themeData)
  }
  localStorage.setItem('scad-custom-themes', JSON.stringify(customThemes.value))
  // Register as an EditorTheme
  registerCustomThemes()
  // Select the new theme
  selectTheme('custom-' + name)
  addToast(t('themeSaved'), 'success')
}

function deleteCustomTheme(name: string) {
  customThemes.value = customThemes.value.filter(ct => ct.name !== name)
  localStorage.setItem('scad-custom-themes', JSON.stringify(customThemes.value))
  registerCustomThemes()
  if (activeThemeId.value === 'custom-' + name) {
    selectTheme('default-dark')
  }
  addToast(t('themeDeleted'), 'info')
}

function registerCustomThemes() {
  // Remove old custom themes
  const baseCount = 6 // number of built-in themes
  while (EDITOR_THEMES.length > baseCount) EDITOR_THEMES.pop()
  // Add custom themes
  for (const ct of customThemes.value) {
    const isDarkTheme = isColorDark(ct.bg)
    EDITOR_THEMES.push({
      id: 'custom-' + ct.name,
      name: { ru: ct.name, en: ct.name, de: ct.name, zh: ct.name },
      dark: isDarkTheme,
      vars: {
        '--bg': ct.bg,
        '--surface': ct.surface,
        '--border': ct.border,
        '--text': ct.text,
        '--text-dim': adjustAlpha(ct.text, 0.6),
        '--accent': ct.accent,
        '--hover': adjustBrightness(ct.surface, isDarkTheme ? 1.2 : 0.95),
        '--canvas-bg': adjustBrightness(ct.bg, isDarkTheme ? 0.85 : 1.05),
        '--hl-comment': adjustAlpha(ct.text, 0.5),
        '--hl-keyword': ct.accent,
        '--hl-number': '#d19a66',
        '--hl-string': '#6ec87a',
        '--hl-boolean': '#c678dd',
        '--hl-special': '#56c8d8',
      },
    })
  }
}

function isColorDark(hex: string): boolean {
  const r = parseInt(hex.slice(1, 3), 16) || 0
  const g = parseInt(hex.slice(3, 5), 16) || 0
  const b = parseInt(hex.slice(5, 7), 16) || 0
  return (r * 0.299 + g * 0.587 + b * 0.114) < 128
}

function adjustAlpha(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16) || 0
  const g = parseInt(hex.slice(3, 5), 16) || 0
  const b = parseInt(hex.slice(5, 7), 16) || 0
  return `rgba(${r},${g},${b},${alpha})`
}

function adjustBrightness(hex: string, factor: number): string {
  const r = Math.min(255, Math.max(0, Math.round((parseInt(hex.slice(1, 3), 16) || 0) * factor)))
  const g = Math.min(255, Math.max(0, Math.round((parseInt(hex.slice(3, 5), 16) || 0) * factor)))
  const b = Math.min(255, Math.max(0, Math.round((parseInt(hex.slice(5, 7), 16) || 0) * factor)))
  return '#' + r.toString(16).padStart(2, '0') + g.toString(16).padStart(2, '0') + b.toString(16).padStart(2, '0')
}

function applyCustomThemePreview() {
  const root = document.documentElement
  root.style.setProperty('--bg', customThemeBg.value)
  root.style.setProperty('--surface', customThemeSurface.value)
  root.style.setProperty('--border', customThemeBorder.value)
  root.style.setProperty('--text', customThemeText.value)
  root.style.setProperty('--accent', customThemeAccent.value)
}

// Register custom themes on load
registerCustomThemes()

/* ── Feature: Playground Mode ── */
const showPlayground = ref(false)
const playgroundStep = ref(0)
const playgroundCompleted = ref<boolean[]>([false, false, false, false, false])

interface PlaygroundChallenge {
  goal: { ru: string; en: string; zh: string }
  hint: { ru: string; en: string; zh: string }
  initialCode: string
  check: (code: string, meshCount: number) => boolean
}

const PLAYGROUND_CHALLENGES: PlaygroundChallenge[] = [
  {
    goal: { ru: 'Создайте куб любого размера', en: 'Create a cube of any size', zh: '创建一个任意大小的立方体' },
    hint: { ru: 'Используйте cube([x,y,z]) или cube(size)', en: 'Use cube([x,y,z]) or cube(size)', zh: '使用 cube([x,y,z]) 或 cube(size)' },
    initialCode: '// Create a cube\n',
    check: (code, mc) => /\bcube\s*\(/.test(code) && mc > 0,
  },
  {
    goal: { ru: 'Добавьте сферу и куб', en: 'Add a sphere and a cube', zh: '添加一个球体和一个立方体' },
    hint: { ru: 'Используйте sphere(r=...) и cube()', en: 'Use sphere(r=...) and cube()', zh: '使用 sphere(r=...) 和 cube()' },
    initialCode: '// Create a sphere and a cube\n',
    check: (code, mc) => /\bsphere\s*\(/.test(code) && /\bcube\s*\(/.test(code) && mc >= 2,
  },
  {
    goal: { ru: 'Используйте difference() чтобы вырезать отверстие', en: 'Use difference() to cut a hole', zh: '使用 difference() 切割一个孔' },
    hint: { ru: 'difference() { куб; цилиндр; }', en: 'difference() { cube; cylinder; }', zh: 'difference() { cube; cylinder; }' },
    initialCode: '// Use difference() to subtract shapes\ndifference() {\n  \n}\n',
    check: (code, mc) => /\bdifference\s*\(\s*\)\s*\{/.test(code) && mc > 0,
  },
  {
    goal: { ru: 'Создайте модуль и вызовите его', en: 'Create a module and call it', zh: '创建一个模块并调用它' },
    hint: { ru: 'module myShape() { ... } myShape();', en: 'module myShape() { ... } myShape();', zh: 'module myShape() { ... } myShape();' },
    initialCode: '// Define a module and use it\n',
    check: (code, mc) => /\bmodule\s+\w+\s*\(/.test(code) && mc > 0,
  },
  {
    goal: { ru: 'Используйте for для создания массива объектов', en: 'Use for to create an array of objects', zh: '使用 for 创建对象数组' },
    hint: { ru: 'for(i=[0:4]) translate([i*10,0,0]) cube(5);', en: 'for(i=[0:4]) translate([i*10,0,0]) cube(5);', zh: 'for(i=[0:4]) translate([i*10,0,0]) cube(5);' },
    initialCode: '// Use a for loop to create multiple objects\n',
    check: (code, mc) => /\bfor\s*\(/.test(code) && mc >= 2,
  },
]

function openPlayground() {
  showPlayground.value = true
  playgroundStep.value = 0
  playgroundCompleted.value = [false, false, false, false, false]
  const challenge = PLAYGROUND_CHALLENGES[0]
  code.value = challenge.initialCode
  doRender()
}

function checkPlayground() {
  const challenge = PLAYGROUND_CHALLENGES[playgroundStep.value]
  if (!challenge) return
  const passed = challenge.check(code.value, meshCount.value)
  if (passed) {
    playgroundCompleted.value[playgroundStep.value] = true
    addToast(t('playgroundSuccess'), 'success')
  } else {
    addToast(t('playgroundFail'), 'error')
  }
}

function nextPlaygroundChallenge() {
  if (playgroundStep.value < PLAYGROUND_CHALLENGES.length - 1) {
    playgroundStep.value++
    const challenge = PLAYGROUND_CHALLENGES[playgroundStep.value]
    code.value = challenge.initialCode
    doRender()
  }
}

function resetPlayground() {
  playgroundStep.value = 0
  playgroundCompleted.value = [false, false, false, false, false]
  const challenge = PLAYGROUND_CHALLENGES[0]
  code.value = challenge.initialCode
  doRender()
}

/* ── Feature: Error Explanation Panel ── */
const showErrorExplanation = ref(false)

interface ErrorExplanationData {
  explanation: string
  fix: string
}

function getErrorExplanation(errorMsg: string): ErrorExplanationData | null {
  const msg = errorMsg.toLowerCase()
  if (msg.includes('unexpected token') || msg.includes('unexpected char')) {
    return {
      explanation: lang.value === 'ru'
        ? 'Обнаружен неожиданный символ. Возможно, есть синтаксическая ошибка.'
        : lang.value === 'zh'
        ? '发现意外字符。可能存在语法错误。'
        : 'An unexpected character was found. There might be a syntax error.',
      fix: lang.value === 'ru'
        ? 'Проверьте парные скобки, кавычки и правильность написания.'
        : lang.value === 'zh'
        ? '检查括号、引号和拼写是否正确。'
        : 'Check for unmatched brackets, quotes, or typos.',
    }
  }
  if (msg.includes('semicolon') || msg.includes(';')) {
    return {
      explanation: lang.value === 'ru'
        ? 'Возможно, пропущена точка с запятой в конце выражения.'
        : lang.value === 'zh'
        ? '可能在语句末尾缺少分号。'
        : 'You may have forgotten a semicolon at the end of a statement.',
      fix: lang.value === 'ru'
        ? 'Добавьте ; в конце строки с вызовом функции или выражением.'
        : lang.value === 'zh'
        ? '在函数调用或表达式的行末添加 ;。'
        : 'Add a ; at the end of the line with the function call or expression.',
    }
  }
  if (msg.includes('unknown') || msg.includes('not supported') || msg.includes('undefined')) {
    return {
      explanation: lang.value === 'ru'
        ? 'Использована неизвестная функция или переменная.'
        : lang.value === 'zh'
        ? '使用了未知的函数或变量。'
        : 'An unknown function or variable was used.',
      fix: lang.value === 'ru'
        ? 'Проверьте написание имени. Откройте справку OpenSCAD для списка доступных функций.'
        : lang.value === 'zh'
        ? '检查名称拼写。查看 OpenSCAD 参考获取可用函数列表。'
        : 'Check the spelling. See the OpenSCAD Reference for available functions.',
    }
  }
  if (msg.includes('bracket') || msg.includes('brace') || msg.includes('paren') || msg.includes('{') || msg.includes('}') || msg.includes('(') || msg.includes(')')) {
    return {
      explanation: lang.value === 'ru'
        ? 'Обнаружена проблема со скобками.'
        : lang.value === 'zh'
        ? '发现括号问题。'
        : 'There is a bracket or parenthesis issue.',
      fix: lang.value === 'ru'
        ? 'Убедитесь, что все открывающие скобки имеют закрывающие пары.'
        : lang.value === 'zh'
        ? '确保所有括号都有对应的配对。'
        : 'Make sure all opening brackets have matching closing brackets.',
    }
  }
  if (msg.includes('number') || msg.includes('argument') || msg.includes('parameter')) {
    return {
      explanation: lang.value === 'ru'
        ? 'Проблема с аргументами функции.'
        : lang.value === 'zh'
        ? '函数参数问题。'
        : 'There is an issue with the function arguments.',
      fix: lang.value === 'ru'
        ? 'Проверьте количество и тип аргументов функции.'
        : lang.value === 'zh'
        ? '检查函数参数的数量和类型。'
        : 'Check the number and type of arguments for the function.',
    }
  }
  // Generic fallback
  return {
    explanation: lang.value === 'ru'
      ? 'Произошла ошибка при разборе кода.'
      : lang.value === 'zh'
      ? '解析代码时发生错误。'
      : 'An error occurred while parsing the code.',
    fix: lang.value === 'ru'
      ? 'Проверьте синтаксис вашего кода. Убедитесь в правильности скобок и точек с запятой.'
      : lang.value === 'zh'
      ? '检查代码语法。确保括号和分号正确。'
      : 'Review your code syntax. Make sure brackets and semicolons are correct.',
  }
}

/* ── Feature: Command Palette (Ctrl+Shift+P / F1) ── */
const showCommandPalette = ref(false)
const commandSearch = ref('')
const commandSelectedIndex = ref(0)
const commandSearchInputRef = ref<HTMLInputElement | null>(null)

interface PaletteCommand {
  id: string
  label: () => string
  shortcut?: string
  action: () => void
}

const paletteCommands: PaletteCommand[] = [
  { id: 'render', label: () => t('cmdRender'), shortcut: 'Ctrl+Enter', action: () => doRender() },
  { id: 'format', label: () => t('cmdFormatCode'), action: () => formatCode() },
  { id: 'wireframe', label: () => t('cmdToggleWireframe'), action: () => toggleWireframe() },
  { id: 'grid', label: () => t('cmdToggleGrid'), action: () => toggleGrid() },
  { id: 'autoRotate', label: () => t('cmdToggleAutoRotate'), action: () => toggleAutoRotate() },
  { id: 'fastPreview', label: () => t('cmdToggleFastPreview'), action: () => toggleFastPreview() },
  { id: 'fullscreen', label: () => t('cmdToggleFullscreen'), action: () => toggleFullscreen() },
  { id: 'ortho', label: () => t('cmdToggleOrtho'), action: () => toggleProjection() },
  { id: 'screenshot', label: () => t('cmdTakeScreenshot'), action: () => takeScreenshot() },
  { id: 'exportPng2x', label: () => t('cmdExportPng2x'), action: () => exportPng(2) },
  { id: 'exportPng4x', label: () => t('cmdExportPng4x'), action: () => exportPng(4) },
  { id: 'copyImage', label: () => t('cmdCopyImage'), action: () => copyCanvasToClipboard() },
  { id: 'exportStl', label: () => t('cmdExportSTL'), action: () => doExportSTL() },
  { id: 'exportObj', label: () => t('cmdExportOBJ'), action: () => doExportOBJ() },
  { id: 'export3mf', label: () => t('cmdExport3MF'), action: () => doExport3MF() },
  { id: 'openFile', label: () => t('cmdOpenFile'), shortcut: 'Ctrl+O', action: () => openFile() },
  { id: 'saveFile', label: () => t('cmdSaveFile'), shortcut: 'Ctrl+S', action: () => saveFile() },
  { id: 'share', label: () => t('cmdShare'), action: () => shareLink() },
  { id: 'toggleTheme', label: () => t('cmdToggleTheme'), action: () => toggleTheme() },
  { id: 'toggleLang', label: () => t('cmdToggleLang'), action: () => toggleLang() },
  { id: 'shortcuts', label: () => t('cmdShowShortcuts'), shortcut: '?', action: () => { showShortcuts.value = true } },
  { id: 'preferences', label: () => t('cmdPreferences'), action: () => { showPreferences.value = true } },
  { id: 'wordWrap', label: () => t('cmdToggleWordWrap'), shortcut: 'Alt+Z', action: () => toggleWordWrap() },
  { id: 'goToLine', label: () => t('cmdGoToLine'), shortcut: 'Ctrl+G', action: () => openGoToLine() },
  { id: 'find', label: () => t('sc_findOnly'), shortcut: 'Ctrl+F', action: () => openFindReplace(false) },
  { id: 'findReplace', label: () => t('sc_findReplace'), shortcut: 'Ctrl+H', action: () => openFindReplace(true) },
  { id: 'minimap', label: () => t('cmdToggleMinimap'), action: () => toggleMinimap() },
  { id: 'console', label: () => t('cmdToggleConsole'), action: () => { showConsole.value = !showConsole.value } },
  { id: 'objectTree', label: () => t('cmdToggleObjectTree'), action: () => { showObjectTree.value = !showObjectTree.value } },
  { id: 'newTab', label: () => t('cmdNewTab'), action: () => addTab() },
  { id: 'statistics', label: () => t('cmdStatistics'), action: () => toggleStatistics() },
  { id: 'buildPlate', label: () => t('buildPlate'), action: () => toggleBuildPlate() },
  { id: 'showWelcome', label: () => t('cmdShowWelcome'), action: () => openWelcome() },
  { id: 'importStl', label: () => t('cmdImportSTL'), action: () => openImportSTL() },
  { id: 'perfPanel', label: () => t('cmdPerfPanel'), action: () => { showPerfPanel.value = !showPerfPanel.value } },
  { id: 'splitEditor', label: () => t('splitEditor'), shortcut: 'Ctrl+\\', action: () => toggleSplitMode() },
  { id: 'history', label: () => t('cmdToggleHistory'), action: () => { showHistory.value = !showHistory.value } },
  { id: 'simpleMode', label: () => t('modeToggle') + ': ' + (simpleMode.value ? t('advancedMode') : t('simpleMode')), action: () => { simpleMode.value = !simpleMode.value } },
  { id: 'exampleGallery', label: () => t('cmdExampleGallery'), action: () => openExampleGallery() },
  { id: 'insertSnippet:centeredCube', label: () => 'Insert: ' + t('snippetCenteredCube'), action: () => insertSnippet(SNIPPETS[0]) },
  { id: 'insertSnippet:cylinderHole', label: () => 'Insert: ' + t('snippetCylinderHole'), action: () => insertSnippet(SNIPPETS[1]) },
  { id: 'insertSnippet:roundedBox', label: () => 'Insert: ' + t('snippetRoundedBox'), action: () => insertSnippet(SNIPPETS[2]) },
  { id: 'insertSnippet:arrayPattern', label: () => 'Insert: ' + t('snippetArrayPattern'), action: () => insertSnippet(SNIPPETS[3]) },
  { id: 'insertSnippet:parametricModule', label: () => 'Insert: ' + t('snippetParametricModule'), action: () => insertSnippet(SNIPPETS[4]) },
  { id: 'insertSnippet:threadedInsert', label: () => 'Insert: ' + t('snippetThreadedInsert'), action: () => insertSnippet(SNIPPETS[5]) },
  { id: 'insertSnippet:hexGrid', label: () => 'Insert: ' + t('snippetHexGrid'), action: () => insertSnippet(SNIPPETS[6]) },
  { id: 'insertSnippet:bevel', label: () => 'Insert: ' + t('snippetBevel'), action: () => insertSnippet(SNIPPETS[7]) },
  { id: 'insertSnippet:spring', label: () => 'Insert: ' + t('snippetSpring'), action: () => insertSnippet(SNIPPETS[8]) },
  { id: 'insertSnippet:boxWithLid', label: () => 'Insert: ' + t('snippetBoxWithLid'), action: () => insertSnippet(SNIPPETS[9]) },
  { id: 'insertSnippet:gearWheel', label: () => 'Insert: ' + t('snippetGearWheel'), action: () => insertSnippet(SNIPPETS[10]) },
  { id: 'exportAllTabs', label: () => t('cmdExportAllTabs'), action: () => doExportAllTabs() },
  { id: 'cameraInfo', label: () => t('showCameraInfo'), action: () => toggleCameraInfo() },
  { id: 'scadReference', label: () => t('cmdScadReference'), action: () => toggleScadReference() },
  { id: 'measureTool', label: () => t('cmdToggleMeasure'), action: () => toggleMeasureMode() },
  { id: 'ghostCompare', label: () => t('cmdToggleGhost'), action: () => toggleGhostMode() },
  { id: 'printCode', label: () => t('cmdPrintCode'), action: () => printCode() },
  { id: 'tour', label: () => t('tourStart'), action: () => startTour() },
  { id: 'snapshotGallery', label: () => t('cmdSnapshotGallery'), action: () => { showSnapshotGallery.value = !showSnapshotGallery.value } },
  { id: 'profilePanel', label: () => t('cmdToggleProfile'), action: () => { showProfilePanel.value = !showProfilePanel.value } },
  { id: 'toonShading', label: () => t('cmdToggleToon'), action: () => toggleToonShading() },
  { id: 'vignette', label: () => t('cmdToggleVignette'), action: () => toggleVignette() },
  { id: 'orbitInertia', label: () => t('cmdToggleInertia'), action: () => toggleInertia() },
  { id: 'sortLinesAsc', label: () => t('sortLinesAsc'), action: () => sortLines('asc') },
  { id: 'sortLinesDesc', label: () => t('sortLinesDesc'), action: () => sortLines('desc') },
  { id: 'upperCase', label: () => t('toggleUpperCase'), shortcut: 'Ctrl+Shift+U', action: () => toggleCase('upper') },
  { id: 'lowerCase', label: () => t('toggleLowerCase'), shortcut: 'Ctrl+Shift+L', action: () => toggleCase('lower') },
  { id: 'joinLines', label: () => t('joinLines'), shortcut: 'Ctrl+J', action: () => joinLinesCmd() },
  { id: 'goochShading', label: () => t('cmdToggleGooch'), action: () => toggleGoochShading() },
  { id: 'groundShadow', label: () => t('cmdToggleGroundShadow'), action: () => toggleGroundShadow() },
  { id: 'zenMode', label: () => t('cmdToggleZen'), shortcut: 'Ctrl+K Z', action: () => toggleZenMode() },
  { id: 'flyCamera', label: () => t('cmdToggleFly'), action: () => toggleFlyCamera() },
  { id: 'bloom', label: () => t('cmdToggleBloom'), action: () => toggleBloom() },
  { id: 'sectionBox', label: () => t('cmdToggleSectionBox'), action: () => toggleSectionBox() },
  { id: 'turntableExport', label: () => t('exportTurntable'), action: () => exportTurntableZip() },
  { id: 'renderAll', label: () => t('cmdRenderAll'), action: () => doRenderAllTabs() },
  { id: 'playground', label: () => t('cmdPlayground'), action: () => openPlayground() },
]

function fuzzyMatch(needle: string, haystack: string): boolean {
  needle = needle.toLowerCase()
  haystack = haystack.toLowerCase()
  let ni = 0
  for (let hi = 0; hi < haystack.length && ni < needle.length; hi++) {
    if (haystack[hi] === needle[ni]) ni++
  }
  return ni === needle.length
}

const filteredCommands = computed(() => {
  const q = commandSearch.value.trim()
  if (!q) return paletteCommands
  return paletteCommands.filter(cmd => fuzzyMatch(q, cmd.label()))
})

function openCommandPalette() {
  showCommandPalette.value = true
  commandSearch.value = ''
  commandSelectedIndex.value = 0
  nextTick(() => commandSearchInputRef.value?.focus())
}

function closeCommandPalette() {
  showCommandPalette.value = false
  commandSearch.value = ''
}

function executeCommand(cmd: PaletteCommand) {
  closeCommandPalette()
  nextTick(() => cmd.action())
}

function scrollCommandIntoView() {
  nextTick(() => {
    const el = document.querySelector('.command-palette-item.active')
    if (el) el.scrollIntoView({ block: 'nearest' })
  })
}

function handleCommandPaletteKeydown(e: KeyboardEvent) {
  const cmds = filteredCommands.value
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    commandSelectedIndex.value = (commandSelectedIndex.value + 1) % cmds.length
    scrollCommandIntoView()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    commandSelectedIndex.value = (commandSelectedIndex.value - 1 + cmds.length) % cmds.length
    scrollCommandIntoView()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (cmds.length > 0) {
      executeCommand(cmds[commandSelectedIndex.value])
    }
  } else if (e.key === 'Escape') {
    e.preventDefault()
    closeCommandPalette()
  }
}

watch(commandSearch, () => {
  commandSelectedIndex.value = 0
})

/* ── Feature: Animation Timeline ($t variable) ── */
const animT = ref(0)
const animPlaying = ref(false)
const animDuration = ref(5)
let animStartTime = 0
let animRAF = 0

function startAnimation() {
  animPlaying.value = true
  animStartTime = performance.now() - (animT.value * animDuration.value * 1000)
  animLoop()
}

function stopAnimation() {
  animPlaying.value = false
  if (animRAF) { cancelAnimationFrame(animRAF); animRAF = 0 }
}

function toggleAnimation() {
  if (animPlaying.value) stopAnimation()
  else startAnimation()
}

function animLoop() {
  if (!animPlaying.value) return
  const elapsed = performance.now() - animStartTime
  const dur = animDuration.value * 1000
  animT.value = (elapsed % dur) / dur
  doRender()
  animRAF = requestAnimationFrame(animLoop)
}

function onAnimSliderInput(e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value)
  animT.value = val
  if (animPlaying.value) {
    animStartTime = performance.now() - (val * animDuration.value * 1000)
  }
  doRender()
}

onUnmounted(() => {
  if (animRAF) cancelAnimationFrame(animRAF)
})

/* ── Feature: Parameterizer ── */
interface ParamVar {
  name: string
  value: number | string | boolean
  min: number
  max: number
  step: number
  type: 'slider' | 'dropdown' | 'checkbox'
  options?: string[]
}

const showParameters = ref(false)
const extractedParams = ref<ParamVar[]>([])

function extractParameters() {
  const src = code.value
  const lines = src.split('\n')
  const params: ParamVar[] = []
  const reNumMeta = /^\s*(\w+)\s*=\s*(-?\d+\.?\d*)\s*;\s*\/\/\s*\[([^\]]*)\]/
  const reStrMeta = /^\s*(\w+)\s*=\s*"([^"]*)"\s*;\s*\/\/\s*\[([^\]]*)\]/
  const reBool = /^\s*(\w+)\s*=\s*(true|false)\s*;/
  const reNumPlain = /^\s*(\w+)\s*=\s*(-?\d+\.?\d*)\s*;/
  for (const line of lines) {
    let m: RegExpMatchArray | null
    // Numeric with metadata: value = 20; // [5:50] or [5:1:50]
    if ((m = line.match(reNumMeta))) {
      const name = m[1]
      const val = parseFloat(m[2])
      const meta = m[3]
      const parts = meta.split(':').map(s => s.trim())
      let min: number, max: number, step: number
      if (parts.length === 2) {
        min = parseFloat(parts[0]); max = parseFloat(parts[1])
        step = Math.abs(max - min) >= 10 ? 1 : 0.1
      } else if (parts.length === 3) {
        min = parseFloat(parts[0]); step = parseFloat(parts[1]); max = parseFloat(parts[2])
      } else {
        // Fallback: check if it's a comma-separated list (string options on a number?)
        min = val * 0.1; max = val * 3; step = 1
        if (min > max) { const tmp = min; min = max; max = tmp }
      }
      params.push({ name, value: val, min, max, step, type: 'slider' })
    // String with options: shape = "round"; // [round, square, hex]
    } else if ((m = line.match(reStrMeta))) {
      const name = m[1]
      const val = m[2]
      const options = m[3].split(',').map(s => s.trim())
      params.push({ name, value: val, min: 0, max: 0, step: 0, type: 'dropdown', options })
    // Boolean: show_holes = true;
    } else if ((m = line.match(reBool))) {
      const name = m[1]
      const val = m[2] === 'true'
      params.push({ name, value: val, min: 0, max: 0, step: 0, type: 'checkbox' })
    // Plain numeric (no metadata)
    } else if ((m = line.match(reNumPlain))) {
      const name = m[1]
      const val = parseFloat(m[2])
      let min: number, max: number, step: number
      if (val === 0) {
        min = -100; max = 100; step = 1
      } else {
        min = val * 0.1
        max = val * 3
        if (min > max) { const tmp = min; min = max; max = tmp }
        step = Math.abs(val) >= 10 ? 1 : 0.1
      }
      params.push({ name, value: val, min, max, step, type: 'slider' })
    }
  }
  extractedParams.value = params
}

function onParamChange(param: ParamVar, newVal: number | string | boolean) {
  param.value = newVal
  const src = code.value
  let updated = src
  if (param.type === 'slider') {
    const re = new RegExp(`^(\\s*${param.name}\\s*=\\s*)(-?\\d+\\.?\\d*)(\\s*;)`, 'm')
    updated = src.replace(re, `$1${newVal}$3`)
  } else if (param.type === 'dropdown') {
    const re = new RegExp(`^(\\s*${param.name}\\s*=\\s*")([^"]*)(")`, 'm')
    updated = src.replace(re, `$1${newVal}$3`)
  } else if (param.type === 'checkbox') {
    const re = new RegExp(`^(\\s*${param.name}\\s*=\\s*)(true|false)(\\s*;)`, 'm')
    updated = src.replace(re, `$1${newVal}$3`)
  }
  if (updated !== src) {
    code.value = updated
  }
}

// Extract parameters when code changes (debounced via existing code watcher)
watch(code, () => {
  if (showParameters.value) {
    extractParameters()
  }
})

watch(showParameters, (v) => {
  if (v) extractParameters()
})

/* ── Feature: Diff Mode ── */
const showDiff = ref(false)

interface DiffLine {
  type: 'same' | 'added' | 'removed' | 'modified'
  lineNum: number
  text: string
}

function computeDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = oldText.split('\n')
  const newLines = newText.split('\n')
  const result: DiffLine[] = []
  const maxLen = Math.max(oldLines.length, newLines.length)
  for (let i = 0; i < maxLen; i++) {
    if (i >= newLines.length) {
      // Line existed in old but not in new → removed
      result.push({ type: 'removed', lineNum: i + 1, text: oldLines[i] })
    } else if (i >= oldLines.length) {
      // Line exists in new but not in old → added
      result.push({ type: 'added', lineNum: i + 1, text: newLines[i] })
    } else if (oldLines[i] === newLines[i]) {
      result.push({ type: 'same', lineNum: i + 1, text: newLines[i] })
    } else {
      result.push({ type: 'modified', lineNum: i + 1, text: newLines[i] })
    }
  }
  return result
}

const diffLines = computed(() => {
  const saved = activeTab.value.savedCode ?? ''
  return computeDiff(saved, code.value)
})

const diffStats = computed(() => {
  const lines = diffLines.value
  return {
    added: lines.filter(l => l.type === 'added').length,
    removed: lines.filter(l => l.type === 'removed').length,
    modified: lines.filter(l => l.type === 'modified').length,
  }
})

const hasUnsavedChanges = computed(() => {
  return activeTab.value.savedCode !== undefined && activeTab.value.savedCode !== code.value
})

/* ── Feature: Lighting Presets ── */
const activeLighting = ref('default')

function setLighting(preset: string) {
  activeLighting.value = preset
  renderer?.setLighting(preset)
}

/* ── Feature: Clipping Plane ── */
const clipEnabled = ref(false)
const clipY = ref(0)
const clipAxis = ref(1) // 0=X, 1=Y, 2=Z

function toggleClip() {
  clipEnabled.value = !clipEnabled.value
  renderer?.setClipEnabled(clipEnabled.value)
}

function onClipYChange(e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value)
  clipY.value = val
  renderer?.setClipValue(val)
}

function onClipAxisChange(e: Event) {
  const val = parseInt((e.target as HTMLSelectElement).value)
  clipAxis.value = val
  renderer?.setClipAxis(val)
  // Reset clip value to middle of new axis range
  const range = clipRange.value
  clipY.value = (range.min + range.max) / 2
  renderer?.setClipValue(clipY.value)
}

const clipRange = computed(() => {
  if (!renderer) return { min: -50, max: 50 }
  return renderer.getClipRange()
})

/* ── Feature: Fog ── */
const fogEnabled = ref(false)

function toggleFog() {
  fogEnabled.value = !fogEnabled.value
  renderer?.setFogEnabled(fogEnabled.value)
}

/* ── Feature: Reflection ── */
const reflectionEnabled = ref(false)

function toggleReflection() {
  reflectionEnabled.value = !reflectionEnabled.value
  renderer?.setReflection(reflectionEnabled.value)
}

/* ── Feature: Render Mode ── */
function setRenderMode(mode: string) {
  activeRenderMode.value = mode
  renderer?.setRenderMode(mode)
  // Sync wireframe ref for any UI that still reads it
  showWireframe.value = mode === 'wireframe'
}

/* ── Feature: Flat Shading ── */
function toggleFlatShading() {
  flatShadingEnabled.value = !flatShadingEnabled.value
  renderer?.setFlatShading(flatShadingEnabled.value)
}

/* ── Feature: SSAO ── */
function toggleSSAO() {
  ssaoEnabled.value = !ssaoEnabled.value
  renderer?.setSsao(ssaoEnabled.value)
}

/* ── Feature: Outline ── */
function toggleOutline() {
  outlineEnabled.value = !outlineEnabled.value
  renderer?.setOutline(outlineEnabled.value)
}

/* ── Feature: Normal Smoothing ── */
function toggleNormalSmoothing() {
  smoothNormalsEnabled.value = !smoothNormalsEnabled.value
  renderer?.setNormalSmoothing(smoothNormalsEnabled.value)
}

/* ── Feature: Toon Shading ── */
function toggleToonShading() {
  toonShadingEnabled.value = !toonShadingEnabled.value
  renderer?.setToonShading(toonShadingEnabled.value)
}

/* ── Feature: Gooch Shading (batch 35) ── */
function toggleGoochShading() {
  goochShadingEnabled.value = !goochShadingEnabled.value
  renderer?.toggleGoochShading()
}

/* ── Feature: Ground Shadow (batch 35) ── */
function toggleGroundShadow() {
  groundShadowEnabled.value = !groundShadowEnabled.value
  renderer?.toggleGroundShadow()
}

/* ── Feature: Zen Mode (batch 35) ── */
function toggleZenMode() {
  zenModeActive.value = !zenModeActive.value
}

/* ── Feature: Tips of the Day (batch 35) ── */
const TIPS = [
  { ru: 'Используйте Ctrl+Enter для быстрого рендера.', en: 'Use Ctrl+Enter for quick render.' },
  { ru: 'Ctrl+ЛКМ привязывает вращение к 15°.', en: 'Ctrl+LMB snaps rotation to 15 degrees.' },
  { ru: 'WASD управляет камерой. Shift+WASD панорамирует.', en: 'WASD controls the camera. Shift+WASD pans.' },
  { ru: 'Нажмите F для вписывания модели в экран.', en: 'Press F to fit the model to screen.' },
  { ru: 'Попробуйте режим Рентген (X-Ray) для просмотра внутренней геометрии.', en: 'Try X-Ray mode to see internal geometry.' },
  { ru: 'Ctrl+Shift+P открывает палитру команд.', en: 'Ctrl+Shift+P opens the command palette.' },
  { ru: 'Используйте $fn для контроля детализации окружностей.', en: 'Use $fn to control circle resolution.' },
  { ru: 'Экспортируйте в STL, OBJ или 3MF из меню Экспорт.', en: 'Export to STL, OBJ, or 3MF from the Export menu.' },
]

const currentTip = ref(TIPS[Math.floor(Math.random() * TIPS.length)])

function dismissTipOfDay() {
  showTipOfDay.value = false
  tipOfDayDismissed.value = true
  localStorage.setItem('scad-no-tips', 'true')
}

function nextTip() {
  currentTip.value = TIPS[Math.floor(Math.random() * TIPS.length)]
}

/* ── Feature: Turntable ZIP Export (batch 35) ── */
async function exportTurntableZip() {
  if (!renderer || turntableExporting.value) return
  turntableExporting.value = true
  addToast(t('turntableProgress'), 'info')

  try {
    const frames: Blob[] = []
    const frameCount = 36
    const origYaw = renderer.yaw
    for (let i = 0; i < frameCount; i++) {
      renderer.yaw = origYaw + (i / frameCount) * Math.PI * 2
      renderer.requestRender()
      // Wait for one frame
      await new Promise(r => requestAnimationFrame(r))
      const blob = await renderer.captureFrameAsBlob()
      if (blob) frames.push(blob)
    }
    renderer.yaw = origYaw
    renderer.requestRender()

    // Build a ZIP with the frames
    // Simple ZIP building - using the existing exportAllTabsAsZip pattern
    // but we'll build it manually for binary data
    const zipParts: { name: string; data: Uint8Array }[] = []
    for (let i = 0; i < frames.length; i++) {
      const ab = await frames[i].arrayBuffer()
      zipParts.push({ name: `frame_${String(i).padStart(3, '0')}.png`, data: new Uint8Array(ab) })
    }

    // Simple ZIP creator (no compression, store only)
    const encoder = new TextEncoder()
    const parts: Uint8Array[] = []
    const centralDir: Uint8Array[] = []
    let offset = 0
    for (const file of zipParts) {
      const nameBytes = encoder.encode(file.name)
      // Local file header
      const lh = new Uint8Array(30 + nameBytes.length)
      const lhv = new DataView(lh.buffer)
      lhv.setUint32(0, 0x04034b50, true) // signature
      lhv.setUint16(4, 20, true) // version
      lhv.setUint16(8, 0, true) // method: stored
      lhv.setUint32(18, file.data.length, true) // compressed
      lhv.setUint32(22, file.data.length, true) // uncompressed
      lhv.setUint16(26, nameBytes.length, true)
      lh.set(nameBytes, 30)
      parts.push(lh)
      parts.push(file.data)

      // Central directory entry
      const cd = new Uint8Array(46 + nameBytes.length)
      const cdv = new DataView(cd.buffer)
      cdv.setUint32(0, 0x02014b50, true)
      cdv.setUint16(4, 20, true)
      cdv.setUint16(6, 20, true)
      cdv.setUint32(20, file.data.length, true)
      cdv.setUint32(24, file.data.length, true)
      cdv.setUint16(28, nameBytes.length, true)
      cdv.setUint32(42, offset, true)
      cd.set(nameBytes, 46)
      centralDir.push(cd)

      offset += lh.length + file.data.length
    }

    const cdOffset = offset
    let cdSize = 0
    for (const cd of centralDir) { parts.push(cd); cdSize += cd.length }

    // End of central directory
    const eocd = new Uint8Array(22)
    const eocdv = new DataView(eocd.buffer)
    eocdv.setUint32(0, 0x06054b50, true)
    eocdv.setUint16(8, zipParts.length, true)
    eocdv.setUint16(10, zipParts.length, true)
    eocdv.setUint32(12, cdSize, true)
    eocdv.setUint32(16, cdOffset, true)
    parts.push(eocd)

    const totalLen = parts.reduce((s, p) => s + p.length, 0)
    const zipData = new Uint8Array(totalLen)
    let pos = 0
    for (const p of parts) { zipData.set(p, pos); pos += p.length }

    const blob = new Blob([zipData], { type: 'application/zip' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `turntable-${Date.now()}.zip`
    a.click()
    URL.revokeObjectURL(url)
    addToast(t('turntableComplete'), 'success')
  } catch (err) {
    console.error('Turntable export failed:', err)
  } finally {
    turntableExporting.value = false
  }
}

/* ── Feature: Fly Camera Mode (batch 36) ── */
function toggleFlyCamera() {
  flyCameraMode.value = !flyCameraMode.value
  renderer?.setFlyMode(flyCameraMode.value)
}

/* ── Feature: Bloom Effect (batch 36) ── */
function toggleBloom() {
  bloomEnabled.value = !bloomEnabled.value
}

/* ── Feature: Section Box (batch 36) ── */
function toggleSectionBox() {
  sectionBoxEnabled.value = !sectionBoxEnabled.value
  renderer?.setSectionBoxEnabled(sectionBoxEnabled.value)
  if (sectionBoxEnabled.value && renderer) {
    // Initialize section box values to middle of bounds
    const rx = renderer.getSectionBoxRange(0)
    const ry = renderer.getSectionBoxRange(1)
    const rz = renderer.getSectionBoxRange(2)
    sectionBoxX.value = (rx.min + rx.max) / 2
    sectionBoxY.value = (ry.min + ry.max) / 2
    sectionBoxZ.value = (rz.min + rz.max) / 2
    renderer.setSectionBoxValues(sectionBoxX.value, sectionBoxY.value, sectionBoxZ.value)
  }
}

function onSectionBoxChange(axis: number, e: Event) {
  const val = parseFloat((e.target as HTMLInputElement).value)
  if (axis === 0) sectionBoxX.value = val
  else if (axis === 1) sectionBoxY.value = val
  else sectionBoxZ.value = val
  renderer?.setSectionBoxAxis(axis, val)
}

const sectionBoxRangeX = computed(() => renderer ? renderer.getSectionBoxRange(0) : { min: -50, max: 50 })
const sectionBoxRangeY = computed(() => renderer ? renderer.getSectionBoxRange(1) : { min: -50, max: 50 })
const sectionBoxRangeZ = computed(() => renderer ? renderer.getSectionBoxRange(2) : { min: -50, max: 50 })

/* ── Feature: Per-Object Color Override (batch 36) ── */
function setObjectColorOverride(meshIndex: number, hexColor: string) {
  // Convert hex to RGBA float array
  const r = parseInt(hexColor.slice(1, 3), 16) / 255
  const g = parseInt(hexColor.slice(3, 5), 16) / 255
  const b = parseInt(hexColor.slice(5, 7), 16) / 255
  meshColorOverrides.value.set(meshIndex, hexColor)
  renderer?.setMeshColorOverride(meshIndex, [r, g, b, 1.0])
}

function clearObjectColorOverride(meshIndex: number) {
  meshColorOverrides.value.delete(meshIndex)
  renderer?.clearMeshColorOverride(meshIndex)
}

function getMeshColorHex(meshIndex: number): string {
  const override = meshColorOverrides.value.get(meshIndex)
  if (override) return override
  if (renderer) {
    const c = renderer.getMeshColor(meshIndex)
    if (c) {
      const r = Math.round(c[0] * 255).toString(16).padStart(2, '0')
      const g = Math.round(c[1] * 255).toString(16).padStart(2, '0')
      const b = Math.round(c[2] * 255).toString(16).padStart(2, '0')
      return `#${r}${g}${b}`
    }
  }
  return '#888888'
}

/* ── Feature: Mesh Visibility Toggle (batch 35) ── */
function toggleMeshVisibility(meshIndex: number) {
  const current = meshVisibilityMap.value.get(meshIndex) ?? true
  const newVal = !current
  meshVisibilityMap.value.set(meshIndex, newVal)
  renderer?.setMeshVisibility(meshIndex, newVal)
}

function isMeshVisible(meshIndex: number): boolean {
  return meshVisibilityMap.value.get(meshIndex) ?? true
}

/* ── Feature: Exploded View ── */
function onExplodeChange(e: Event) {
  const v = parseFloat((e.target as HTMLInputElement).value)
  explodeFactorVal.value = v
  renderer?.setExplode(v)
}

/* ── Feature: Vignette ── */
function toggleVignette() {
  vignetteEnabled.value = !vignetteEnabled.value
}

/* ── Feature: FOV Slider ── */
function onFovChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  fovDeg.value = v
  renderer?.setFov(v * Math.PI / 180)
}

/* ── Feature: Orbit Inertia ── */
function toggleInertia() {
  inertiaEnabled.value = !inertiaEnabled.value
  renderer?.setInertia(inertiaEnabled.value)
  localStorage.setItem('scad-orbit-inertia', String(inertiaEnabled.value))
}

/* ── Feature: Sort Lines ── */
function sortLines(direction: 'asc' | 'desc') {
  const el = textareaRef.value
  if (!el) return
  const src = code.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd
  const hasSelection = selStart !== selEnd

  let lineStartIdx: number, lineEndIdx: number
  if (hasSelection) {
    lineStartIdx = src.lastIndexOf('\n', selStart - 1) + 1
    lineEndIdx = src.indexOf('\n', selEnd)
    if (lineEndIdx === -1) lineEndIdx = src.length
  } else {
    lineStartIdx = 0
    lineEndIdx = src.length
  }

  const block = src.substring(lineStartIdx, lineEndIdx)
  const lines = block.split('\n')
  lines.sort((a, b) => direction === 'asc' ? a.localeCompare(b) : b.localeCompare(a))
  const newBlock = lines.join('\n')
  code.value = src.substring(0, lineStartIdx) + newBlock + src.substring(lineEndIdx)
  nextTick(() => {
    el.selectionStart = lineStartIdx
    el.selectionEnd = lineStartIdx + newBlock.length
    el.focus()
  })
}

/* ── Feature: Toggle Case ── */
function toggleCase(mode: 'upper' | 'lower') {
  const el = textareaRef.value
  if (!el) return
  const src = code.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd
  if (selStart === selEnd) return // no selection

  const selected = src.substring(selStart, selEnd)
  const transformed = mode === 'upper' ? selected.toUpperCase() : selected.toLowerCase()
  code.value = src.substring(0, selStart) + transformed + src.substring(selEnd)
  nextTick(() => {
    el.selectionStart = selStart
    el.selectionEnd = selStart + transformed.length
    el.focus()
  })
}

/* ── Feature: Join Lines ── */
function joinLinesCmd() {
  const el = textareaRef.value
  if (!el) return
  const src = code.value
  const selStart = el.selectionStart
  const selEnd = el.selectionEnd

  if (selStart !== selEnd) {
    // Join all selected lines into one
    const lineStartIdx = src.lastIndexOf('\n', selStart - 1) + 1
    let lineEndIdx = src.indexOf('\n', selEnd)
    if (lineEndIdx === -1) lineEndIdx = src.length
    const block = src.substring(lineStartIdx, lineEndIdx)
    const joined = block.split('\n').map(l => l.trim()).join(' ')
    code.value = src.substring(0, lineStartIdx) + joined + src.substring(lineEndIdx)
    nextTick(() => {
      el.selectionStart = lineStartIdx
      el.selectionEnd = lineStartIdx + joined.length
      el.focus()
    })
  } else {
    // Join current line with next
    let lineEnd = src.indexOf('\n', selStart)
    if (lineEnd === -1) return // last line, nothing to join
    const before = src.substring(0, lineEnd)
    const after = src.substring(lineEnd + 1)
    // Trim leading whitespace from the next line
    const afterTrimmed = after.replace(/^\s+/, '')
    const joined = before + ' ' + afterTrimmed
    code.value = joined
    nextTick(() => {
      el.selectionStart = el.selectionEnd = lineEnd + 1
      el.focus()
    })
  }
}

/* ── Feature: Color Grading ── */
function setColorGrading(preset: string) {
  colorGrading.value = preset
}

/* ── Feature: Skybox / Environment Background ── */
function setSkyPreset(preset: string) {
  skyPreset.value = preset
  renderer?.setSkyPreset(preset)
}

/* ── Feature 3: Build Plate / Print Bed ── */
const buildPlateEnabled = ref(localStorage.getItem('scad-buildplate') === 'true')
const buildPlateX = ref(parseInt(localStorage.getItem('scad-buildplate-x') || '220') || 220)
const buildPlateZ = ref(parseInt(localStorage.getItem('scad-buildplate-z') || '220') || 220)

function applyBuildPlate() {
  renderer?.setBuildPlate(buildPlateEnabled.value, buildPlateX.value, buildPlateZ.value)
}

function toggleBuildPlate() {
  buildPlateEnabled.value = !buildPlateEnabled.value
  localStorage.setItem('scad-buildplate', String(buildPlateEnabled.value))
  applyBuildPlate()
}

function onBuildPlateDimChange() {
  if (buildPlateX.value < 1) buildPlateX.value = 1
  if (buildPlateZ.value < 1) buildPlateZ.value = 1
  localStorage.setItem('scad-buildplate-x', String(buildPlateX.value | 0))
  localStorage.setItem('scad-buildplate-z', String(buildPlateZ.value | 0))
  applyBuildPlate()
}

/* ── Feature 4: Model Statistics ── */
const showStatistics = ref(false)
const statsVolume = ref(0)
const statsSurfaceArea = ref(0)

function computeStatistics() {
  let vol = 0
  let area = 0
  for (const m of lastParsedMeshes) {
    // Skip transparent / subtracted (difference red) bodies so they don't
    // inflate the numbers.
    if (m.color[3] < 0.99) continue
    const v = m.vertices       // interleaved pos(3) + normal(3)
    const idx = m.indices
    const tf = m.transform     // Mat4 (Float32Array, row-major 16)

    // Apply the mesh transform to a vertex position (index into v / 6).
    const wx = (vi: number) => {
      const x = v[vi * 6], y = v[vi * 6 + 1], z = v[vi * 6 + 2]
      return [
        tf[0] * x + tf[1] * y + tf[2] * z + tf[3],
        tf[4] * x + tf[5] * y + tf[6] * z + tf[7],
        tf[8] * x + tf[9] * y + tf[10] * z + tf[11],
      ] as [number, number, number]
    }

    let meshVol = 0
    for (let i = 0; i < idx.length; i += 3) {
      const v0 = wx(idx[i]), v1 = wx(idx[i + 1]), v2 = wx(idx[i + 2])
      // Signed tetrahedron volume: dot(v0, cross(v1, v2)) / 6
      const cx = v1[1] * v2[2] - v1[2] * v2[1]
      const cy = v1[2] * v2[0] - v1[0] * v2[2]
      const cz = v1[0] * v2[1] - v1[1] * v2[0]
      meshVol += (v0[0] * cx + v0[1] * cy + v0[2] * cz) / 6

      // Triangle area: 0.5 * |cross(v1-v0, v2-v0)|
      const e1x = v1[0] - v0[0], e1y = v1[1] - v0[1], e1z = v1[2] - v0[2]
      const e2x = v2[0] - v0[0], e2y = v2[1] - v0[1], e2z = v2[2] - v0[2]
      const ax = e1y * e2z - e1z * e2y
      const ay = e1z * e2x - e1x * e2z
      const az = e1x * e2y - e1y * e2x
      area += 0.5 * Math.sqrt(ax * ax + ay * ay + az * az)
    }
    vol += Math.abs(meshVol)
  }
  statsVolume.value = vol
  statsSurfaceArea.value = area
}

function formatNumber(n: number, decimals = 1): string {
  return n.toLocaleString(lang.value === 'ru' ? 'ru-RU' : 'en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })
}

function fmtInt(n: number): string {
  return n.toLocaleString(lang.value === 'ru' ? 'ru-RU' : 'en-US', {
    maximumFractionDigits: 0,
  })
}

function toggleStatistics() {
  showStatistics.value = !showStatistics.value
  if (showStatistics.value) computeStatistics()
}

// Recompute stats when geometry changes (only while the panel is open).
watch([meshCount, triCount], () => {
  if (showStatistics.value) computeStatistics()
})

/* ── Feature: Weight & Cost Estimator ── */
interface MaterialDef { id: string; nameKey: string; density: number }
const MATERIALS: MaterialDef[] = [
  { id: 'pla', nameKey: 'materialPLA', density: 1.24 },
  { id: 'abs', nameKey: 'materialABS', density: 1.04 },
  { id: 'petg', nameKey: 'materialPETG', density: 1.27 },
  { id: 'nylon', nameKey: 'materialNylon', density: 1.14 },
  { id: 'resin', nameKey: 'materialResin', density: 1.18 },
]
const selectedMaterial = ref(localStorage.getItem('scad-material') || 'pla')
const materialCostPerKg = ref(parseFloat(localStorage.getItem('scad-cost-per-kg') || '25'))
const selectedCurrency = ref(localStorage.getItem('scad-currency') || 'USD')
const CURRENCIES = ['USD', 'EUR', 'RUB'] as const

const estimatedWeight = computed(() => {
  const mat = MATERIALS.find(m => m.id === selectedMaterial.value)
  if (!mat) return 0
  // volume is in mm³, density in g/cm³; 1 cm³ = 1000 mm³
  return statsVolume.value * mat.density / 1000
})

const estimatedCost = computed(() => {
  // weight in grams, cost per kg
  return estimatedWeight.value * materialCostPerKg.value / 1000
})

const currencySymbol = computed(() => {
  switch (selectedCurrency.value) {
    case 'EUR': return '€'
    case 'RUB': return '₽'
    default: return '$'
  }
})

watch(selectedMaterial, v => localStorage.setItem('scad-material', v))
watch(materialCostPerKg, v => localStorage.setItem('scad-cost-per-kg', String(v)))
watch(selectedCurrency, v => localStorage.setItem('scad-currency', v))

/* ── Feature: About Dialog ── */
const showAbout = ref(false)

/* ── Feature: Screenshot Comparison ── */
const referenceScreenshot = ref<string | null>(null)
const showScreenshotCompare = ref(false)
const screenshotCompareMode = ref<'overlay' | 'sideBySide' | 'difference'>('sideBySide')
const currentScreenshot = ref<string | null>(null)
const comparisonSlider = ref(50)

function saveReferenceScreenshot() {
  if (!renderer) return
  const canvas = canvasRef.value
  if (!canvas) return
  referenceScreenshot.value = canvas.toDataURL('image/png')
  addToast(t('saveReference'), 'success')
}

function compareWithReference() {
  if (!referenceScreenshot.value) {
    addToast(t('referenceNotSaved'), 'error')
    return
  }
  const canvas = canvasRef.value
  if (!canvas) return
  currentScreenshot.value = canvas.toDataURL('image/png')
  showScreenshotCompare.value = true
}

/* ── OpenSCAD Reference Panel ── */
const showScadReference = ref(false)

interface RefEntry { name: string; sig: string; desc: { ru: string; en: string } }

const SCAD_REF_SECTIONS: { key: string; entries: RefEntry[] }[] = [
  { key: 'refPrimitives3D', entries: [
    { name: 'cube', sig: 'cube(size, center)', desc: { ru: 'Прямоугольный параллелепипед', en: 'Rectangular box' } },
    { name: 'sphere', sig: 'sphere(r|d, $fn)', desc: { ru: 'Сфера', en: 'Sphere' } },
    { name: 'cylinder', sig: 'cylinder(h, r|r1/r2, center, $fn)', desc: { ru: 'Цилиндр или конус', en: 'Cylinder or cone' } },
    { name: 'polyhedron', sig: 'polyhedron(points, faces)', desc: { ru: 'Произвольный многогранник', en: 'Custom polyhedron from points and faces' } },
    { name: 'surface', sig: 'surface(data, center)', desc: { ru: 'Поверхность из 2D-массива высот', en: 'Surface from 2D height array' } },
    { name: 'text', sig: 'text(text, size, spacing)', desc: { ru: 'Объёмный текст', en: '3D text' } },
    { name: 'star', sig: 'star(points, r1, r2, h, $fn)', desc: { ru: 'Звезда (экструдированная)', en: 'Extruded star shape' } },
    { name: 'thread', sig: 'thread(d, pitch, length, $fn)', desc: { ru: 'Резьбовой цилиндр', en: 'Threaded cylinder' } },
    { name: 'prism', sig: 'prism(sides, r, h, center)', desc: { ru: 'Правильная призма', en: 'Regular N-sided prism' } },
    { name: 'cone', sig: 'cone(r, h, center, $fn)', desc: { ru: 'Конус', en: 'Cone' } },
    { name: 'capsule', sig: 'capsule(r, h, center, $fn)', desc: { ru: 'Капсула', en: 'Capsule with hemisphere caps' } },
    { name: 'gear', sig: 'gear(teeth, mod, thickness, $fn)', desc: { ru: 'Шестерня', en: 'Simplified spur gear' } },
    { name: 'ogive', sig: 'ogive(r, h, $fn)', desc: { ru: 'Оживальная форма (носовой обтекатель)', en: 'Ogive / nose cone shape' } },
    { name: 'teardrop', sig: 'teardrop(r, h, $fn)', desc: { ru: 'Каплевидная форма (для 3D-печати)', en: 'Teardrop shape (3D print friendly)' } },
    { name: 'ring', sig: 'ring(r1, r2, h, $fn)', desc: { ru: 'Кольцо (полый цилиндр)', en: 'Ring (hollow cylinder)' } },
    { name: 'tube', sig: 'tube(r, wall, h, $fn)', desc: { ru: 'Труба (с толщиной стенки)', en: 'Tube (with wall thickness)' } },
  ]},
  { key: 'refPrimitives2D', entries: [
    { name: 'circle', sig: 'circle(r|d, $fn)', desc: { ru: 'Окружность', en: 'Circle' } },
    { name: 'square', sig: 'square(size, center)', desc: { ru: 'Прямоугольник', en: 'Rectangle' } },
    { name: 'polygon', sig: 'polygon(points, paths)', desc: { ru: '2D-многоугольник', en: '2D polygon' } },
  ]},
  { key: 'refTransforms', entries: [
    { name: 'translate', sig: 'translate([x, y, z])', desc: { ru: 'Перемещение', en: 'Move children' } },
    { name: 'rotate', sig: 'rotate([x,y,z]) | rotate(a, v)', desc: { ru: 'Поворот', en: 'Rotate children' } },
    { name: 'scale', sig: 'scale([x, y, z])', desc: { ru: 'Масштабирование', en: 'Scale children' } },
    { name: 'mirror', sig: 'mirror([x, y, z])', desc: { ru: 'Зеркальное отражение', en: 'Mirror children' } },
    { name: 'resize', sig: 'resize(newsize, auto)', desc: { ru: 'Изменить размер до заданного', en: 'Resize children to target size' } },
    { name: 'multmatrix', sig: 'multmatrix(m)', desc: { ru: 'Произвольная матрица 4x4', en: 'Apply arbitrary 4x4 matrix' } },
    { name: 'color', sig: 'color(c, alpha)', desc: { ru: 'Задать цвет', en: 'Set color of children' } },
    { name: 'offset', sig: 'offset(r|delta, chamfer)', desc: { ru: 'Смещение 2D-контура', en: 'Offset 2D outline' } },
    { name: 'radial_array', sig: 'radial_array(count, r) { ... }', desc: { ru: 'Круговой массив', en: 'Radial array around circle' } },
    { name: 'linear_array', sig: 'linear_array(count, spacing) { ... }', desc: { ru: 'Линейный массив', en: 'Linear array along direction' } },
    { name: 'mirror_copy', sig: 'mirror_copy(v) { ... }', desc: { ru: 'Зеркальная копия (оригинал + отражение)', en: 'Mirror copy (original + reflection)' } },
    { name: 'distribute', sig: 'distribute(count, spacing) { ... }', desc: { ru: 'Распределение копий (линейный массив)', en: 'Distribute copies (linear array alias)' } },
  ]},
  { key: 'refCSG', entries: [
    { name: 'union', sig: 'union() { ... }', desc: { ru: 'Объединение', en: 'Combine children' } },
    { name: 'difference', sig: 'difference() { ... }', desc: { ru: 'Вычитание: первый минус остальные', en: 'Subtract subsequent from first' } },
    { name: 'intersection', sig: 'intersection() { ... }', desc: { ru: 'Пересечение', en: 'Keep only overlap' } },
    { name: 'hull', sig: 'hull() { ... }', desc: { ru: 'Выпуклая оболочка', en: 'Convex hull of children' } },
    { name: 'minkowski', sig: 'minkowski() { ... }', desc: { ru: 'Сумма Минковского', en: 'Minkowski sum' } },
  ]},
  { key: 'refExtrusions', entries: [
    { name: 'linear_extrude', sig: 'linear_extrude(height, twist, slices, $fn)', desc: { ru: 'Линейная экструзия 2D-формы', en: 'Extrude 2D shape along Z' } },
    { name: 'rotate_extrude', sig: 'rotate_extrude(angle, $fn)', desc: { ru: 'Вращательная экструзия', en: 'Revolve 2D shape around Z' } },
    { name: 'projection', sig: 'projection(cut)', desc: { ru: 'Проекция 3D на плоскость XY', en: 'Project 3D onto XY plane' } },
  ]},
  { key: 'refMathFunctions', entries: [
    { name: 'sin/cos/tan', sig: 'sin(deg), cos(deg), tan(deg)', desc: { ru: 'Тригонометрия (градусы)', en: 'Trigonometry (degrees)' } },
    { name: 'asin/acos/atan', sig: 'asin(x), acos(x), atan(x), atan2(y,x)', desc: { ru: 'Обратная тригонометрия', en: 'Inverse trig (returns degrees)' } },
    { name: 'sqrt/pow/exp/ln', sig: 'sqrt(x), pow(b,e), exp(x), ln(x)', desc: { ru: 'Корень, степень, экспонента, логарифм', en: 'Root, power, exponential, log' } },
    { name: 'abs/sign', sig: 'abs(x), sign(x)', desc: { ru: 'Модуль и знак', en: 'Absolute value and sign' } },
    { name: 'min/max', sig: 'min(a,b,...), max(a,b,...)', desc: { ru: 'Минимум/максимум', en: 'Minimum / maximum' } },
    { name: 'floor/ceil/round', sig: 'floor(x), ceil(x), round(x)', desc: { ru: 'Округление', en: 'Rounding functions' } },
    { name: 'len', sig: 'len(array|string)', desc: { ru: 'Длина массива или строки', en: 'Length of array or string' } },
    { name: 'norm/cross', sig: 'norm(v), cross(v1, v2)', desc: { ru: 'Норма вектора, векторное произведение', en: 'Vector magnitude, cross product' } },
    { name: 'rands', sig: 'rands(min, max, count, seed)', desc: { ru: 'Массив случайных чисел', en: 'Array of random numbers' } },
    { name: 'lookup', sig: 'lookup(key, table)', desc: { ru: 'Интерполяция по таблице', en: 'Table-based interpolation' } },
    { name: 'str/chr/ord', sig: 'str(...), chr(code), ord(char)', desc: { ru: 'Строковые функции', en: 'String functions' } },
    { name: 'concat', sig: 'concat(a, b, ...)', desc: { ru: 'Объединение массивов/строк', en: 'Concatenate arrays or strings' } },
  ]},
  { key: 'refSpecialVars', entries: [
    { name: '$fn', sig: '$fn = N', desc: { ru: 'Количество сегментов для скруглений', en: 'Number of segments for curves' } },
    { name: '$fa', sig: '$fa = angle', desc: { ru: 'Минимальный угол сегмента', en: 'Minimum angle per segment' } },
    { name: '$fs', sig: '$fs = size', desc: { ru: 'Минимальная длина сегмента', en: 'Minimum segment size' } },
    { name: '$t', sig: '$t (0..1)', desc: { ru: 'Параметр анимации', en: 'Animation parameter' } },
  ]},
  { key: 'refControlFlow', entries: [
    { name: 'for', sig: 'for(i = [start:step:end]) { ... }', desc: { ru: 'Цикл', en: 'Loop' } },
    { name: 'if/else', sig: 'if (cond) { ... } else { ... }', desc: { ru: 'Условие', en: 'Conditional' } },
    { name: 'let', sig: 'let(var = val) { ... }', desc: { ru: 'Локальная переменная', en: 'Local variable binding' } },
    { name: 'module', sig: 'module name(params) { ... }', desc: { ru: 'Определение модуля', en: 'Module definition' } },
    { name: 'children()', sig: 'children()', desc: { ru: 'Дочерние объекты модуля', en: 'Module child objects' } },
    { name: 'echo', sig: 'echo(val, ...)', desc: { ru: 'Вывод в консоль', en: 'Print to console' } },
    { name: 'assert', sig: 'assert(cond, msg)', desc: { ru: 'Проверка условия', en: 'Assertion check' } },
  ]},
  { key: 'refOther', entries: [
    { name: 'use', sig: 'use <file>', desc: { ru: 'Импорт модулей из файла', en: 'Import modules from file' } },
    { name: 'include', sig: 'include <file>', desc: { ru: 'Включение файла целиком', en: 'Include entire file' } },
    { name: 'import', sig: 'import("file.stl")', desc: { ru: 'Импорт 3D-модели (не поддерживается в веб)', en: 'Import 3D model (not supported in web)' } },
  ]},
]

function toggleScadReference() {
  showScadReference.value = !showScadReference.value
}

const refSearchQuery = ref('')

const filteredRefSections = computed(() => {
  const q = refSearchQuery.value.trim().toLowerCase()
  if (!q) return SCAD_REF_SECTIONS
  return SCAD_REF_SECTIONS
    .map(section => ({
      ...section,
      entries: section.entries.filter(e =>
        e.name.toLowerCase().includes(q) ||
        e.sig.toLowerCase().includes(q) ||
        (e.desc as any)[lang.value]?.toLowerCase().includes(q) ||
        e.desc.en.toLowerCase().includes(q)
      ),
    }))
    .filter(section => section.entries.length > 0)
})

function highlightMatch(text: string): string {
  const q = refSearchQuery.value.trim()
  if (!q) return text
  const escaped = q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const regex = new RegExp(`(${escaped})`, 'gi')
  return text.replace(regex, '<mark class="ref-highlight">$1</mark>')
}

/* ── Feature 5: Custom-Resolution PNG Export ── */
const pngScale = ref(parseInt(localStorage.getItem('scad-png-scale') || '1') || 1)

function exportPng(scale: number) {
  if (!renderer) return
  pngScale.value = scale
  localStorage.setItem('scad-png-scale', String(scale))
  const meta = screenshotMeta.value
  const anyMeta = meta.modelName || meta.dimensions || meta.triangles || meta.date
  if (anyMeta) {
    screenshotWithMeta(scale)
  } else {
    if (scale <= 1) {
      renderer.screenshot()
    } else {
      renderer.screenshotScaled(scale)
    }
  }
  addToast(scale <= 1 ? t('screenshot') : (t('exportPng') + ' ' + scale + 'x'), 'success')
}

/* ── Viewport Dropdown Menus ── */
const viewMenuOpen = ref(false)
const renderMenuOpen = ref(false)
const exportMenuOpen = ref(false)

function closeAllMenus() {
  viewMenuOpen.value = false
  renderMenuOpen.value = false
  exportMenuOpen.value = false
}

function toggleViewMenu() {
  const was = viewMenuOpen.value
  closeAllMenus()
  viewMenuOpen.value = !was
  if (!was) nextTick(() => focusFirstMenuItem('view'))
}
function toggleRenderMenu() {
  const was = renderMenuOpen.value
  closeAllMenus()
  renderMenuOpen.value = !was
  if (!was) nextTick(() => focusFirstMenuItem('render'))
}
function toggleExportMenu() {
  const was = exportMenuOpen.value
  closeAllMenus()
  exportMenuOpen.value = !was
  if (!was) nextTick(() => focusFirstMenuItem('export'))
}

function focusFirstMenuItem(_menuId: string) {
  const dropdown = document.querySelector('.vp-dropdown[role="menu"]') as HTMLElement | null
  if (!dropdown) return
  const first = dropdown.querySelector('[role="menuitem"]') as HTMLElement | null
  if (first) first.focus()
}

function onMenuKeydown(e: KeyboardEvent, _menuId: string) {
  const dropdown = (e.currentTarget as HTMLElement)
  const items = Array.from(dropdown.querySelectorAll('[role="menuitem"]:not([style*="display: none"]):not([hidden])')) as HTMLElement[]
  if (!items.length) return

  const currentIndex = items.indexOf(document.activeElement as HTMLElement)

  switch (e.key) {
    case 'ArrowDown': {
      e.preventDefault()
      const next = currentIndex < items.length - 1 ? currentIndex + 1 : 0
      items[next].focus()
      break
    }
    case 'ArrowUp': {
      e.preventDefault()
      const prev = currentIndex > 0 ? currentIndex - 1 : items.length - 1
      items[prev].focus()
      break
    }
    case 'Home': {
      e.preventDefault()
      items[0].focus()
      break
    }
    case 'End': {
      e.preventDefault()
      items[items.length - 1].focus()
      break
    }
    case 'Enter':
    case ' ': {
      e.preventDefault()
      if (document.activeElement && items.includes(document.activeElement as HTMLElement)) {
        (document.activeElement as HTMLElement).click()
      }
      break
    }
    case 'Escape': {
      e.preventDefault()
      closeAllMenus()
      break
    }
    default: {
      // Type-ahead: jump to first item starting with typed letter
      if (e.key.length === 1) {
        const letter = e.key.toLowerCase()
        const match = items.find((item, idx) => {
          if (idx <= currentIndex) return false
          const text = item.textContent?.trim().toLowerCase() || ''
          return text.startsWith(letter)
        }) || items.find(item => {
          const text = item.textContent?.trim().toLowerCase() || ''
          return text.startsWith(letter)
        })
        if (match) match.focus()
      }
      break
    }
  }
}

/* ── Camera Bookmarks ── */
interface CameraBookmark {
  id: string
  name: string
  yaw: number
  pitch: number
  dist: number
  tx: number
  ty: number
  tz: number
  thumbnail?: string
}

function loadBookmarks(): CameraBookmark[] {
  const parsed = safeParse<CameraBookmark[]>(localStorage.getItem('scad-bookmarks'), [])
  return Array.isArray(parsed) ? parsed : []
}

const cameraBookmarks = ref<CameraBookmark[]>(loadBookmarks())

function saveBookmarksToStorage() {
  localStorage.setItem('scad-bookmarks', JSON.stringify(cameraBookmarks.value))
}

function saveCameraBookmark() {
  if (!renderer) return
  const thumb = captureBookmarkThumbnail()
  const bk: CameraBookmark = {
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 4),
    name: `Cam ${cameraBookmarks.value.length + 1}`,
    yaw: renderer.yaw,
    pitch: renderer.pitch,
    dist: renderer.dist,
    tx: renderer.tx,
    ty: renderer.ty,
    tz: renderer.tz,
    thumbnail: thumb,
  }
  cameraBookmarks.value.push(bk)
  if (cameraBookmarks.value.length > 10) {
    cameraBookmarks.value = cameraBookmarks.value.slice(-10)
  }
  saveBookmarksToStorage()
  addToast(t('bookmarkSaved'), 'success')
}

function restoreBookmark(bk: CameraBookmark) {
  if (!renderer) return
  renderer.yaw = bk.yaw
  renderer.pitch = bk.pitch
  renderer.dist = bk.dist
  renderer.tx = bk.tx
  renderer.ty = bk.ty
  renderer.tz = bk.tz
}

function deleteBookmark(id: string) {
  cameraBookmarks.value = cameraBookmarks.value.filter(b => b.id !== id)
  saveBookmarksToStorage()
}

/* ── STL Import ── */
function openImportSTL() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.stl'
  input.onchange = () => {
    const file = input.files?.[0]
    if (!file) return
    importSTLFile(file)
  }
  input.click()
}

function importSTLFile(file: File) {
  const reader = new FileReader()
  reader.onload = () => {
    try {
      const buffer = reader.result as ArrayBuffer
      const meshes = parseSTL(buffer)
      if (!renderer) return
      lastParsedMeshes = meshes
      meshCount.value = meshes.length
      triCount.value = meshes.reduce((s: number, m: MeshData) => s + m.indices.length / 3, 0)
      renderer.setMeshes(meshes)
      renderer.autoFitAll()
      addToast(t('importedStl') + ': ' + file.name, 'success')
      addConsoleEntry('info', t('importedStl') + ': ' + file.name + ' (' + triCount.value + ' triangles)')
    } catch (e: any) {
      addToast(e.message || String(e), 'error')
      addConsoleEntry('error', e.message || String(e))
    }
  }
  reader.readAsArrayBuffer(file)
}

/* ── Canvas drag-and-drop for STL ── */
const isCanvasDragOver = ref(false)
let canvasDragCounter = 0

function onCanvasDragEnter(e: DragEvent) {
  e.preventDefault()
  canvasDragCounter++
  isCanvasDragOver.value = true
}
function onCanvasDragOver(e: DragEvent) {
  e.preventDefault()
}
function onCanvasDragLeave(e: DragEvent) {
  e.preventDefault()
  canvasDragCounter--
  if (canvasDragCounter <= 0) { canvasDragCounter = 0; isCanvasDragOver.value = false }
}
function onCanvasDrop(e: DragEvent) {
  e.preventDefault()
  canvasDragCounter = 0
  isCanvasDragOver.value = false
  const file = e.dataTransfer?.files?.[0]
  if (!file) return
  if (file.name.toLowerCase().endsWith('.stl')) {
    importSTLFile(file)
  } else if (file.name.toLowerCase().endsWith('.scad')) {
    const reader = new FileReader()
    reader.onload = () => {
      const content = reader.result as string
      code.value = content
      addToRecent(file.name.replace(/\.scad$/, ''), content)
    }
    reader.readAsText(file)
  }
}

/* ── Feature: Example Gallery Modal ── */
const showExampleGallery = ref(false)
const exampleGallerySearch = ref('')

interface ExampleCard {
  key: string
  nameKey: string
  tipKey: string
}

const EXAMPLE_CARDS: ExampleCard[] = [
  { key: 'basic', nameKey: 'basic', tipKey: 'basicTip' },
  { key: 'csg', nameKey: 'csg', tipKey: 'csgTip' },
  { key: 'house', nameKey: 'house', tipKey: 'houseTip' },
  { key: 'tower', nameKey: 'tower', tipKey: 'towerTip' },
  { key: 'paramGear', nameKey: 'paramGear', tipKey: 'paramGearTip' },
  { key: 'staircase', nameKey: 'staircase', tipKey: 'staircaseTip' },
  { key: 'paramVase', nameKey: 'paramVase', tipKey: 'paramVaseTip' },
  { key: 'gear', nameKey: 'gear', tipKey: 'gearTip' },
  { key: 'vase', nameKey: 'vase', tipKey: 'vaseTip' },
  { key: 'chess', nameKey: 'chess', tipKey: 'chessTip' },
  { key: 'mechanical', nameKey: 'mechanical', tipKey: 'mechanicalTip' },
  { key: 'honeycomb', nameKey: 'honeycomb', tipKey: 'honeycombTip' },
  { key: 'springCoil', nameKey: 'springCoil', tipKey: 'springCoilTip' },
  { key: 'knurledCylinder', nameKey: 'knurledCylinder', tipKey: 'knurledCylinderTip' },
  { key: 'chamferedBox', nameKey: 'chamferedBox', tipKey: 'chamferedBoxTip' },
  { key: 'loftShape', nameKey: 'loftShape', tipKey: 'loftShapeTip' },
  { key: 'lattice', nameKey: 'latticeEx', tipKey: 'latticeTip' },
  { key: 'slotCross', nameKey: 'slotCrossEx', tipKey: 'slotCrossTip' },
  { key: 'maze', nameKey: 'mazeEx', tipKey: 'mazeTip' },
  { key: 'fibSphere', nameKey: 'fibSphereEx', tipKey: 'fibSphereTip' },
]

function getExamplePreview(key: string): string {
  const ex = EXAMPLES[key]
  if (!ex) return ''
  return ex.split('\n').slice(0, 3).join('\n')
}

const filteredExamples = computed(() => {
  const q = exampleGallerySearch.value.toLowerCase().trim()
  if (!q) return EXAMPLE_CARDS
  return EXAMPLE_CARDS.filter(card => {
    const name = t(card.nameKey).toLowerCase()
    const tip = t(card.tipKey).toLowerCase()
    return name.includes(q) || tip.includes(q) || card.key.toLowerCase().includes(q)
  })
})

function openExampleGallery() {
  showExampleGallery.value = true
  exampleGallerySearch.value = ''
}

function closeExampleGallery() {
  showExampleGallery.value = false
  exampleGallerySearch.value = ''
}

function loadExampleFromGallery(key: string) {
  loadExample(key)
  closeExampleGallery()
}

/* ── Feature: Template Snippets ── */
const showSnippetPanel = ref(false)

interface Snippet {
  id: string
  nameKey: string
  code: string
}

const SNIPPETS: Snippet[] = [
  { id: 'centeredCube', nameKey: 'snippetCenteredCube', code: 'cube([10,10,10], center=true);' },
  { id: 'cylinderHole', nameKey: 'snippetCylinderHole', code: 'difference() {\n    cylinder(h=10, r=15, $fn=64);\n    cylinder(h=12, r=10, $fn=64, center=true);\n}' },
  { id: 'roundedBox', nameKey: 'snippetRoundedBox', code: 'minkowski() {\n    cube([20,20,10]);\n    sphere(r=2, $fn=16);\n}' },
  { id: 'arrayPattern', nameKey: 'snippetArrayPattern', code: 'for(i=[0:5])\n    translate([i*12, 0, 0])\n        cube(10);' },
  { id: 'parametricModule', nameKey: 'snippetParametricModule', code: 'module box(w=10, h=5, d=3) {\n    cube([w,h,d], center=true);\n}' },
  { id: 'threadedInsert', nameKey: 'snippetThreadedInsert', code: 'difference() {\n    cylinder(h=10, r=8, $fn=32);\n    translate([0,0,-1])\n        cylinder(h=12, r=5, $fn=32);\n}' },
  { id: 'hexGrid', nameKey: 'snippetHexGrid', code: 'module hex(r=5, h=3) {\n    cylinder(r=r, h=h, $fn=6);\n}\nfor (row=[0:3])\n    for (col=[0:3])\n        translate([col*11 + (row%2)*5.5, row*9.5, 0])\n            hex();' },
  { id: 'bevel', nameKey: 'snippetBevel', code: 'difference() {\n    cube([20,20,10]);\n    translate([20,0,-0.1])\n        rotate([0,0,45])\n            cube([4,4,10.2]);\n}' },
  { id: 'spring', nameKey: 'snippetSpring', code: 'module spring(r=10, wire=2, turns=5, pitch=5) {\n    for (i=[0:turns*$fn-1]) {\n        a1 = i * 360 / $fn;\n        a2 = (i+1) * 360 / $fn;\n        hull() {\n            translate([r*cos(a1), r*sin(a1), i*pitch/$fn])\n                sphere(wire/2, $fn=8);\n            translate([r*cos(a2), r*sin(a2), (i+1)*pitch/$fn])\n                sphere(wire/2, $fn=8);\n        }\n    }\n}\nspring($fn=24);' },
  { id: 'boxWithLid', nameKey: 'snippetBoxWithLid', code: '// Box with Lid\nw=30; d=20; h=15; wall=2;\n// Box\ndifference() {\n    cube([w, d, h]);\n    translate([wall, wall, wall])\n        cube([w-2*wall, d-2*wall, h]);\n}\n// Lid\ntranslate([0, 0, h+2])\n    cube([w, d, wall]);' },
  { id: 'gearWheel', nameKey: 'snippetGearWheel', code: '// Simple Gear Wheel\nteeth=12; r=20; th=5;\nunion() {\n    cylinder(r=r-3, h=th, $fn=48);\n    for (i=[0:teeth-1])\n        rotate([0,0,i*360/teeth])\n            translate([r-2,0,0])\n                cylinder(r=2.5, h=th, $fn=6);\n}' },
]

function insertSnippet(snippet: Snippet) {
  const el = textareaRef.value
  if (!el) return
  pushUndoSnapshot()
  const pos = el.selectionStart
  const before = code.value.substring(0, pos)
  const after = code.value.substring(el.selectionEnd)
  code.value = before + snippet.code + after
  showSnippetPanel.value = false
  nextTick(() => {
    const newPos = pos + snippet.code.length
    el.selectionStart = el.selectionEnd = newPos
    el.focus()
  })
}

/* ── Feature: Code Statistics ── */
interface CodeStats {
  lines: number
  chars: number
  words: number
  modules: number
  forLoops: number
  nestingDepth: number
}

const codeStatsData = ref<CodeStats>({ lines: 0, chars: 0, words: 0, modules: 0, forLoops: 0, nestingDepth: 0 })
let codeStatsDebounce: ReturnType<typeof setTimeout> | null = null

function computeCodeStats(src: string): CodeStats {
  const lines = src.split('\n').length
  const chars = src.length
  const words = src.split(/\s+/).filter(w => w.length > 0).length
  const modules = (src.match(/\bmodule\s+\w+/g) || []).length
  const forLoops = (src.match(/\bfor\s*\(/g) || []).length
  let maxDepth = 0
  let depth = 0
  for (let i = 0; i < src.length; i++) {
    if (src[i] === '{') { depth++; if (depth > maxDepth) maxDepth = depth }
    else if (src[i] === '}') { depth = Math.max(0, depth - 1) }
  }
  return { lines, chars, words, modules, forLoops, nestingDepth: maxDepth }
}

watch(code, (v) => {
  if (codeStatsDebounce) clearTimeout(codeStatsDebounce)
  codeStatsDebounce = setTimeout(() => {
    codeStatsData.value = computeCodeStats(v)
  }, 300)
}, { immediate: true })

/* ── Feature: Code Complexity Indicator ── */
const codeComplexity = computed(() => {
  const src = code.value
  const lines = src.split('\n').length
  const modules = (src.match(/\bmodule\s+\w+/g) || []).length
  const forLoops = (src.match(/\bfor\s*\(/g) || []).length
  const csgOps = (src.match(/\b(union|difference|intersection|hull|minkowski)\s*\(/g) || []).length

  // Nesting depth
  let maxDepth = 0, curDepth = 0
  for (const ch of src) {
    if (ch === '{') { curDepth++; if (curDepth > maxDepth) maxDepth = curDepth }
    else if (ch === '}') curDepth--
  }

  // Score: weighted combination
  const score = lines + modules * 10 + forLoops * 8 + csgOps * 5 + maxDepth * 4

  let level: 'simple' | 'medium' | 'complex'
  let color: string
  if (lines < 50 && score < 80) { level = 'simple'; color = '#4caf50' }
  else if (lines < 200 && score < 300) { level = 'medium'; color = '#ff9800' }
  else { level = 'complex'; color = '#f44336' }

  const labelKey = level === 'simple' ? 'complexitySimple' : level === 'medium' ? 'complexityMedium' : 'complexityComplex'

  return { level, color, lines, modules, forLoops, csgOps, maxDepth, label: labelKey }
})

/* ── Feature: Viewport Camera Info ── */
const showCameraInfo = ref(false)
const cameraInfoData = ref({ yaw: 0, pitch: 0, dist: 0, tx: 0, ty: 0, tz: 0 })
let cameraInfoInterval: ReturnType<typeof setInterval> | null = null

function toggleCameraInfo() {
  showCameraInfo.value = !showCameraInfo.value
  if (showCameraInfo.value && !cameraInfoInterval) {
    cameraInfoInterval = setInterval(() => {
      if (!renderer) return
      cameraInfoData.value = {
        yaw: (renderer.yaw * 180 / Math.PI),
        pitch: (renderer.pitch * 180 / Math.PI),
        dist: renderer.dist,
        tx: renderer.tx,
        ty: renderer.ty,
        tz: renderer.tz,
      }
    }, 100)
  } else if (!showCameraInfo.value && cameraInfoInterval) {
    clearInterval(cameraInfoInterval)
    cameraInfoInterval = null
  }
}

// Clean up camera info interval
onUnmounted(() => {
  if (cameraInfoInterval) clearInterval(cameraInfoInterval)
  if (codeStatsDebounce) clearTimeout(codeStatsDebounce)
})

/* ── Feature: Viewport Compass ── */
const compassYaw = ref(0)
let compassAnimFrame = 0
function updateCompass() {
  if (renderer) compassYaw.value = renderer.yaw * 180 / Math.PI
  compassAnimFrame = requestAnimationFrame(updateCompass)
}
onMounted(() => { compassAnimFrame = requestAnimationFrame(updateCompass) })
onUnmounted(() => { cancelAnimationFrame(compassAnimFrame) })

/* ── Feature: Export All Formats Dropdown (toolbar) ── */
const showExportDropdown = ref(false)

function toggleExportDropdown() {
  showExportDropdown.value = !showExportDropdown.value
}

function closeExportDropdown() {
  showExportDropdown.value = false
}

/* ── Feature: 3D Annotations ── */
const parsedAnnotations = ref<Annotation[]>([])

interface AnnotationScreenPos {
  text: string
  x: number
  y: number
  visible: boolean
}

const annotationScreenPositions = ref<AnnotationScreenPos[]>([])
let annotationRAF = 0

function updateAnnotations() {
  if (!renderer || !parsedAnnotations.value.length) {
    annotationScreenPositions.value = []
    annotationRAF = requestAnimationFrame(updateAnnotations)
    return
  }
  const positions: AnnotationScreenPos[] = []
  for (const ann of parsedAnnotations.value) {
    const sp = renderer.getScreenPosition(ann.position[0], ann.position[1], ann.position[2])
    positions.push({
      text: ann.text,
      x: sp.x,
      y: sp.y,
      visible: !sp.behind,
    })
  }
  annotationScreenPositions.value = positions
  annotationRAF = requestAnimationFrame(updateAnnotations)
}

/* ── Feature: Bookmark Thumbnails ── */
function captureBookmarkThumbnail(): string {
  if (!renderer || !canvasRef.value) return ''
  try {
    renderer.requestRender()
    const srcCanvas = canvasRef.value
    const tmpCanvas = document.createElement('canvas')
    tmpCanvas.width = 80
    tmpCanvas.height = 60
    const ctx = tmpCanvas.getContext('2d')
    if (!ctx) return ''
    ctx.drawImage(srcCanvas, 0, 0, srcCanvas.width, srcCanvas.height, 0, 0, 80, 60)
    return tmpCanvas.toDataURL('image/png', 0.6)
  } catch { return '' }
}

/* ── Feature: Viewport Watermark ── */
const wmEnabled = ref(localStorage.getItem('scad-wm-enabled') === 'true')
const wmText = ref(localStorage.getItem('scad-wm-text') || '')
const wmPosition = ref(localStorage.getItem('scad-wm-position') || 'bottom-right')
const wmOpacity = ref(parseFloat(localStorage.getItem('scad-wm-opacity') || '0.15'))

watch(wmEnabled, v => localStorage.setItem('scad-wm-enabled', String(v)))
watch(wmText, v => localStorage.setItem('scad-wm-text', v))
watch(wmPosition, v => localStorage.setItem('scad-wm-position', v))
watch(wmOpacity, v => localStorage.setItem('scad-wm-opacity', String(v)))

const wmStyle = computed(() => {
  const pos: Record<string, string> = {}
  if (wmPosition.value.includes('top')) pos.top = '10px'
  if (wmPosition.value.includes('bottom')) pos.bottom = '40px'
  if (wmPosition.value.includes('left')) pos.left = '10px'
  if (wmPosition.value.includes('right')) pos.right = '10px'
  return { ...pos, opacity: wmOpacity.value }
})

/* ── Feature: Snapshot Gallery ── */
interface Snapshot {
  id: string
  dataUrl: string
  timestamp: number
}
const SNAPSHOT_MAX = 20
const SNAPSHOT_KEY = 'scad-snapshots'
const snapshots = ref<Snapshot[]>(loadSnapshots())
const showSnapshotGallery = ref(false)
const snapshotPreview = ref<string | null>(null)

function loadSnapshots(): Snapshot[] {
  const parsed = safeParse<Snapshot[]>(localStorage.getItem(SNAPSHOT_KEY), [])
  return Array.isArray(parsed) ? parsed : []
}

function saveSnapshots() {
  try {
    localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(snapshots.value))
  } catch {
    // localStorage full — remove oldest
    if (snapshots.value.length > 1) {
      snapshots.value.shift()
      saveSnapshots()
    }
  }
}

function takeSnapshot() {
  if (!renderer || !canvasRef.value) return
  try {
    renderer.requestRender()
    const srcCanvas = canvasRef.value
    const tmpCanvas = document.createElement('canvas')
    tmpCanvas.width = 160
    tmpCanvas.height = 120
    const ctx = tmpCanvas.getContext('2d')
    if (!ctx) return
    ctx.drawImage(srcCanvas, 0, 0, srcCanvas.width, srcCanvas.height, 0, 0, 160, 120)
    const dataUrl = tmpCanvas.toDataURL('image/jpeg', 0.6)
    const snap: Snapshot = {
      id: Date.now().toString(36) + Math.random().toString(36).slice(2, 4),
      dataUrl,
      timestamp: Date.now(),
    }
    snapshots.value.push(snap)
    if (snapshots.value.length > SNAPSHOT_MAX) snapshots.value.shift()
    saveSnapshots()
    addToast(t('snapshotSaved'), 'success')
  } catch { /* ignore */ }
}

function deleteSnapshot(id: string) {
  snapshots.value = snapshots.value.filter(s => s.id !== id)
  saveSnapshots()
  addToast(t('snapshotDeleted'), 'info')
}

function formatSnapshotTime(ts: number): string {
  return new Date(ts).toLocaleString()
}

/* ── Feature: Code Profiling ── */
const showProfilePanel = ref(false)
const profileEntries = ref<ProfileEntry[]>([])

const sortedProfileEntries = computed(() => {
  return [...profileEntries.value].sort((a, b) => b.timeMs - a.timeMs)
})

function jumpToProfileLine(line: number) {
  const ta = textareaRef.value
  if (!ta) return
  const lines = code.value.split('\n')
  let pos = 0
  for (let i = 0; i < Math.min(line - 1, lines.length); i++) {
    pos += lines[i].length + 1
  }
  ta.focus()
  ta.setSelectionRange(pos, pos)
  // Scroll to line
  const lineHeight = parseFloat(getComputedStyle(ta).lineHeight) || 20
  ta.scrollTop = (line - 5) * lineHeight
}

/* ── Feature: Gutter Decorations ── */
const gutterDecorationsEnabled = ref(localStorage.getItem('scad-gutter-deco') !== 'false')
watch(gutterDecorationsEnabled, v => localStorage.setItem('scad-gutter-deco', String(v)))

function getGutterIcon(lineText: string): string {
  const trimmed = lineText.trim()
  if (/^(module|function)\s+\w+/.test(trimmed)) return '<span class="gutter-icon gutter-icon-module" title="module/function">&#9670;</span>'
  if (/^\bfor\b\s*\(/.test(trimmed)) return '<span class="gutter-icon gutter-icon-loop" title="for loop">&#10227;</span>'
  if (/^\bif\b\s*\(/.test(trimmed)) return '<span class="gutter-icon gutter-icon-cond" title="if">?</span>'
  return ''
}

/* ── Feature: Session Auto-Save / Recovery ── */
const SESSION_KEY = 'scad-session-backup'
const showSessionRestore = ref(false)
let sessionAutoSaveInterval: ReturnType<typeof setInterval> | null = null

interface SessionBackup {
  tabs: EditorTab[]
  activeTabId: string
  timestamp: number
}

function saveSessionBackup() {
  const backup: SessionBackup = {
    tabs: tabs.value,
    activeTabId: activeTabId.value,
    timestamp: Date.now(),
  }
  try {
    localStorage.setItem(SESSION_KEY, JSON.stringify(backup))
  } catch { /* quota exceeded – ignore */ }
}

function checkSessionRestore() {
  const backup = safeParse<SessionBackup | null>(localStorage.getItem(SESSION_KEY), null)
  if (!backup || !Array.isArray(backup.tabs)) return
  // Only show restore if backup is less than 24 hours old and differs from current tabs
  const age = Date.now() - (backup.timestamp || 0)
  if (age > 24 * 60 * 60 * 1000) {
    localStorage.removeItem(SESSION_KEY)
    return
  }
  // Check if backup differs from current state — compare the set of tabs AND
  // their content (id, name, code), not just IDs, so a content-only change
  // (same tabs, edited code) still offers a restore.
  const sig = (tbs: Array<{ id: string; name: string; code: string }>) =>
    tbs.map(tb => `${tb.id} ${tb.name} ${tb.code}`).join('')
  const currentSig = sig(tabs.value)
  const backupSig = sig(backup.tabs)
  if (currentSig === backupSig) return
  showSessionRestore.value = true
}

function restoreSession() {
  const backup = safeParse<SessionBackup | null>(localStorage.getItem(SESSION_KEY), null)
  if (backup && Array.isArray(backup.tabs) && backup.tabs.length > 0) {
    tabs.value = backup.tabs
    activeTabId.value = backup.activeTabId || backup.tabs[0].id
    saveTabs()
    doRender()
  }
  showSessionRestore.value = false
  localStorage.removeItem(SESSION_KEY)
}

function dismissSessionRestore() {
  showSessionRestore.value = false
  localStorage.removeItem(SESSION_KEY)
}

/* ── Feature: Batch Export ── */
function doExportAllTabs() {
  if (tabs.value.length === 0) return
  exportAllTabsAsZip(tabs.value.map(tb => ({ name: tb.name, code: tb.code })))
  addToast(t('exportAllTabs'), 'success')
}

/* -- Feature: Render All Tabs (Batch) -- */
const batchRendering = ref(false)
const batchProgress = ref(0)
const batchTotal = ref(0)
const batchResults = ref<{name: string, tris: number, timeMs: number}[]>([])
const showBatchResults = ref(false)

function doRenderAllTabs() {
  if (batchRendering.value || tabs.value.length === 0) return
  batchRendering.value = true
  batchResults.value = []
  batchTotal.value = tabs.value.length
  batchProgress.value = 0

  const tabsCopy = [...tabs.value]
  let idx = 0

  function renderNext() {
    if (idx >= tabsCopy.length) {
      batchRendering.value = false
      showBatchResults.value = true
      addToast(t('renderAllTabs') + ' (' + tabsCopy.length + ')', 'success')
      return
    }
    const tab = tabsCopy[idx]
    try {
      const t0 = performance.now()
      const result = parseOpenSCADWithAST(tab.code, (name: string) => {
        const baseName = name.replace(/\.scad$/, '')
        const found = tabs.value.find(tb => {
          const tabBase = tb.name.replace(/\.scad$/, '')
          return tabBase === baseName || tabBase === name || tb.name === name
        })
        return found ? found.code : null
      })
      const t1 = performance.now()
      const tris = result.meshes.reduce((s: number, m: any) => s + m.indices.length / 3, 0)
      batchResults.value.push({ name: tab.name, tris, timeMs: Math.round(t1 - t0) })
    } catch (e: any) {
      batchResults.value.push({ name: tab.name, tris: 0, timeMs: 0 })
    }
    idx++
    batchProgress.value = idx
    setTimeout(renderNext, 10)
  }

  setTimeout(renderNext, 10)
}

/* -- Feature: Custom Keyboard Shortcut Editor -- */
const showShortcutEditor = ref(false)

function loadCustomBindings(): Record<string, string> {
  const parsed = safeParse<Record<string, string>>(localStorage.getItem('scad-custom-bindings'), {})
  return (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) ? parsed : {}
}

const customBindings = ref<Record<string, string>>(loadCustomBindings())
const capturingShortcutId = ref<string | null>(null)

const ALL_SHORTCUTS = [
  { id: 'render', label: () => t('sc_render'), defaultKey: 'Ctrl+Enter' },
  { id: 'format', label: () => t('cmdFormatCode'), defaultKey: 'Ctrl+Shift+F' },
  { id: 'fullscreen', label: () => t('fullscreen'), defaultKey: 'F11' },
  { id: 'commandPalette', label: () => t('sc_commandPalette'), defaultKey: 'Ctrl+Shift+P' },
  { id: 'find', label: () => t('sc_findOnly'), defaultKey: 'Ctrl+F' },
  { id: 'findReplace', label: () => t('sc_findReplace'), defaultKey: 'Ctrl+H' },
  { id: 'goToLine', label: () => t('sc_goToLine'), defaultKey: 'Ctrl+G' },
  { id: 'wordWrap', label: () => t('sc_wordWrap'), defaultKey: 'Alt+Z' },
  { id: 'closeTab', label: () => t('sc_closeTab'), defaultKey: 'Ctrl+W' },
  { id: 'nextTab', label: () => t('sc_nextTab'), defaultKey: 'Ctrl+Tab' },
  { id: 'prevTab', label: () => t('sc_prevTab'), defaultKey: 'Ctrl+Shift+Tab' },
]

function saveCustomBindings() {
  localStorage.setItem('scad-custom-bindings', JSON.stringify(customBindings.value))
}

function getEffectiveBinding(id: string): string {
  if (customBindings.value[id]) return customBindings.value[id]
  const sc = ALL_SHORTCUTS.find(s => s.id === id)
  return sc ? sc.defaultKey : ''
}

function startCapture(id: string) {
  capturingShortcutId.value = id
}

function onCaptureKey(e: KeyboardEvent) {
  if (!capturingShortcutId.value) return
  e.preventDefault()
  e.stopPropagation()
  // Ignore lone modifier keys
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return
  const parts: string[] = []
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl')
  if (e.shiftKey) parts.push('Shift')
  if (e.altKey) parts.push('Alt')
  parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key)
  customBindings.value[capturingShortcutId.value] = parts.join('+')
  capturingShortcutId.value = null
  saveCustomBindings()
}

function resetBinding(id: string) {
  delete customBindings.value[id]
  saveCustomBindings()
}

function resetAllBindings() {
  customBindings.value = {}
  saveCustomBindings()
  addToast(t('shortcutResetAll'), 'success')
}

/* -- Feature: Screenshot Metadata -- */
function loadScreenshotMeta(): { modelName: boolean, dimensions: boolean, triangles: boolean, date: boolean } {
  const fallback = { modelName: true, dimensions: true, triangles: true, date: true }
  const parsed = safeParse<typeof fallback>(localStorage.getItem('scad-screenshot-meta'), fallback)
  return (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) ? parsed : fallback
}

const screenshotMeta = ref(loadScreenshotMeta())

watch(screenshotMeta, (v) => {
  localStorage.setItem('scad-screenshot-meta', JSON.stringify(v))
}, { deep: true })

function screenshotWithMeta(scale: number) {
  if (!renderer) return
  const canvas = canvasRef.value
  if (!canvas) {
    // Fallback: no canvas access
    if (scale <= 1) renderer.screenshot()
    else renderer.screenshotScaled(scale)
    return
  }
  const w = canvas.width * scale
  const h = canvas.height * scale
  const offscreen = document.createElement('canvas')
  offscreen.width = w
  offscreen.height = h
  const ctx = offscreen.getContext('2d')
  if (!ctx) {
    if (scale <= 1) renderer.screenshot()
    else renderer.screenshotScaled(scale)
    return
  }
  ctx.drawImage(canvas, 0, 0, w, h)
  // Draw metadata box
  const meta = screenshotMeta.value
  const lines: string[] = []
  if (meta.modelName) lines.push(activeTab.value.name)
  if (meta.dimensions) {
    const bs = boundsSize.value
    lines.push(bs[0].toFixed(1) + ' x ' + bs[1].toFixed(1) + ' x ' + bs[2].toFixed(1))
  }
  if (meta.triangles) lines.push(triCount.value.toLocaleString() + ' triangles')
  if (meta.date) lines.push(new Date().toLocaleDateString())

  if (lines.length > 0) {
    const fontSize = Math.max(12, Math.round(14 * scale))
    ctx.font = fontSize + 'px sans-serif'
    const padding = 8 * scale
    const lineHeight = fontSize + 4 * scale
    const maxWidth = Math.max(...lines.map(l => ctx.measureText(l).width))
    const boxW = maxWidth + padding * 2
    const boxH = lines.length * lineHeight + padding * 2
    const boxX = padding
    const boxY = h - boxH - padding
    ctx.fillStyle = 'rgba(0,0,0,0.55)'
    ctx.beginPath()
    ctx.roundRect(boxX, boxY, boxW, boxH, 6 * scale)
    ctx.fill()
    ctx.fillStyle = '#ffffff'
    ctx.textBaseline = 'top'
    for (let i = 0; i < lines.length; i++) {
      ctx.fillText(lines[i], boxX + padding, boxY + padding + i * lineHeight)
    }
  }

  offscreen.toBlob((blob) => {
    if (!blob) return
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = activeTab.value.name.replace(/\.scad$/, '') + '.png'
    a.click()
    URL.revokeObjectURL(url)
  }, 'image/png')
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

  gear: `// Gear with rectangular teeth
// Outer ring with evenly spaced teeth

color([0.6, 0.6, 0.7])
difference() {
    // Main gear body
    cylinder(h = 6, r = 25, $fn = 48);

    // Center hole
    translate([0, 0, -1])
        cylinder(h = 8, r = 8, $fn = 32);

    // Lightening holes around center
    translate([16, 0, -1])
        cylinder(h = 8, r = 4, $fn = 16);
    translate([-16, 0, -1])
        cylinder(h = 8, r = 4, $fn = 16);
    translate([0, 16, -1])
        cylinder(h = 8, r = 4, $fn = 16);
    translate([0, -16, -1])
        cylinder(h = 8, r = 4, $fn = 16);
}

// Teeth around the perimeter
color([0.65, 0.65, 0.75])
difference() {
    cylinder(h = 6, r = 30, $fn = 24);
    translate([0, 0, -1])
        cylinder(h = 8, r = 25, $fn = 48);
}

// Hub
color([0.5, 0.5, 0.6])
translate([0, 0, 3])
    cylinder(h = 4, r = 10, $fn = 32);

// Axle
color([0.4, 0.4, 0.5])
translate([0, 0, -2])
    cylinder(h = 12, r = 3, $fn = 16);
`,

  vase: `// Decorative vase using stacked rings
// Varying radii create a curved profile

color([0.7, 0.3, 0.35])
difference() {
    // Outer shell - stacked cylinders
    union() {
        // Base
        cylinder(h = 3, r1 = 14, r2 = 12, $fn = 32);
        // Lower body
        translate([0, 0, 3])
            cylinder(h = 10, r1 = 12, r2 = 18, $fn = 32);
        // Belly
        translate([0, 0, 13])
            cylinder(h = 10, r1 = 18, r2 = 16, $fn = 32);
        // Upper body
        translate([0, 0, 23])
            cylinder(h = 10, r1 = 16, r2 = 10, $fn = 32);
        // Neck
        translate([0, 0, 33])
            cylinder(h = 8, r1 = 10, r2 = 12, $fn = 32);
        // Rim
        translate([0, 0, 41])
            cylinder(h = 2, r1 = 12, r2 = 13, $fn = 32);
    }

    // Hollow inside
    translate([0, 0, 3])
        cylinder(h = 42, r1 = 10, r2 = 11, $fn = 32);
}

// Decorative rings
color([0.8, 0.4, 0.3])
translate([0, 0, 13])
    cylinder(h = 1.5, r = 18.5, $fn = 32);

color([0.8, 0.4, 0.3])
translate([0, 0, 33])
    cylinder(h = 1.5, r = 10.5, $fn = 32);

// Base plate
color([0.5, 0.25, 0.25])
cylinder(h = 1, r = 16, $fn = 32);
`,

  paramGear: `// Parametric Gear using module + for + math
module gear(teeth=12, r=20, h=5) {
  cylinder(h=h, r=r*0.7, $fn=teeth*2);
  for(i=[0:teeth-1])
    rotate([0,0,i*360/teeth])
      translate([r*0.85, 0, 0])
        cylinder(h=h, r=r*0.15, $fn=6);
}
gear(teeth=16, r=25);
`,

  staircase: `// Spiral Staircase using for + translate + rotate
for(i=[0:15]) {
  rotate([0,0,i*22.5])
    translate([15,0,i*2])
      cube([12,4,1.5]);
}
// Central column
cylinder(h=35, r=3, $fn=24);
// Railing posts
for(i=[0:15])
  rotate([0,0,i*22.5])
    translate([26,0,i*2])
      cylinder(h=3, r=0.5, $fn=8);
`,

  paramVase: `// Parametric Vase using module + for + math
module vase_ring(z, r) {
  translate([0,0,z])
    cylinder(h=1.5, r1=r, r2=r, $fn=32);
}
for(i=[0:20]) {
  r = 8 + sin(i*18)*4;
  vase_ring(z=i*1.5, r=r);
}
`,

  chess: `// Simplified chess pawn piece

// Base
color([0.85, 0.8, 0.7])
cylinder(h = 3, r1 = 14, r2 = 12, $fn = 32);

// Base ring
color([0.8, 0.75, 0.65])
translate([0, 0, 3])
    cylinder(h = 2, r1 = 12, r2 = 10, $fn = 32);

// Lower column
color([0.85, 0.8, 0.7])
translate([0, 0, 5])
    cylinder(h = 12, r1 = 10, r2 = 6, $fn = 32);

// Collar
color([0.8, 0.75, 0.65])
translate([0, 0, 17])
    cylinder(h = 2, r1 = 7, r2 = 7.5, $fn = 32);

color([0.8, 0.75, 0.65])
translate([0, 0, 19])
    cylinder(h = 2, r1 = 7.5, r2 = 6, $fn = 32);

// Upper column (neck)
color([0.85, 0.8, 0.7])
translate([0, 0, 21])
    cylinder(h = 8, r1 = 6, r2 = 5, $fn = 32);

// Head (sphere)
color([0.9, 0.85, 0.75])
translate([0, 0, 33])
    sphere(r = 7, $fn = 32);

// Top nub
color([0.85, 0.8, 0.7])
translate([0, 0, 39])
    sphere(r = 3, $fn = 24);
`,

  mechanical: `// Mechanical assembly: gear + prism + radial_array + capsule
// Central gear
color([0.6, 0.6, 0.7])
gear(teeth=16, mod=2, thickness=6);

// Hub
color([0.5, 0.5, 0.6])
translate([0, 0, 6])
  cylinder(h=3, r=5, $fn=32);

// Hexagonal standoff (prism)
color([0.7, 0.7, 0.4])
translate([0, 0, 9])
  prism(sides=6, r=4, h=5, center=false);

// Radial array of capsule pins
color([0.8, 0.4, 0.3])
translate([0, 0, 3])
radial_array(count=8, r=12) {
  capsule(r=1.2, h=4, center=true, $fn=16);
}

// Mounting bolts at corners using linear_array
color([0.4, 0.5, 0.6])
translate([-20, -20, 0])
linear_array(count=3, spacing=[20, 0, 0]) {
  cone(r=3, h=8, $fn=24);
}

// Base plate
color([0.3, 0.3, 0.35])
translate([0, 0, -2])
  cylinder(h=2, r=22, $fn=48);
`,

  honeycomb: `// Honeycomb hex grid
color([0.9, 0.7, 0.2])
honeycomb(rows=4, cols=4, r=5, h=3, wall=1);
`,

  springCoil: `// Coil spring
color([0.5, 0.5, 0.6])
spring(r=10, wire_r=1.5, coils=6, pitch=4, $fn=20);
`,

  knurledCylinder: `// Knurled cylinder
color([0.6, 0.6, 0.7])
knurl(d=15, h=20, pitch=2, depth=0.8, $fn=48);
`,

  chamferedBox: `// Chamfered cube
color([0.4, 0.6, 0.8])
chamfer_cube(size=[20,15,10], chamfer=2, center=true);
`,

  loftShape: `// Loft between profiles
color([0.5, 0.8, 0.5])
loft(
  profiles=[
    [[0,0],[10,0],[10,10],[0,10]],
    [[2,2],[12,2],[8,12],[0,8]],
    [[1,1],[9,1],[9,9],[1,9]]
  ],
  heights=[0, 15, 30],
  $fn=16
);
`,

  lattice: `// Cubic lattice structure
// Great for lightweight 3D-printable infill
color([0.3, 0.6, 0.9])
lattice(type="cubic", cell=5, r=0.4, size=[25,25,25], $fn=8);
`,

  slotCross: `// Slot and cross primitives
// Stadium-shaped slot for bolt holes
color([0.6, 0.7, 0.8])
slot(length=30, width=8, h=5);

// Cross / plus shape
translate([0, 20, 0])
color([0.8, 0.5, 0.3])
cross(size=[25,25,8], arm=7);
`,

  maze: `// Random maze generator
color([0.7, 0.6, 0.5])
maze(rows=8, cols=8, cell=4, wall=0.8, h=3);
`,

  fibSphere: `// Fibonacci sphere — uniform point distribution
color([0.2, 0.7, 0.6])
fibonacci_sphere(count=150, r=15, $fn=8);
`,
}
</script>

<template>
  <div class="app" :class="[isDark ? 'dark' : 'light', { 'app-loaded': appLoaded, 'zen-mode': zenModeActive }]">
    <nav class="topbar">
      <div class="topbar-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
        </svg>
        <span class="brand topbar-brand-text">{{ t('title') }}</span>
      </div>
      <div class="topbar-right">
        <!-- Always visible: Render-relevant & essential controls -->
        <button class="tb-btn" @click="toggleLang" :aria-label="t('ariaToggleLang')">{{ lang === 'ru' ? 'RU' : lang === 'en' ? 'EN' : lang === 'de' ? 'DE' : 'ZH' }}</button>
        <!-- Theme selector dropdown -->
        <div class="theme-selector-wrapper">
          <button class="tb-btn" @click.stop="showThemeDropdown = !showThemeDropdown" :title="t('themeSelector')" :aria-label="t('ariaThemeSelector')">
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
        <!-- Mode toggle: Simple / Advanced -->
        <button class="tb-btn topbar-collapsible" @click="simpleMode = !simpleMode" :title="t('modeToggle')">
          {{ simpleMode ? t('simpleMode') : t('advancedMode') }}
        </button>
        <!-- Collapsible buttons: hidden on small screens, shown in hamburger -->
        <button class="tb-btn topbar-collapsible" @click="toggleScadReference" :title="t('scadReference')" :aria-label="t('ariaScadReference')">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 19.5A2.5 2.5 0 016.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z"/>
          </svg>
        </button>
        <button class="tb-btn tb-btn-help topbar-collapsible" :class="{ 'has-badge': hasNewFeatures }" @click="if (hasNewFeatures) { openWelcome(); dismissNewFeatures() } else { showShortcuts = true }" :title="hasNewFeatures ? t('whatsNew') : t('shortcuts')" :aria-label="t('ariaHelp')">?<span v-if="hasNewFeatures" class="notif-badge"></span></button>
        <button class="tb-btn tb-btn-gear topbar-collapsible" @click="showPreferences = !showPreferences" :title="t('preferences')" :aria-label="t('ariaPreferences')">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/>
          </svg>
        </button>
        <!-- Hamburger button: visible only on small screens -->
        <div class="hamburger-wrapper">
          <button class="tb-btn hamburger-btn" @click.stop="hamburgerOpen = !hamburgerOpen" :title="t('menu')">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
            </svg>
          </button>
          <div v-if="hamburgerOpen" class="hamburger-dropdown" @click="hamburgerOpen = false">
            <button class="hamburger-item" @click="toggleScadReference">{{ t('scadReference') }}</button>
            <button class="hamburger-item" @click="showShortcuts = true">{{ t('shortcuts') }}</button>
            <button class="hamburger-item" @click="showPreferences = !showPreferences">{{ t('preferences') }}</button>
            <button class="hamburger-item" @click="saveReferenceScreenshot()">{{ t('saveReference') }}</button>
            <button class="hamburger-item" @click="compareWithReference()">{{ t('compareScreenshots') }}</button>
            <button class="hamburger-item" @click="showAbout = true">{{ t('about') }}</button>
          </div>
        </div>
      </div>
    </nav>

    <!-- About modal -->
    <Teleport to="body">
      <div v-if="showAbout" class="modal-backdrop" @click.self="showAbout = false">
        <div class="modal-box" role="dialog" aria-modal="true" style="max-width:400px">
          <div class="modal-header">
            <span class="modal-title">{{ t('aboutTitle') }}</span>
            <button class="modal-close" @click="showAbout = false">&times;</button>
          </div>
          <div class="modal-body" style="text-align:center;padding:20px">
            <div style="font-size:20px;font-weight:700;margin-bottom:4px">OpenSCAD 3D Viewer</div>
            <div style="color:var(--text-dim);margin-bottom:12px">{{ t('aboutVersion') }}: 0.1.0</div>
            <div style="margin-bottom:8px">{{ t('aboutBuiltWith') }}</div>
            <div style="color:var(--text-dim);margin-bottom:12px">{{ t('aboutLicense') }}</div>
            <div style="font-size:11px;color:var(--text-dim)">WebGPU-powered 3D rendering</div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Screenshot Comparison modal -->
    <Teleport to="body">
      <div v-if="showScreenshotCompare" class="modal-backdrop" @click.self="showScreenshotCompare = false">
        <div class="modal-box" role="dialog" aria-modal="true" style="max-width:900px;width:90vw">
          <div class="modal-header">
            <span class="modal-title">{{ t('compareScreenshots') }}</span>
            <div class="modal-header-actions">
              <button :class="{'btn': true, 'btn-sm': true, 'btn-active': screenshotCompareMode === 'sideBySide'}" @click="screenshotCompareMode = 'sideBySide'" style="margin-right:4px">{{ t('sideBySide') }}</button>
              <button :class="{'btn': true, 'btn-sm': true, 'btn-active': screenshotCompareMode === 'overlay'}" @click="screenshotCompareMode = 'overlay'" style="margin-right:4px">{{ t('overlay') }}</button>
              <button class="modal-close" @click="showScreenshotCompare = false">&times;</button>
            </div>
          </div>
          <div class="modal-body" style="padding:12px">
            <div v-if="screenshotCompareMode === 'sideBySide'" style="display:flex;gap:12px;justify-content:center">
              <div style="flex:1;text-align:center">
                <div style="font-size:11px;color:var(--text-dim);margin-bottom:4px">{{ t('saveReference') }}</div>
                <img v-if="referenceScreenshot" :src="referenceScreenshot" style="max-width:100%;border:1px solid var(--border);border-radius:4px" />
              </div>
              <div style="flex:1;text-align:center">
                <div style="font-size:11px;color:var(--text-dim);margin-bottom:4px">{{ t('screenshot') }}</div>
                <img v-if="currentScreenshot" :src="currentScreenshot" style="max-width:100%;border:1px solid var(--border);border-radius:4px" />
              </div>
            </div>
            <div v-else-if="screenshotCompareMode === 'overlay'" style="position:relative;text-align:center">
              <img v-if="referenceScreenshot" :src="referenceScreenshot" style="max-width:100%;border:1px solid var(--border);border-radius:4px" />
              <img v-if="currentScreenshot" :src="currentScreenshot" :style="{position:'absolute',top:'0',left:'50%',transform:'translateX(-50%)',maxWidth:'100%',opacity: comparisonSlider / 100,borderRadius:'4px'}" />
              <input type="range" v-model.number="comparisonSlider" min="0" max="100" style="width:100%;margin-top:8px" />
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Shortcuts modal -->
    <Teleport to="body">
      <div v-if="showShortcuts" class="modal-backdrop" @click.self="showShortcuts = false">
        <div class="modal-box shortcuts-modal" role="dialog" aria-modal="true" :aria-label="t('ariaShortcutsDialog')">
          <div class="modal-header">
            <span class="modal-title">{{ t('shortcutsTitle') }}</span>
            <div class="modal-header-actions">
              <button class="btn btn-sm shortcuts-print-btn" @click="printShortcuts">{{ t('printShortcuts') }}</button>
              <button class="modal-close" @click="showShortcuts = false" :aria-label="t('ariaCloseModal')">&times;</button>
            </div>
          </div>
          <div class="modal-body shortcuts-scroll">
            <div class="shortcut-category">{{ t('shortcatEditor') }}</div>
            <div class="shortcut-row"><kbd>Ctrl+Enter</kbd><span>{{ t('sc_render') }}</span></div>
            <div class="shortcut-row"><kbd>Tab</kbd><span>{{ t('sc_indent') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Z</kbd><span>{{ t('sc_undo_custom') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Shift+Z / Ctrl+Y</kbd><span>{{ t('sc_redo') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+F</kbd><span>{{ t('sc_findOnly') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+H</kbd><span>{{ t('sc_findReplace') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+/</kbd><span>{{ t('sc_commentToggle') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+D</kbd><span>{{ t('sc_duplicateLine') }}</span></div>
            <div class="shortcut-row"><kbd>Alt+Up</kbd><span>{{ t('sc_moveLineUp') }}</span></div>
            <div class="shortcut-row"><kbd>Alt+Down</kbd><span>{{ t('sc_moveLineDown') }}</span></div>
            <div class="shortcut-row"><kbd>Alt+Z</kbd><span>{{ t('sc_wordWrap') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+G</kbd><span>{{ t('sc_goToLine') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Shift+P / F1</kbd><span>{{ t('sc_commandPalette') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+J</kbd><span>{{ t('sc_joinLines') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Shift+U</kbd><span>{{ t('sc_upperCase') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Shift+L</kbd><span>{{ t('sc_lowerCase') }}</span></div>

            <div class="shortcut-category">{{ t('shortcatNavigation') }}</div>
            <div class="shortcut-row"><kbd>W / A / S / D</kbd><span>{{ t('sc_wasd') }}</span></div>
            <div class="shortcut-row"><kbd>Q / E</kbd><span>{{ t('sc_qe') }}</span></div>
            <div class="shortcut-row"><kbd>Shift+W/A/S/D</kbd><span>{{ t('sc_shiftWasd') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+{{ lang === 'ru' ? 'ЛКМ' : 'LMB' }}</kbd><span>{{ t('sc_orbitSnap') }}</span></div>

            <div class="shortcut-category">{{ t('shortcatView') }}</div>
            <div class="shortcut-row"><kbd>?</kbd><span>{{ t('sc_shortcuts') }}</span></div>
            <div class="shortcut-row"><kbd>Escape</kbd><span>{{ t('sc_close') }}</span></div>

            <div class="shortcut-category">{{ t('shortcatFile') }}</div>
            <div class="shortcut-row"><kbd>Ctrl+Tab</kbd><span>{{ t('sc_nextTab') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+Shift+Tab</kbd><span>{{ t('sc_prevTab') }}</span></div>
            <div class="shortcut-row"><kbd>Ctrl+W</kbd><span>{{ t('sc_closeTab') }}</span></div>
          </div>
        </div>
      </div>

      <!-- OpenSCAD Reference Panel -->
      <div v-if="showScadReference" class="modal-backdrop" @click.self="showScadReference = false">
        <div class="modal-box scad-ref-modal" role="dialog" aria-modal="true" :aria-label="t('ariaScadReference')">
          <div class="modal-header">
            <span class="modal-title">{{ t('scadReference') }}</span>
            <button class="modal-close" @click="showScadReference = false" :aria-label="t('ariaCloseModal')">&times;</button>
          </div>
          <div class="ref-search-bar">
            <svg class="ref-search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input class="ref-search-input" v-model="refSearchQuery" :placeholder="t('refSearch')" />
          </div>
          <div class="modal-body scad-ref-scroll">
            <div v-for="section in filteredRefSections" :key="section.key" class="scad-ref-section">
              <div class="scad-ref-section-title" v-html="highlightMatch(t(section.key))"></div>
              <div v-for="entry in section.entries" :key="entry.name" class="scad-ref-entry">
                <div class="scad-ref-name" v-html="highlightMatch(entry.name)"></div>
                <code class="scad-ref-sig" v-html="highlightMatch(entry.sig)"></code>
                <div class="scad-ref-desc" v-html="highlightMatch((entry.desc as any)[lang] || entry.desc.en)"></div>
              </div>
            </div>
            <div v-if="filteredRefSections.length === 0" class="ref-no-results">{{ t('noMatches') }}</div>
          </div>
        </div>
      </div>

      <!-- Command Palette -->
      <div v-if="showCommandPalette" class="command-palette-backdrop" @click.self="closeCommandPalette">
        <div class="command-palette" role="dialog" aria-modal="true" :aria-label="t('ariaCommandPaletteDialog')">
          <div class="command-palette-input-wrapper">
            <svg class="command-palette-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input
              ref="commandSearchInputRef"
              class="command-palette-input"
              v-model="commandSearch"
              :placeholder="t('commandPalette')"
              @keydown="handleCommandPaletteKeydown"
            />
          </div>
          <div class="command-palette-list" v-if="filteredCommands.length">
            <div
              v-for="(cmd, idx) in filteredCommands"
              :key="cmd.id"
              class="command-palette-item"
              :class="{ active: idx === commandSelectedIndex }"
              @click="executeCommand(cmd)"
              @mouseenter="commandSelectedIndex = idx"
            >
              <span class="command-palette-label">{{ cmd.label() }}</span>
              <kbd v-if="cmd.shortcut" class="command-palette-shortcut">{{ cmd.shortcut }}</kbd>
            </div>
          </div>
          <div v-else class="command-palette-empty">{{ t('noMatches') }}</div>
        </div>
      </div>

      <!-- Go to Line dialog -->
      <div v-if="showGoToLine" class="go-to-line-backdrop" @click.self="closeGoToLine">
        <div class="go-to-line-box" role="dialog" aria-modal="true" :aria-label="t('ariaGoToLineDialog')">
          <input
            ref="goToLineInputRef"
            class="go-to-line-input"
            v-model="goToLineText"
            :placeholder="t('goToLinePlaceholder').replace('{n}', String(lineCount))"
            @keydown="handleGoToLineKeydown"
            type="number"
            min="1"
            :max="lineCount"
          />
        </div>
      </div>

      <!-- Preferences modal -->
      <div v-if="showPreferences" class="modal-backdrop" @click.self="showPreferences = false">
        <div class="modal-box pref-modal" role="dialog" aria-modal="true" :aria-label="t('ariaPreferencesDialog')">
          <div class="modal-header">
            <span class="modal-title">{{ t('preferences') }}</span>
            <button class="modal-close" @click="showPreferences = false" :aria-label="t('ariaCloseModal')">&times;</button>
          </div>
          <div class="modal-body">
            <div class="pref-row">
              <label class="pref-label">{{ t('fontSize') }}</label>
              <div class="pref-control">
                <input type="range" min="10" max="20" v-model.number="prefFontSize" class="pref-slider" />
                <span class="pref-value">{{ prefFontSize }}px</span>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('tabSize') }}</label>
              <div class="pref-control pref-radio-group">
                <label class="pref-radio"><input type="radio" :value="2" v-model.number="prefTabSize" /> 2</label>
                <label class="pref-radio"><input type="radio" :value="4" v-model.number="prefTabSize" /> 4</label>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('autoRenderDelay') }}</label>
              <div class="pref-control">
                <input type="range" min="100" max="2000" step="50" v-model.number="prefAutoRenderDelay" class="pref-slider" />
                <span class="pref-value">{{ prefAutoRenderDelay }}ms</span>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('showMinimapPref') }}</label>
              <div class="pref-control">
                <input type="checkbox" v-model="prefShowMinimap" class="pref-checkbox" />
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('showLineNumbers') }}</label>
              <div class="pref-control">
                <input type="checkbox" v-model="prefShowLineNumbers" class="pref-checkbox" />
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('fontFamily') }}</label>
              <div class="pref-control">
                <select class="pref-select" v-model="prefFontFamily">
                  <option v-for="font in FONT_OPTIONS" :key="font" :value="font" :style="{ fontFamily: font + ', monospace' }">{{ font }}</option>
                </select>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('modeToggle') }}</label>
              <div class="pref-control pref-radio-group">
                <label class="pref-radio"><input type="radio" :value="false" v-model="simpleMode" /> {{ t('advancedMode') }}</label>
                <label class="pref-radio"><input type="radio" :value="true" v-model="simpleMode" /> {{ t('simpleMode') }}</label>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('shortcutPreset') }}</label>
              <div class="pref-control">
                <select class="pref-select" v-model="shortcutPreset">
                  <option value="default">{{ t('presetDefault') }}</option>
                  <option value="vscode">{{ t('presetVSCode') }}</option>
                  <option value="sublime">{{ t('presetSublime') }}</option>
                  <option value="emacs">{{ t('presetEmacs') }}</option>
                </select>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('customizeShortcuts') }}</label>
              <div class="pref-control">
                <button class="btn btn-sm" @click="showShortcutEditor = true">{{ t('customizeShortcuts') }}</button>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('watermark') }}</label>
              <div class="pref-control">
                <input type="checkbox" v-model="wmEnabled" class="pref-checkbox" />
              </div>
            </div>
            <div v-if="wmEnabled" class="pref-row">
              <label class="pref-label">{{ t('watermarkText') }}</label>
              <div class="pref-control">
                <input type="text" v-model="wmText" class="pref-text-input" placeholder="Project name..." />
              </div>
            </div>
            <div v-if="wmEnabled" class="pref-row">
              <label class="pref-label">{{ t('watermarkPosition') }}</label>
              <div class="pref-control">
                <select class="pref-select" v-model="wmPosition">
                  <option value="top-left">{{ t('wmTopLeft') }}</option>
                  <option value="top-right">{{ t('wmTopRight') }}</option>
                  <option value="bottom-left">{{ t('wmBottomLeft') }}</option>
                  <option value="bottom-right">{{ t('wmBottomRight') }}</option>
                </select>
              </div>
            </div>
            <div v-if="wmEnabled" class="pref-row">
              <label class="pref-label">{{ t('watermarkOpacity') }}</label>
              <div class="pref-control">
                <input type="range" min="0.05" max="0.5" step="0.05" v-model.number="wmOpacity" class="pref-slider" />
                <span class="pref-value">{{ (wmOpacity * 100).toFixed(0) }}%</span>
              </div>
            </div>
            <div class="pref-section-divider"></div>
            <div class="pref-section-title">{{ t('screenshotMetadata') }}</div>
            <div class="pref-row">
              <label class="pref-label">{{ t('metaModelName') }}</label>
              <div class="pref-control"><input type="checkbox" v-model="screenshotMeta.modelName" class="pref-checkbox" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('metaDimensions') }}</label>
              <div class="pref-control"><input type="checkbox" v-model="screenshotMeta.dimensions" class="pref-checkbox" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('metaTriangles') }}</label>
              <div class="pref-control"><input type="checkbox" v-model="screenshotMeta.triangles" class="pref-checkbox" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('metaDate') }}</label>
              <div class="pref-control"><input type="checkbox" v-model="screenshotMeta.date" class="pref-checkbox" /></div>
            </div>
            <!-- Custom Theme Section -->
            <div class="pref-section-divider"></div>
            <div class="pref-section-title">{{ t('customTheme') }}</div>
            <div class="pref-row">
              <label class="pref-label">{{ t('themeBackground') }}</label>
              <div class="pref-control"><input type="color" v-model="customThemeBg" class="pref-color-input" @input="applyCustomThemePreview" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('themeSurface') }}</label>
              <div class="pref-control"><input type="color" v-model="customThemeSurface" class="pref-color-input" @input="applyCustomThemePreview" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('themeBorder') }}</label>
              <div class="pref-control"><input type="color" v-model="customThemeBorder" class="pref-color-input" @input="applyCustomThemePreview" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('themeText') }}</label>
              <div class="pref-control"><input type="color" v-model="customThemeText" class="pref-color-input" @input="applyCustomThemePreview" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('themeAccent') }}</label>
              <div class="pref-control"><input type="color" v-model="customThemeAccent" class="pref-color-input" @input="applyCustomThemePreview" /></div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('customThemeName') }}</label>
              <div class="pref-control preset-save-row">
                <input type="text" v-model="customThemeName" class="pref-text-input" :placeholder="t('themeNamePlaceholder')" @keydown.enter="saveCustomTheme" />
                <button class="btn btn-sm btn-primary" @click="saveCustomTheme" :disabled="!customThemeName.trim()">{{ t('themeSave') }}</button>
              </div>
            </div>
            <div v-if="customThemes.length > 0" class="pref-row">
              <label class="pref-label">{{ t('customTheme') }}</label>
              <div class="pref-control preset-list">
                <div v-for="ct in customThemes" :key="ct.name" class="preset-item">
                  <button class="btn btn-sm preset-btn" @click="selectTheme('custom-' + ct.name)">{{ ct.name }}</button>
                  <button class="btn btn-sm preset-del-btn" @click="deleteCustomTheme(ct.name)">&times;</button>
                </div>
              </div>
            </div>
            <!-- Presets Section -->
            <div class="pref-section-divider"></div>
            <div class="pref-section-title">{{ t('presets') }}</div>
            <div class="pref-row">
              <label class="pref-label">{{ t('builtInPresets') }}</label>
              <div class="pref-control preset-btns">
                <button class="btn btn-sm preset-btn" @click="applyPreset(BUILT_IN_PRESETS.cadPro)">{{ t('presetCadPro') }}</button>
                <button class="btn btn-sm preset-btn" @click="applyPreset(BUILT_IN_PRESETS.printPreview)">{{ t('presetPrintPreview') }}</button>
                <button class="btn btn-sm preset-btn" @click="applyPreset(BUILT_IN_PRESETS.presentation)">{{ t('presetPresentation') }}</button>
              </div>
            </div>
            <div class="pref-row" v-if="Object.keys(userPresets).length > 0">
              <label class="pref-label">{{ t('userPresets') }}</label>
              <div class="pref-control preset-list">
                <div v-for="(preset, name) in userPresets" :key="name" class="preset-item">
                  <button class="btn btn-sm preset-btn" @click="applyPreset(preset)">{{ name }}</button>
                  <button class="btn btn-sm preset-del-btn" @click="deleteUserPreset(name as string)">&times;</button>
                </div>
              </div>
            </div>
            <div class="pref-row">
              <label class="pref-label">{{ t('savePreset') }}</label>
              <div class="pref-control preset-save-row">
                <input type="text" v-model="presetNameInput" class="pref-text-input" :placeholder="t('presetName')" @keydown.enter="savePreset" />
                <button class="btn btn-sm btn-primary" @click="savePreset" :disabled="!presetNameInput.trim()">{{ t('savePreset') }}</button>
              </div>
            </div>
            <div class="pref-footer">
              <button class="btn btn-sm pref-reset-btn" @click="resetPreferences">{{ t('resetPrefs') }}</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Batch Results modal -->
      <div v-if="showBatchResults" class="modal-backdrop" @click.self="showBatchResults = false">
        <div class="modal-box" role="dialog" aria-modal="true">
          <div class="modal-header">
            <span class="modal-title">{{ t('batchResults') }}</span>
            <button class="modal-close" @click="showBatchResults = false">&times;</button>
          </div>
          <div class="modal-body">
            <table style="width:100%;border-collapse:collapse;">
              <thead>
                <tr>
                  <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border);">{{ t('batchTabName') }}</th>
                  <th style="text-align:right;padding:4px 8px;border-bottom:1px solid var(--border);">{{ t('batchTriangles') }}</th>
                  <th style="text-align:right;padding:4px 8px;border-bottom:1px solid var(--border);">{{ t('batchTime') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(row, i) in batchResults" :key="i">
                  <td style="padding:4px 8px;">{{ row.name }}</td>
                  <td style="text-align:right;padding:4px 8px;">{{ row.tris.toLocaleString() }}</td>
                  <td style="text-align:right;padding:4px 8px;">{{ row.timeMs }}ms</td>
                </tr>
              </tbody>
              <tfoot>
                <tr style="font-weight:bold;border-top:1px solid var(--border);">
                  <td style="padding:4px 8px;">{{ t('batchTotal') }}</td>
                  <td style="text-align:right;padding:4px 8px;">{{ batchResults.reduce((s, r) => s + r.tris, 0).toLocaleString() }}</td>
                  <td style="text-align:right;padding:4px 8px;">{{ batchResults.reduce((s, r) => s + r.timeMs, 0) }}ms</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      </div>

      <!-- Batch rendering progress -->
      <div v-if="batchRendering" class="modal-backdrop" style="z-index:9999;">
        <div style="background:var(--bg);padding:24px 32px;border-radius:12px;text-align:center;">
          <div style="margin-bottom:12px;font-size:14px;">{{ t('batchRendering') }}</div>
          <div style="width:240px;height:6px;background:var(--border);border-radius:3px;overflow:hidden;">
            <div :style="{width: (batchTotal > 0 ? (batchProgress/batchTotal*100) : 0) + '%', height:'100%', background:'var(--accent)', transition:'width 0.2s'}"></div>
          </div>
          <div style="margin-top:8px;font-size:12px;opacity:0.7;">{{ t('batchProgress').replace('{n}', String(batchProgress)).replace('{total}', String(batchTotal)) }}</div>
        </div>
      </div>

      <!-- Shortcut Editor modal -->
      <div v-if="showShortcutEditor" class="modal-backdrop" @click.self="showShortcutEditor = false">
        <div class="modal-box" role="dialog" aria-modal="true" style="max-width:520px;">
          <div class="modal-header">
            <span class="modal-title">{{ t('shortcutEditorTitle') }}</span>
            <button class="modal-close" @click="showShortcutEditor = false">&times;</button>
          </div>
          <div class="modal-body">
            <table style="width:100%;border-collapse:collapse;">
              <thead>
                <tr>
                  <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border);">{{ t('shortcutAction') }}</th>
                  <th style="text-align:left;padding:4px 8px;border-bottom:1px solid var(--border);">{{ t('shortcutBinding') }}</th>
                  <th style="padding:4px 8px;border-bottom:1px solid var(--border);"></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="sc in ALL_SHORTCUTS" :key="sc.id">
                  <td style="padding:4px 8px;font-size:13px;">{{ sc.label() }}</td>
                  <td style="padding:4px 8px;">
                    <kbd v-if="capturingShortcutId !== sc.id" style="font-size:11px;padding:2px 6px;background:var(--bg);border:1px solid var(--border);border-radius:3px;">{{ getEffectiveBinding(sc.id) }}</kbd>
                    <span v-else style="font-size:11px;color:var(--accent);font-style:italic;">{{ t('shortcutCapturing') }}</span>
                  </td>
                  <td style="padding:4px 8px;white-space:nowrap;">
                    <button v-if="capturingShortcutId !== sc.id" class="btn btn-sm" @click="startCapture(sc.id)" style="margin-right:4px;">{{ t('shortcutEdit') }}</button>
                    <button v-if="capturingShortcutId === sc.id" class="btn btn-sm" @click="capturingShortcutId = null">{{ t('shortcutCancel') }}</button>
                    <button v-if="customBindings[sc.id]" class="btn btn-sm" @click="resetBinding(sc.id)">{{ t('shortcutReset') }}</button>
                  </td>
                </tr>
              </tbody>
            </table>
            <div style="margin-top:12px;text-align:right;">
              <button class="btn btn-sm" @click="resetAllBindings">{{ t('shortcutResetAll') }}</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Example Gallery Modal -->
      <div v-if="showExampleGallery" class="modal-backdrop" @click.self="closeExampleGallery">
        <div class="modal-box example-gallery-modal" role="dialog" aria-modal="true" :aria-label="t('ariaExampleGallery')">
          <div class="modal-header">
            <span class="modal-title">{{ t('exampleGallery') }}</span>
            <button class="modal-close" @click="closeExampleGallery" :aria-label="t('ariaCloseModal')">&times;</button>
          </div>
          <div class="modal-body">
            <input
              class="example-gallery-search"
              v-model="exampleGallerySearch"
              :placeholder="t('exampleGallerySearch')"
              autofocus
            />
            <div class="example-gallery-grid">
              <div
                v-for="card in filteredExamples"
                :key="card.key"
                class="example-card"
                @click="loadExampleFromGallery(card.key)"
              >
                <div class="example-card-name">{{ t(card.nameKey) }}</div>
                <div class="example-card-desc">{{ t(card.tipKey) }}</div>
                <pre class="example-card-code"><code>{{ getExamplePreview(card.key) }}</code></pre>
                <button class="btn btn-sm example-card-load">{{ t('loadExample') }}</button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Welcome / Onboarding modal -->
      <div v-if="showWelcome" class="modal-backdrop" @click.self="dismissWelcome">
        <div class="modal-box welcome-modal" role="dialog" aria-modal="true" :aria-label="t('ariaWelcomeDialog')">
          <div class="modal-header">
            <span class="modal-title">
              <svg class="welcome-logo" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
              </svg>
              {{ t('welcomeTitle') }}
            </span>
            <button class="modal-close" @click="dismissWelcome" :aria-label="t('ariaCloseModal')">&times;</button>
          </div>
          <div class="modal-body">
            <p class="welcome-tagline">{{ t('welcomeTagline') }}</p>
            <ul class="welcome-list">
              <li class="welcome-item">
                <svg class="welcome-check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                <span>{{ t('welcomeFeat1') }}</span>
              </li>
              <li class="welcome-item">
                <svg class="welcome-check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                <span>{{ t('welcomeFeat2') }}</span>
              </li>
              <li class="welcome-item">
                <svg class="welcome-check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                <span>{{ t('welcomeFeat3') }}</span>
              </li>
              <li class="welcome-item">
                <svg class="welcome-check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                <span>{{ t('welcomeFeat4') }}</span>
              </li>
            </ul>
            <button class="btn btn-primary welcome-start-btn" @click="dismissWelcome(); dismissNewFeatures()">{{ t('welcomeStart') }}</button>
            <button class="btn btn-sm welcome-tour-btn" @click="startTour(); dismissNewFeatures()">{{ t('tourStart') }}</button>
            <button class="btn btn-sm welcome-tour-btn" @click="dismissWelcome(); dismissNewFeatures(); openPlayground()" style="margin-left:6px">{{ t('playground') }}</button>
            <!-- What's New section -->
            <div class="whats-new-section">
              <h3 class="whats-new-title">{{ t('whatsNewTitle') }}</h3>
              <div v-for="release in WHATS_NEW_ITEMS" :key="release.version" class="whats-new-release">
                <div class="whats-new-version">v{{ release.version }}</div>
                <ul class="whats-new-list">
                  <li v-for="(item, idx) in (release.items as any)[lang] || release.items.en" :key="idx">{{ item }}</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Playground Modal -->
      <div v-if="showPlayground" class="modal-backdrop" @click.self="showPlayground = false">
        <div class="modal-box playground-modal" role="dialog" aria-modal="true">
          <div class="modal-header">
            <span class="modal-title">{{ t('playgroundTitle') }}</span>
            <button class="modal-close" @click="showPlayground = false">&times;</button>
          </div>
          <div class="modal-body">
            <template v-if="playgroundStep < PLAYGROUND_CHALLENGES.length">
              <div class="playground-progress">
                <span v-for="(_, idx) in PLAYGROUND_CHALLENGES" :key="idx"
                  class="playground-dot"
                  :class="{ completed: playgroundCompleted[idx], active: idx === playgroundStep }"
                ></span>
              </div>
              <div class="playground-challenge-num">{{ t('playgroundChallenge').replace('{n}', String(playgroundStep + 1)) }}</div>
              <div class="playground-goal">
                <strong>{{ t('playgroundGoal') }}:</strong>
                {{ (PLAYGROUND_CHALLENGES[playgroundStep].goal as any)[lang] || PLAYGROUND_CHALLENGES[playgroundStep].goal.en }}
              </div>
              <div class="playground-hint">
                <strong>{{ t('playgroundHint') }}:</strong>
                <code>{{ (PLAYGROUND_CHALLENGES[playgroundStep].hint as any)[lang] || PLAYGROUND_CHALLENGES[playgroundStep].hint.en }}</code>
              </div>
              <div class="playground-actions">
                <button class="btn btn-primary" @click="checkPlayground">{{ t('playgroundCheck') }}</button>
                <button v-if="playgroundCompleted[playgroundStep]" class="btn btn-sm" @click="nextPlaygroundChallenge">{{ t('playgroundNext') }}</button>
              </div>
            </template>
            <template v-else>
              <div class="playground-complete">{{ t('playgroundComplete') }}</div>
              <button class="btn btn-sm" @click="resetPlayground">{{ t('playgroundReset') }}</button>
            </template>
          </div>
        </div>
      </div>

      <!-- Interactive Tour Overlay -->
      <div v-if="tourActive" class="tour-overlay" @click.self="endTour">
        <div class="tour-spotlight" :style="tourSpotlightStyle"></div>
        <div class="tour-tooltip" :style="tourTooltipStyle">
          <div class="tour-tooltip-step">{{ tourStep + 1 }} / {{ TOUR_STEPS.length }}</div>
          <div class="tour-tooltip-text">{{ t(TOUR_STEPS[tourStep].key) }}</div>
          <div class="tour-tooltip-actions">
            <button v-if="tourStep > 0" class="btn btn-sm tour-btn" @click="prevTourStep">{{ t('tourPrev') }}</button>
            <span style="flex:1"></span>
            <button class="btn btn-sm tour-btn" @click="endTour">{{ t('tourSkip') }}</button>
            <button class="btn btn-sm btn-primary tour-btn" @click="nextTourStep">{{ tourStep < TOUR_STEPS.length - 1 ? t('tourNext') : t('tourFinish') }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Tip of the Day Banner -->
    <transition name="tip-slide">
    <div v-if="showTipOfDay && !zenModeActive" class="tip-of-day-banner">
      <span class="tip-icon">&#128161;</span>
      <span class="tip-label">{{ t('tipOfDay') }}:</span>
      <span class="tip-text">{{ lang === 'ru' ? currentTip.ru : currentTip.en }}</span>
      <button class="tip-next-btn" @click="nextTip" title="&#8635;">&#8635;</button>
      <button class="tip-dismiss-btn" @click="dismissTipOfDay">{{ t('dontShowTips') }}</button>
      <button class="tip-close-btn" @click="showTipOfDay = false">&times;</button>
    </div>
    </transition>

    <!-- Zen Mode Exit Button -->
    <transition name="overlay-fade">
    <button v-if="zenModeActive" class="zen-exit-btn" @click="zenModeActive = false">{{ t('exitZenMode') }} (Esc)</button>
    </transition>

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
          <button class="btn btn-sm" @click="doRenderAllTabs" :disabled="batchRendering" :title="t('renderAllTabs')">
            {{ batchRendering ? t('batchProgress').replace('{n}', String(batchProgress)).replace('{total}', String(batchTotal)) : t('renderAllTabs') }}
          </button>
          <label class="auto-check">
            <input type="checkbox" v-model="autoRender" /> {{ t('auto') }}
          </label>
          <!-- Undo/Redo buttons -->
          <button class="btn btn-sm btn-icon undo-redo-btn" :class="{ 'btn-disabled': !undoStack.length }" @click="customUndo" :title="t('undoBtn') + (undoStack.length ? ' (' + undoStack.length + ')' : '')" :aria-label="t('undoBtn')">
            <span class="undo-redo-icon">&#8630;</span>
            <span v-if="undoStack.length" class="undo-redo-badge">{{ undoStack.length }}</span>
          </button>
          <button class="btn btn-sm btn-icon undo-redo-btn" :class="{ 'btn-disabled': !redoStack.length }" @click="customRedo" :title="t('redoBtn') + (redoStack.length ? ' (' + redoStack.length + ')' : '')" :aria-label="t('redoBtn')">
            <span class="undo-redo-icon">&#8631;</span>
            <span v-if="redoStack.length" class="undo-redo-badge">{{ redoStack.length }}</span>
          </button>
          <button class="btn btn-sm btn-icon" @click="openFile" :title="t('open')" :aria-label="t('open')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
          </button>
          <button class="btn btn-sm btn-icon" @click="saveFile" :title="t('save')" :aria-label="t('save')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
          </button>
          <!-- STL Export button -->
          <button class="btn btn-sm btn-icon" @click="doExportSTL" :title="t('exportStl')" :aria-label="t('exportStl')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              <rect x="14" y="1" width="8" height="6" rx="1" fill="currentColor" opacity="0.3"/>
            </svg>
          </button>
          <!-- OBJ Export button -->
          <button class="btn btn-sm btn-icon" @click="doExportOBJ" :title="t('exportObj')" :aria-label="t('exportObj')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              <rect x="14" y="1" width="8" height="6" rx="1" fill="currentColor" opacity="0.15"/>
            </svg>
          </button>
          <!-- Share button -->
          <div class="share-wrapper" v-show="!simpleMode">
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
          <button class="btn btn-sm" v-show="!simpleMode" @click="formatCode" :title="t('format')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <line x1="3" y1="6" x2="21" y2="6"/><line x1="7" y1="12" x2="21" y2="12"/><line x1="5" y1="18" x2="21" y2="18"/>
            </svg>
            {{ t('format') }}
          </button>
          <!-- Snapshot button -->
          <button class="btn btn-sm" v-show="!simpleMode" @click="takeSnapshot" :title="t('snapshotBtn')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="12" cy="12" r="3"/><path d="M3 9h2"/><path d="M19 9h2"/>
            </svg>
            {{ t('snapshotBtn') }}
          </button>
          <!-- Snapshot Gallery toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': showSnapshotGallery }" @click="showSnapshotGallery = !showSnapshotGallery" :title="t('snapshotGallery')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <rect x="2" y="2" width="8" height="8" rx="1"/><rect x="14" y="2" width="8" height="8" rx="1"/><rect x="2" y="14" width="8" height="8" rx="1"/><rect x="14" y="14" width="8" height="8" rx="1"/>
            </svg>
            {{ snapshots.length > 0 ? snapshots.length : '' }}
          </button>
          <!-- Profile toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': showProfilePanel }" @click="showProfilePanel = !showProfilePanel" :title="t('cmdToggleProfile')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <path d="M12 20V10"/><path d="M18 20V4"/><path d="M6 20v-4"/>
            </svg>
          </button>
          <!-- Recent dropdown -->
          <div class="recent-wrapper" v-show="!simpleMode">
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
          <!-- Parameters toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': showParameters }" @click="showParameters = !showParameters" :title="t('parameters')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/><line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/><line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/>
              <line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/>
            </svg>
            {{ t('parameters') }}
          </button>
          <!-- Diff toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': showDiff }" @click="showDiff = !showDiff" :title="t('cmdToggleDiff')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <path d="M12 3v18"/><path d="M5 9l-3 3 3 3"/><path d="M19 9l3 3-3 3"/>
            </svg>
            {{ t('showChanges') }}
          </button>
          <span v-if="showDiff && !simpleMode" class="diff-summary">
            <span class="diff-stat diff-stat-added">+{{ diffStats.added }}</span>
            <span class="diff-stat diff-stat-removed">-{{ diffStats.removed }}</span>
            <span class="diff-stat diff-stat-modified">~{{ diffStats.modified }}</span>
          </span>
          <!-- History toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': showHistory }" @click="showHistory = !showHistory" :title="t('cmdToggleHistory')" :aria-label="t('ariaHistory')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
            </svg>
            {{ t('history') }}
          </button>
          <!-- Split editor toggle -->
          <button class="btn btn-sm" v-show="!simpleMode" :class="{ 'btn-active': splitMode }" @click="toggleSplitMode" :title="t('splitEditorTip')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <rect x="3" y="3" width="18" height="18" rx="2"/><line x1="12" y1="3" x2="12" y2="21"/>
            </svg>
            {{ t('splitEditor') }}
          </button>
          <!-- Snippets button -->
          <div class="snippet-panel-wrapper" v-show="!simpleMode" @click.stop>
            <button class="btn btn-sm" :class="{ 'btn-active': showSnippetPanel }" @click="showSnippetPanel = !showSnippetPanel" :title="t('snippets')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
                <path d="M16 18l6-6-6-6"/><path d="M8 6l-6 6 6 6"/>
              </svg>
              {{ t('snippets') }}
            </button>
            <div v-if="showSnippetPanel" class="snippet-dropdown">
              <button
                v-for="s in SNIPPETS"
                :key="s.id"
                class="snippet-item"
                @click="insertSnippet(s)"
              >
                <span class="snippet-name">{{ t(s.nameKey) }}</span>
                <code class="snippet-preview">{{ s.code.substring(0, 40) }}{{ s.code.length > 40 ? '...' : '' }}</code>
              </button>
            </div>
          </div>
          <!-- Export All Formats dropdown -->
          <div class="export-dropdown-wrapper" @click.stop>
            <button class="btn btn-sm" @click="toggleExportDropdown" :title="t('exportDropdown')">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
              {{ t('exportDropdown') }}
              <svg width="8" height="8" viewBox="0 0 12 12" fill="currentColor" style="margin-left:3px;vertical-align:0px;"><path d="M2 4l4 4 4-4z"/></svg>
            </button>
            <div v-if="showExportDropdown" class="export-dropdown">
              <button class="export-dd-item" @click="saveFile(); closeExportDropdown()">{{ t('exportScad') }}</button>
              <div class="export-dd-sep"></div>
              <button class="export-dd-item" @click="doExportSTL(); closeExportDropdown()">{{ t('exportStl') }}</button>
              <button class="export-dd-item" @click="doExportOBJ(); closeExportDropdown()">{{ t('exportObj') }}</button>
              <button class="export-dd-item" @click="doExport3MF(); closeExportDropdown()">{{ t('export3mf') }}</button>
              <div class="export-dd-sep"></div>
              <button class="export-dd-item" @click="exportPng(1); closeExportDropdown()">{{ t('exportPng') }} (1x)</button>
              <button class="export-dd-item" @click="exportPng(2); closeExportDropdown()">{{ t('exportPng') }} (2x)</button>
              <button class="export-dd-item" @click="exportPng(4); closeExportDropdown()">{{ t('exportPng') }} (4x)</button>
              <div class="export-dd-sep"></div>
              <button class="export-dd-item" @click="copyCanvasToClipboard(); closeExportDropdown()">{{ t('copyImage') }}</button>
              <button class="export-dd-item" @click="shareLink(); closeExportDropdown()">{{ t('share') }}</button>
              <div class="export-dd-sep"></div>
              <div class="export-dd-sep"></div>
              <button class="export-dd-item" @click="doExportAllTabs(); closeExportDropdown()">{{ t('exportAllTabs') }}</button>
              <div class="export-dd-sep"></div>
              <button class="export-dd-item" @click="printCode(); closeExportDropdown()">{{ t('printCode') }}</button>
            </div>
          </div>
          <span class="spacer" />
          <!-- Examples gallery button (replaces inline example buttons) -->
          <button class="btn btn-sm" @click="openExampleGallery" :title="t('exampleGallery')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align: -1px; margin-right: 2px;">
              <rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/>
            </svg>
            {{ t('examples') }}
          </button>
        </div>

        <!-- Parameters panel -->
        <transition name="panel-slide">
          <div v-if="showParameters" class="params-panel">
            <div v-if="extractedParams.length === 0" class="params-empty">{{ t('noParameters') }}</div>
            <div v-for="p in extractedParams" :key="p.name" class="param-row">
              <span class="param-name">{{ p.name }}</span>
              <template v-if="p.type === 'slider'">
                <input
                  type="range"
                  class="param-slider"
                  :min="p.min"
                  :max="p.max"
                  :step="p.step"
                  :value="p.value"
                  @input="onParamChange(p, parseFloat(($event.target as HTMLInputElement).value))"
                />
                <span class="param-value">{{ p.value }}</span>
              </template>
              <template v-else-if="p.type === 'dropdown'">
                <select
                  class="param-select"
                  :value="p.value"
                  @change="onParamChange(p, ($event.target as HTMLSelectElement).value)"
                >
                  <option v-for="opt in p.options" :key="opt" :value="opt">{{ opt }}</option>
                </select>
              </template>
              <template v-else-if="p.type === 'checkbox'">
                <input
                  type="checkbox"
                  class="param-checkbox"
                  :checked="p.value === true"
                  @change="onParamChange(p, ($event.target as HTMLInputElement).checked)"
                />
                <span class="param-value">{{ p.value }}</span>
              </template>
            </div>
          </div>
        </transition>

        <!-- Tab bar -->
        <div class="tab-bar">
          <div
            v-for="tab in sortedTabs"
            :key="tab.id"
            class="tab-item"
            :class="{ active: tab.id === activeTabId, pinned: tab.pinned }"
            @click="switchTab(tab.id)"
            @dblclick.stop="startRenameTab(tab.id)"
            @contextmenu.prevent="onTabContextMenu($event, tab.id)"
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
              <svg v-if="tab.pinned" class="tab-pin-icon" width="10" height="10" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                <path d="M16 2l-4 4-6 1-3 3 5 5-6 7 7-6 5 5 3-3 1-6 4-4z"/>
              </svg>
              <span class="tab-name">{{ tab.name }}</span>
              <span v-if="tab.savedCode !== undefined && tab.savedCode !== tab.code" class="tab-unsaved" :title="t('unsaved')"></span>
              <button
                v-if="tabs.length > 1 && !tab.pinned"
                class="tab-close"
                @click.stop="closeTab(tab.id)"
                :title="t('closeTab')"
                :aria-label="t('closeTab')"
              >&times;</button>
            </template>
          </div>
          <button class="tab-add" @click="addTab" :title="t('newTab')" :aria-label="t('ariaNewTab')">+</button>
        </div>

        <!-- Tab context menu -->
        <Teleport to="body">
          <div
            v-if="showTabContextMenu"
            class="tab-ctx-menu"
            :style="{ left: tabContextMenuX + 'px', top: tabContextMenuY + 'px' }"
            @click.stop
            @contextmenu.prevent
          >
            <button class="tab-ctx-item" @click="togglePinTab(tabContextMenuId)">
              {{ tabs.find(tb => tb.id === tabContextMenuId)?.pinned ? t('unpinTab') : t('pinTab') }}
            </button>
            <button class="tab-ctx-item" @click="closeTab(tabContextMenuId); closeTabContextMenu()">{{ t('closeTab') }}</button>
            <button class="tab-ctx-item" @click="closeOtherTabs(tabContextMenuId)">{{ t('closeOtherTabs') }}</button>
          </div>
        </Teleport>

        <!-- Breadcrumb bar -->
        <div v-if="breadcrumbs.length > 0" class="breadcrumb-bar">
          <template v-for="(crumb, idx) in breadcrumbs" :key="idx">
            <span v-if="idx > 0" class="breadcrumb-sep">&rsaquo;</span>
            <span class="breadcrumb-item" @click="onBreadcrumbClick(crumb)" :title="crumb.label">{{ crumb.label }}</span>
          </template>
        </div>

        <!-- Find & Replace panel -->
        <transition name="panel-slide">
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
        </transition>

        <div class="editor-split-container" :class="{ 'split-active': splitMode }">
        <div class="code-editor" :class="{ 'word-wrap-on': wordWrap }" :style="{ '--editor-font-size': prefFontSize + 'px', '--editor-tab-size': prefTabSize, '--editor-font-family': prefFontFamily + ', monospace' }">
          <pre v-if="prefShowLineNumbers" class="line-numbers" ref="lineNumRef" aria-hidden="true" v-html="lineNumbers" @click="onLineNumClick"></pre>
          <div class="code-area">
            <pre class="highlight-layer" ref="highlightRef" aria-hidden="true" @click="onHighlightClick"><code v-html="highlightedCode"></code></pre>
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
              @mousemove="onEditorMouseMove"
              @mouseleave="onEditorMouseLeave"
            />
            <!-- Hover documentation tooltip -->
            <div
              v-if="hoverDocVisible"
              class="hover-doc-tooltip"
              :style="{ top: hoverDocY + 'px', left: hoverDocX + 'px' }"
            >
              <div class="hover-doc-sig">{{ hoverDocContent.sig }}</div>
              <div class="hover-doc-desc">{{ hoverDocContent.desc }}</div>
            </div>
            <!-- Color picker popup -->
            <div
              v-if="colorPickerVisible"
              class="color-picker-popup"
              :style="{ top: colorPickerY + 'px', left: colorPickerX + 'px' }"
              @click.stop
            >
              <div class="color-palette-grid">
                <button
                  v-for="(hex, name) in colorPaletteItems"
                  :key="name"
                  class="color-palette-swatch"
                  :style="{ background: hex }"
                  :title="name"
                  @click="insertColorName(name as string)"
                />
              </div>
              <div class="color-picker-row">
                <input
                  type="color"
                  :value="colorPickerValue"
                  @input="onColorPickerChange"
                  class="color-picker-input"
                />
                <button class="color-picker-close" @click="closeColorPicker">&times;</button>
              </div>
            </div>
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
          <button class="minimap-toggle" @click="toggleMinimap" :title="t('minimap')" :aria-label="t('ariaMinimap')" :class="{ active: showMinimap }">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="7" height="18" rx="1"/><line x1="14" y1="5" x2="21" y2="5"/><line x1="14" y1="9" x2="21" y2="9"/><line x1="14" y1="13" x2="19" y2="13"/><line x1="14" y1="17" x2="20" y2="17"/>
            </svg>
          </button>
        </div>

        <!-- Split editor right pane -->
        <div v-if="splitMode" class="split-editor-pane">
          <div class="split-pane-header">
            <select class="split-pane-select" v-model="rightTabId">
              <option v-for="tab in tabs" :key="tab.id" :value="tab.id">{{ tab.name }}</option>
            </select>
          </div>
          <div class="code-editor split-code-editor" :style="{ '--editor-font-size': prefFontSize + 'px', '--editor-tab-size': prefTabSize, '--editor-font-family': prefFontFamily + ', monospace' }">
            <div class="code-area">
              <textarea
                class="code"
                :value="rightCode"
                @input="onRightPaneInput"
                spellcheck="false"
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                @scroll="onRightPaneScroll"
              />
            </div>
          </div>
        </div>
        </div>

        <!-- History panel -->
        <transition name="panel-slide">
          <div v-if="showHistory" class="history-panel" role="region" :aria-label="t('ariaHistory')">
            <div class="history-header">
              <span class="history-title">{{ t('historyTitle') }}</span>
              <button class="history-close" @click="showHistory = false">&times;</button>
            </div>
            <div class="history-body">
              <!-- Timeline visualization -->
              <div v-if="currentTabHistory.length > 0" class="history-timeline">
                <div class="timeline-bar">
                  <div
                    v-for="(entry, idx) in currentTabHistory"
                    :key="'tl-' + entry.timestamp"
                    class="timeline-dot"
                    :class="{ 'timeline-dot-current': idx === 0 }"
                    :style="{ left: currentTabHistory.length > 1 ? ((currentTabHistory.length - 1 - idx) / (currentTabHistory.length - 1) * 100) + '%' : '50%' }"
                    :title="formatHistoryTime(entry.timestamp)"
                    @click="restoreHistoryEntry(entry)"
                  >
                    <span class="timeline-tooltip">{{ formatHistoryTime(entry.timestamp) }}</span>
                  </div>
                </div>
              </div>
              <div v-if="currentTabHistory.length === 0" class="history-empty">{{ t('historyEmpty') }}</div>
              <div
                v-for="(entry, idx) in currentTabHistory"
                :key="entry.timestamp"
                class="history-entry"
              >
                <div class="history-entry-info">
                  <span class="history-entry-time">{{ formatHistoryTime(entry.timestamp) }}</span>
                  <span class="history-entry-size">{{ t('historyEntry').replace('{n}', String(entry.code.length)) }}</span>
                </div>
                <div class="history-entry-preview">{{ entry.code.substring(0, 80) }}{{ entry.code.length > 80 ? '...' : '' }}</div>
                <button class="btn btn-sm history-restore-btn" @click="restoreHistoryEntry(entry)">{{ t('historyRestore') }}</button>
              </div>
            </div>
          </div>
        </transition>

        <!-- Console toggle button -->
        <button class="console-toggle-btn" v-show="!simpleMode" @click="showConsole = !showConsole" :aria-label="t('ariaConsole')" :class="{ active: showConsole }">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
          <span>{{ t('console') }}</span>
        </button>

        <!-- Console panel -->
        <transition name="console-slide">
        <div v-if="showConsole && !simpleMode" class="console-panel" :style="{ height: consolePanelHeight + 'px' }">
          <div class="console-drag-handle" @mousedown="onConsoleDragStart"></div>
          <div class="console-header">
            <span class="console-title">{{ t('console') }}</span>
            <button class="console-clear-btn" @click="clearConsole">{{ t('consoleClear') }}</button>
          </div>
          <div class="console-entries" ref="consoleRef">
            <div
              v-for="entry in consoleEntries"
              :key="entry.id"
              class="console-entry"
              :class="'console-' + entry.type"
            >
              <span class="console-time">{{ formatConsoleTime(entry.timestamp) }}</span>
              <span class="console-icon" v-if="entry.type === 'info'">&#9432;</span>
              <span class="console-icon" v-else-if="entry.type === 'warn'">&#9888;</span>
              <span class="console-icon" v-else>&#10006;</span>
              <span class="console-msg">{{ entry.message }}</span>
            </div>
            <div v-if="!consoleEntries.length" class="console-empty">--</div>
          </div>
        </div>
        </transition>

        <transition name="panel-slide">
          <div v-if="error" class="error">
            <div class="error-main">
              <span>{{ error }}</span>
              <button class="error-help-btn" @click="showErrorExplanation = !showErrorExplanation" :title="t('showErrorHelp')">?</button>
            </div>
            <div v-if="showErrorExplanation && getErrorExplanation(error)" class="error-explanation">
              <div class="error-expl-row"><strong>{{ t('errorExplanation') }}:</strong> {{ getErrorExplanation(error)!.explanation }}</div>
              <div class="error-expl-row"><strong>{{ t('errorFixHint') }}:</strong> {{ getErrorExplanation(error)!.fix }}</div>
            </div>
          </div>
        </transition>

        <div class="stats">
          <div class="stat-seg" :title="t('statMeshes')">
            <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2 2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
            </svg>
            <span class="stat-val">{{ fmtInt(meshCount) }}</span>
          </div>
          <span class="stat-div"></span>
          <div class="stat-seg" :title="t('statTriangles')">
            <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 3 22 20 2 20 12 3z"/>
            </svg>
            <span class="stat-val">{{ fmtInt(triCount) }}</span>
          </div>
          <span class="stat-div"></span>
          <div class="stat-seg" :title="t('statVertices')">
            <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="5" cy="5" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="12" cy="19" r="2"/><path d="M5 5 19 5 12 19 5 5z"/>
            </svg>
            <span class="stat-val">{{ fmtInt(vertexCount) }}</span>
          </div>
          <template v-if="renderTime > 0">
            <span class="stat-div"></span>
            <div class="stat-seg stat-render" :title="t('statRender')">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/>
              </svg>
              <span class="stat-val">{{ fmtInt(renderTime) }}ms</span>
            </div>
          </template>
          <span class="stat-div"></span>
          <div class="stat-seg stat-fps" :title="t('statFps')">
            <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>
            </svg>
            <span class="stat-val">{{ fpsVal }}</span>
          </div>
          <template v-if="boundsSize[0] > 0 || boundsSize[1] > 0 || boundsSize[2] > 0">
            <span class="stat-div"></span>
            <div class="stat-seg stat-size" :title="t('statSize')">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 16V8a2 2 0 0 0-1-1.7l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.7l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>
              </svg>
              <span class="stat-val">{{ formatNumber(boundsSize[0]) }}&times;{{ formatNumber(boundsSize[1]) }}&times;{{ formatNumber(boundsSize[2]) }}</span>
            </div>
          </template>
          <template v-if="selectionInfo">
            <span class="stat-div"></span>
            <div class="stat-seg stat-selection">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 4h6v2H6v4H4z"/><path d="M20 20h-6v-2h4v-4h2z"/>
              </svg>
              <span class="stat-val">{{ selectionInfo }}</span>
            </div>
          </template>
          <template v-if="wordWrap">
            <span class="stat-div"></span>
            <div class="stat-seg stat-wordwrap" :title="t('wordWrap')">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 6h18"/><path d="M3 12h15a3 3 0 1 1 0 6h-4"/><path d="m16 16-2 2 2 2"/><path d="M3 18h7"/>
              </svg>
            </div>
          </template>
          <!-- Code Stats -->
          <template v-if="!simpleMode">
            <span class="stat-div"></span>
            <div class="stat-seg stat-codestats" :title="t('codeStats')">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M16 18l6-6-6-6"/><path d="M8 6l-6 6 6 6"/>
              </svg>
              <span class="stat-val">{{ codeStatsData.lines }}{{ t('codeLines').charAt(0).toLowerCase() }} {{ codeStatsData.modules }}{{ t('codeModules').charAt(0).toLowerCase() }} {{ t('codeNestingDepth').charAt(0).toLowerCase() }}{{ codeStatsData.nestingDepth }}</span>
            </div>
          </template>
          <!-- Code Complexity -->
          <template v-if="!simpleMode">
            <span class="stat-div"></span>
            <div class="stat-seg stat-complexity" :title="t('complexity') + ': ' + t(codeComplexity.label) + ' (' + codeComplexity.lines + ' ' + t('codeLines') + ', ' + codeComplexity.modules + ' ' + t('codeModules') + ', ' + codeComplexity.forLoops + ' ' + t('codeForLoops') + ', ' + codeComplexity.csgOps + ' CSG, ' + t('codeNestingDepth') + ' ' + codeComplexity.maxDepth + ')'">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" :stroke="codeComplexity.color" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 20V10"/><path d="M18 20V4"/><path d="M6 20v-4"/>
              </svg>
              <span class="stat-val" :style="{ color: codeComplexity.color }">{{ t(codeComplexity.label) }}</span>
            </div>
          </template>
          <!-- Camera Info toggle -->
          <template v-if="!simpleMode">
            <span class="stat-div"></span>
            <div class="stat-seg stat-camera-toggle" :title="t('showCameraInfo')" @click="toggleCameraInfo" style="cursor:pointer;">
              <svg class="stat-ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2z"/><circle cx="12" cy="13" r="4"/>
              </svg>
              <span class="stat-val" :style="{ color: showCameraInfo ? 'var(--accent)' : '' }">{{ t('cameraInfo') }}</span>
            </div>
          </template>
          <span class="diff-note">{{ t('diff_note') }}</span>
        </div>
      </div>

      <div class="divider" v-show="!isFullscreen" @mousedown="onDividerDown"></div>

      <div class="canvas-panel">
        <div v-if="canvasGradient" class="canvas-gradient-bg" :style="{ background: canvasGradient }"></div>
        <canvas
          ref="canvasRef"
          class="gpu-canvas"
          :class="{ 'canvas-transparent': !!canvasGradient }"
          :style="{ filter: canvasFilter }"
          tabindex="0"
          @keydown="handleCanvasKeydown"
          @pointerdown="onCanvasPointerDown"
          @pointermove="onCanvasPointerMove"
          @pointerup="onCanvasPointerUp"
          @contextmenu="onCanvasContextMenu"
          @dragenter="onCanvasDragEnter"
          @dragover="onCanvasDragOver"
          @dragleave="onCanvasDragLeave"
          @drop="onCanvasDrop"
        />

        <!-- Vignette overlay -->
        <div v-if="vignetteEnabled" class="vignette-overlay"></div>

        <!-- STL drag-and-drop overlay on canvas -->
        <div v-if="isCanvasDragOver" class="canvas-drop-overlay">
          <div class="canvas-drop-content">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
            </svg>
            <span>{{ t('importStlDrop') }}</span>
          </div>
        </div>

        <!-- WebGPU loading overlay -->
        <div v-if="!rendererReady" class="webgpu-loading">
          <div class="webgpu-spinner"></div>
          <span class="webgpu-loading-text">{{ t('initWebGPU') }}</span>
        </div>

        <!-- Empty state (no geometry & no error) -->
        <div v-if="rendererReady && meshCount === 0 && !error" class="empty-state">
          <svg class="empty-state-icon" width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
          </svg>
          <span class="empty-state-text">{{ t('emptyState') }}</span>
        </div>

        <!-- 3D Annotations -->
        <div
          v-for="(ann, idx) in annotationScreenPositions"
          :key="'ann-' + idx"
          v-show="ann.visible"
          class="annotation-label"
          :style="{ left: ann.x + 'px', top: ann.y + 'px' }"
        >
          <div class="annotation-pin"></div>
          <div class="annotation-callout">{{ ann.text }}</div>
        </div>

        <!-- Viewport Watermark -->
        <div
          v-if="wmEnabled && wmText"
          class="viewport-watermark"
          :style="wmStyle"
        >{{ wmText }}</div>

        <!-- Measurement overlay -->
        <div v-if="measureMode" class="measure-overlay">
          <div class="measure-hint" v-if="!measurePoint1">{{ t('measureHint') }}</div>
          <div class="measure-result" v-if="measureDistance !== null">
            <span class="measure-dist">{{ measureDistance.toFixed(2) }} mm</span>
            <button class="measure-clear-btn" @click="clearMeasurement" :title="t('measureClear')">&times;</button>
          </div>
          <div class="measure-badge" v-if="measureMode">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M2 12h20"/><path d="M2 12l4-4m-4 4l4 4"/><path d="M22 12l-4-4m4 4l-4 4"/>
            </svg>
            {{ t('measureMode') }}
          </div>
        </div>

        <!-- Ghost tab selector (shown when ghost mode is active) -->
        <div v-if="ghostMode" class="ghost-controls">
          <label class="ghost-label">{{ t('ghostTab') }}:</label>
          <select class="ghost-select" v-model="ghostTabId" @change="updateGhostMeshes()">
            <option v-for="tab in tabs.filter(tb => tb.id !== activeTabId)" :key="tab.id" :value="tab.id">{{ tab.name }}</option>
          </select>
          <label class="ghost-label" style="margin-left:8px">{{ t('ghostOverlayMode') }}:</label>
          <select class="ghost-select" v-model="ghostOverlay" @change="updateGhostMeshes()">
            <option value="transparent">{{ t('ghostTransparent') }}</option>
            <option value="side-by-side">{{ t('ghostSideBySide') }}</option>
            <option value="difference">{{ t('ghostDifference') }}</option>
          </select>
        </div>

        <!-- Orientation cube / navigation gizmo -->
        <div class="nav-gizmo" :title="t('gizmoTip')" role="group" :aria-label="t('ariaGizmo')" :style="{ width: GIZMO_SIZE + 'px', height: GIZMO_SIZE + 'px' }">
          <svg class="nav-gizmo-svg" :viewBox="`0 0 ${GIZMO_SIZE} ${GIZMO_SIZE}`" :width="GIZMO_SIZE" :height="GIZMO_SIZE">
            <!-- axis lines from center -->
            <g v-for="ax in gizmoAxes" :key="ax.id + '-l'">
              <line
                v-if="ax.positive"
                :x1="GIZMO_SIZE / 2" :y1="GIZMO_SIZE / 2"
                :x2="ax.x" :y2="ax.y"
                :stroke="ax.color"
                stroke-width="2"
                stroke-linecap="round"
                :opacity="ax.z < 0 ? 0.95 : 0.45"
              />
            </g>
            <!-- axis end dots (clickable) -->
            <g v-for="ax in gizmoAxes" :key="ax.id + '-d'" class="gizmo-axis" @click="snapGizmoAxis(ax.id)">
              <circle
                :cx="ax.x" :cy="ax.y"
                :r="ax.positive ? 8.5 : 5"
                :fill="ax.positive ? ax.color : 'transparent'"
                :stroke="ax.color"
                stroke-width="1.5"
                :opacity="ax.z < 0 ? 1 : 0.5"
                class="gizmo-dot"
              />
              <text
                v-if="ax.label"
                :x="ax.x" :y="ax.y"
                text-anchor="middle"
                dominant-baseline="central"
                class="gizmo-text"
                :opacity="ax.z < 0 ? 1 : 0.6"
              >{{ ax.label }}</text>
            </g>
          </svg>
        </div>

        <!-- Viewport Compass -->
        <div class="viewport-compass" :title="t('compassN')">
          <svg width="60" height="60" viewBox="0 0 60 60">
            <g :transform="`rotate(${-compassYaw}, 30, 30)`">
              <circle cx="30" cy="30" r="26" fill="none" stroke="var(--border)" stroke-width="1.5" opacity="0.5"/>
              <circle cx="30" cy="30" r="2" fill="var(--text-dim)" />
              <!-- N marker (top) -->
              <line x1="30" y1="30" x2="30" y2="8" stroke="#f44336" stroke-width="2" stroke-linecap="round"/>
              <text x="30" y="7" text-anchor="middle" dominant-baseline="auto" fill="#f44336" font-size="9" font-weight="bold">{{ t('compassN') }}</text>
              <!-- S marker (bottom) -->
              <line x1="30" y1="30" x2="30" y2="52" stroke="var(--text-dim)" stroke-width="1.2" stroke-linecap="round"/>
              <text x="30" y="59" text-anchor="middle" dominant-baseline="auto" fill="var(--text-dim)" font-size="8">{{ t('compassS') }}</text>
              <!-- E marker (right) -->
              <line x1="30" y1="30" x2="52" y2="30" stroke="var(--text-dim)" stroke-width="1.2" stroke-linecap="round"/>
              <text x="58" y="33" text-anchor="middle" dominant-baseline="central" fill="var(--text-dim)" font-size="8">{{ t('compassE') }}</text>
              <!-- W marker (left) -->
              <line x1="30" y1="30" x2="8" y2="30" stroke="var(--text-dim)" stroke-width="1.2" stroke-linecap="round"/>
              <text x="2" y="33" text-anchor="middle" dominant-baseline="central" fill="var(--text-dim)" font-size="8">{{ t('compassW') }}</text>
            </g>
          </svg>
        </div>

        <!-- Right-click context menu -->
        <div
          v-if="showContextMenu"
          class="ctx-menu"
          :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
          @click.stop
          @contextmenu.prevent
        >
          <button
            v-for="item in contextMenuItems"
            :key="item.id"
            class="ctx-menu-item"
            @click="runContextMenuItem(item)"
          >
            <svg class="ctx-menu-ico" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path :d="item.icon" />
            </svg>
            <span>{{ item.label() }}</span>
          </button>
        </div>

        <!-- ── Viewport toolbar: 3 dropdown menus ── -->
        <div class="vp-toolbar">
          <!-- View menu -->
          <div class="vp-menu-wrapper" @click.stop>
            <button class="vp-menu-btn" @click="toggleViewMenu" :title="t('menuView')" :aria-label="t('menuView')" aria-haspopup="true" :aria-expanded="viewMenuOpen">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="viewMenuOpen" class="vp-dropdown" role="menu" @keydown="onMenuKeydown($event, 'view')">
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="setView('top'); closeAllMenus()">{{ t('top') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="setView('front'); closeAllMenus()">{{ t('front') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="setView('right'); closeAllMenus()">{{ t('right') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="setView('iso'); closeAllMenus()">{{ t('iso') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="setView('reset'); closeAllMenus()">{{ t('reset') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleProjection(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isOrthographic">&#10003;</span>
                {{ t('ortho') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleFullscreen(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isFullscreen">&#10003;</span>
                {{ t('fullscreen') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleInertia()">
                <span class="vp-dd-check" v-if="inertiaEnabled">&#10003;</span>
                {{ t('orbitInertia') }}
              </button>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <div class="vp-dd-label" v-show="!simpleMode">{{ t('fovSlider') }}: {{ fovDeg }}&deg;</div>
              <div v-show="!simpleMode" class="vp-dd-slider-row">
                <input type="range" class="clip-slider" min="15" max="120" step="1" :value="fovDeg" @input="onFovChange" />
                <span class="clip-value">{{ fovDeg }}&deg;</span>
              </div>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleMeasureMode(); closeAllMenus()">
                <span class="vp-dd-check" v-if="measureMode">&#10003;</span>
                {{ t('measureMode') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleGhostMode(); closeAllMenus()">
                <span class="vp-dd-check" v-if="ghostMode">&#10003;</span>
                {{ t('ghostCompare') }}
              </button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleFlyCamera(); closeAllMenus()">
                <span class="vp-dd-check" v-if="flyCameraMode">&#10003;</span>
                {{ t('flyCamera') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleZenMode(); closeAllMenus()">
                <span class="vp-dd-check" v-if="zenModeActive">&#10003;</span>
                {{ t('zenMode') }}
              </button>
              <div class="vp-dd-sep"></div>
              <div class="vp-dd-label">{{ t('bgColor') }}</div>
              <div class="vp-dd-swatches">
                <button
                  v-for="(c, idx) in bgColors"
                  :key="idx"
                  class="bg-swatch"
                  :class="{ active: idx === activeBg }"
                  :style="{ background: c.gradient || c.hex }"
                  :title="c.nameKey ? t(c.nameKey) : c.name"
                  @click="setBgColor(idx)"
                />
              </div>
              <div class="vp-dd-sep"></div>
              <div class="vp-dd-label">{{ t('bookmarks') }}</div>
              <div v-if="!cameraBookmarks.length" class="vp-dd-empty">{{ t('noBookmarks') }}</div>
              <div v-for="bk in cameraBookmarks" :key="bk.id" class="vp-dd-bookmark" :class="{ 'has-thumb': !!bk.thumbnail }">
                <button class="vp-dd-item vp-dd-bk-name" role="menuitem" tabindex="-1" @click="restoreBookmark(bk); closeAllMenus()">{{ bk.name }}</button>
                <button class="vp-dd-bk-del" @click.stop="deleteBookmark(bk.id)" :title="t('deleteBookmark')">&times;</button>
                <div v-if="bk.thumbnail" class="bk-thumb-popup">
                  <img :src="bk.thumbnail" class="bk-thumb-img" :alt="bk.name" />
                </div>
              </div>
            </div>
            </transition>
          </div>

          <!-- Render menu -->
          <div class="vp-menu-wrapper" @click.stop>
            <button class="vp-menu-btn" @click="toggleRenderMenu" :title="t('menuRender')" :aria-label="t('menuRender')" aria-haspopup="true" :aria-expanded="renderMenuOpen">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 16V8a2 2 0 0 0-1-1.7l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.7l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
                <path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="renderMenuOpen" class="vp-dropdown" role="menu" @keydown="onMenuKeydown($event, 'render')">
              <div class="vp-dd-label">{{ t('renderMode') }}</div>
              <select class="lighting-select vp-dd-select" :value="activeRenderMode" @change="setRenderMode(($event.target as HTMLSelectElement).value)">
                <option value="solid">{{ t('modeSolid') }}</option>
                <option value="solid+edges">{{ t('modeSolidEdges') }}</option>
                <option value="wireframe">{{ t('modeWireframe') }}</option>
                <option value="xray">{{ t('modeXray') }}</option>
                <option value="hidden-line">{{ t('modeHiddenLine') }}</option>
              </select>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleFlatShading()">
                <span class="vp-dd-check" v-if="flatShadingEnabled">&#10003;</span>
                {{ t('flatShading') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleGrid(); closeAllMenus()">
                <span class="vp-dd-check" v-if="showGrid">&#10003;</span>
                {{ t('grid') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleReflection(); closeAllMenus()">
                <span class="vp-dd-check" v-if="reflectionEnabled">&#10003;</span>
                {{ t('reflection') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleFog(); closeAllMenus()">
                <span class="vp-dd-check" v-if="fogEnabled">&#10003;</span>
                {{ t('fog') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleSSAO(); closeAllMenus()">
                <span class="vp-dd-check" v-if="ssaoEnabled">&#10003;</span>
                {{ t('ssao') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleOutline(); closeAllMenus()">
                <span class="vp-dd-check" v-if="outlineEnabled">&#10003;</span>
                {{ t('outline') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleNormalSmoothing(); closeAllMenus()">
                <span class="vp-dd-check" v-if="smoothNormalsEnabled">&#10003;</span>
                {{ t('smoothNormals') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleToonShading()">
                <span class="vp-dd-check" v-if="toonShadingEnabled">&#10003;</span>
                {{ t('toonShading') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleVignette()">
                <span class="vp-dd-check" v-if="vignetteEnabled">&#10003;</span>
                {{ t('vignette') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleGoochShading()">
                <span class="vp-dd-check" v-if="goochShadingEnabled">&#10003;</span>
                {{ t('goochShading') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleGroundShadow()">
                <span class="vp-dd-check" v-if="groundShadowEnabled">&#10003;</span>
                {{ t('groundShadow') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleBloom()">
                <span class="vp-dd-check" v-if="bloomEnabled">&#10003;</span>
                {{ t('bloom') }}
              </button>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <div class="vp-dd-label" v-show="!simpleMode">{{ t('explodedView') }}</div>
              <div v-show="!simpleMode" class="vp-dd-slider-row">
                <input type="range" class="clip-slider" min="0" max="1" step="0.05" :value="explodeFactorVal" @input="onExplodeChange" />
                <span class="clip-value">{{ explodeFactorVal.toFixed(2) }}</span>
              </div>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleClip()">
                <span class="vp-dd-check" v-if="clipEnabled">&#10003;</span>
                {{ t('clipPlane') }}
              </button>
              <div v-if="clipEnabled && !simpleMode" class="vp-dd-slider-row">
                <select class="clip-axis-select" :value="clipAxis" @change="onClipAxisChange">
                  <option :value="0">X</option>
                  <option :value="1">Y</option>
                  <option :value="2">Z</option>
                </select>
                <input type="range" class="clip-slider" :min="clipRange.min" :max="clipRange.max" step="0.5" :value="clipY" @input="onClipYChange" />
                <span class="clip-value">{{ clipY.toFixed(1) }}</span>
              </div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleSectionBox()">
                <span class="vp-dd-check" v-if="sectionBoxEnabled">&#10003;</span>
                {{ t('sectionBox') }}
              </button>
              <div v-if="sectionBoxEnabled && !simpleMode">
                <div class="vp-dd-slider-row">
                  <span class="clip-axis-label">X</span>
                  <input type="range" class="clip-slider" :min="sectionBoxRangeX.min" :max="sectionBoxRangeX.max" step="0.5" :value="sectionBoxX" @input="onSectionBoxChange(0, $event)" />
                  <span class="clip-value">{{ sectionBoxX.toFixed(1) }}</span>
                </div>
                <div class="vp-dd-slider-row">
                  <span class="clip-axis-label">Y</span>
                  <input type="range" class="clip-slider" :min="sectionBoxRangeY.min" :max="sectionBoxRangeY.max" step="0.5" :value="sectionBoxY" @input="onSectionBoxChange(1, $event)" />
                  <span class="clip-value">{{ sectionBoxY.toFixed(1) }}</span>
                </div>
                <div class="vp-dd-slider-row">
                  <span class="clip-axis-label">Z</span>
                  <input type="range" class="clip-slider" :min="sectionBoxRangeZ.min" :max="sectionBoxRangeZ.max" step="0.5" :value="sectionBoxZ" @input="onSectionBoxChange(2, $event)" />
                  <span class="clip-value">{{ sectionBoxZ.toFixed(1) }}</span>
                </div>
              </div>
              <div class="vp-dd-sep"></div>
              <div class="vp-dd-label">{{ t('lighting') }}</div>
              <select class="lighting-select vp-dd-select" :value="activeLighting" @change="setLighting(($event.target as HTMLSelectElement).value)">
                <option value="default">{{ t('lightDefault') }}</option>
                <option value="studio">{{ t('lightStudio') }}</option>
                <option value="outdoor">{{ t('lightOutdoor') }}</option>
                <option value="dramatic">{{ t('lightDramatic') }}</option>
                <option value="soft">{{ t('lightSoft') }}</option>
              </select>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <div class="vp-dd-label" v-show="!simpleMode">{{ t('colorGrading') }}</div>
              <select v-show="!simpleMode" class="lighting-select vp-dd-select" :value="colorGrading" @change="setColorGrading(($event.target as HTMLSelectElement).value)">
                <option value="none">{{ t('cgNone') }}</option>
                <option value="warm">{{ t('cgWarm') }}</option>
                <option value="cool">{{ t('cgCool') }}</option>
                <option value="vintage">{{ t('cgVintage') }}</option>
                <option value="noir">{{ t('cgNoir') }}</option>
                <option value="vivid">{{ t('cgVivid') }}</option>
              </select>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <div class="vp-dd-label" v-show="!simpleMode">{{ t('skyPreset') }}</div>
              <select v-show="!simpleMode" class="lighting-select vp-dd-select" :value="skyPreset" @change="setSkyPreset(($event.target as HTMLSelectElement).value)">
                <option value="none">{{ t('skyNone') }}</option>
                <option value="clearSky">{{ t('skyClearSky') }}</option>
                <option value="sunset">{{ t('skySunset') }}</option>
                <option value="studio">{{ t('skyStudio') }}</option>
                <option value="neutral">{{ t('skyNeutral') }}</option>
              </select>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleAutoRotate(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isAutoRotate">&#10003;</span>
                {{ t('autoRotate') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="toggleFastPreview()">
                <span class="vp-dd-check" v-if="fastPreviewMode">&#10003;</span>
                {{ t('fastPreview') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleBuildPlate()">
                <span class="vp-dd-check" v-if="buildPlateEnabled">&#10003;</span>
                {{ t('buildPlate') }}
              </button>
              <div v-if="buildPlateEnabled" class="vp-dd-dims">
                <label class="bed-dim">
                  <span class="bed-dim-label">{{ t('buildPlateX') }}</span>
                  <input type="number" class="bed-dim-input" min="1" step="10" v-model.number="buildPlateX" @change="onBuildPlateDimChange" />
                </label>
                <label class="bed-dim">
                  <span class="bed-dim-label">{{ t('buildPlateZ') }}</span>
                  <input type="number" class="bed-dim-input" min="1" step="10" v-model.number="buildPlateZ" @change="onBuildPlateDimChange" />
                </label>
              </div>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="showObjectTree = !showObjectTree; closeAllMenus()">
                <span class="vp-dd-check" v-if="showObjectTree">&#10003;</span>
                {{ t('objectTree') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="toggleStatistics(); closeAllMenus()">
                <span class="vp-dd-check" v-if="showStatistics">&#10003;</span>
                {{ t('statistics') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="showPerfPanel = !showPerfPanel; closeAllMenus()">
                <span class="vp-dd-check" v-if="showPerfPanel">&#10003;</span>
                {{ t('perfPanel') }}
              </button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" v-show="!simpleMode" @click="showProfilePanel = !showProfilePanel; closeAllMenus()">
                <span class="vp-dd-check" v-if="showProfilePanel">&#10003;</span>
                {{ t('profilePanel') }}
              </button>
            </div>
            </transition>
          </div>

          <!-- Export menu -->
          <div class="vp-menu-wrapper" @click.stop>
            <button class="vp-menu-btn" @click="toggleExportMenu" :title="t('menuExport')" :aria-label="t('menuExport')" aria-haspopup="true" :aria-expanded="exportMenuOpen">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="exportMenuOpen" class="vp-dropdown" role="menu" @keydown="onMenuKeydown($event, 'export')">
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="exportPng(1); closeAllMenus()">{{ t('screenshot') }} (1×)</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="exportPng(2); closeAllMenus()">{{ t('screenshot') }} (2×)</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="exportPng(4); closeAllMenus()">{{ t('screenshot') }} (4×)</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="copyCanvasToClipboard(); closeAllMenus()">{{ t('copyImage') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="doExportSTL(); closeAllMenus()">{{ t('exportStl') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="doExportOBJ(); closeAllMenus()">{{ t('exportObj') }}</button>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="doExport3MF(); closeAllMenus()">{{ t('export3mf') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="openImportSTL(); closeAllMenus()">{{ t('importStl') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="doExportAllTabs(); closeAllMenus()">{{ t('exportAllTabs') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" @click="printCode(); closeAllMenus()">{{ t('printCode') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" role="menuitem" tabindex="-1" :disabled="turntableExporting" @click="exportTurntableZip(); closeAllMenus()">{{ t('exportTurntable') }}</button>
            </div>
            </transition>
          </div>
        </div>

        <!-- Zoom controls (kept as direct overlays) -->
        <div class="vp-zoom-controls">
          <button class="view-btn zoom-btn" @click="doZoomIn" :title="t('zoomIn')" :aria-label="t('zoomIn')">+</button>
          <button class="view-btn zoom-btn" @click="doZoomOut" :title="t('zoomOut')" :aria-label="t('zoomOut')">&minus;</button>
          <!-- Save camera bookmark -->
          <button class="view-btn zoom-btn vp-bookmark-btn" @click="saveCameraBookmark" :title="t('saveBookmark')" :aria-label="t('saveBookmark')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
            </svg>
          </button>
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

        <!-- Object Tree Panel (overlay left of canvas) -->
        <transition name="overlay-fade">
        <div v-if="showObjectTree" class="object-tree-panel">
          <div class="object-tree-header">
            <span class="object-tree-title">{{ t('objectTree') }}</span>
            <button class="object-tree-close" @click="showObjectTree = false">&times;</button>
          </div>
          <div class="object-tree-body">
            <div
              v-for="item in flatTree"
              :key="item.node.pos"
              class="tree-node"
              :style="{ paddingLeft: (item.depth * 16 + 8) + 'px' }"
              @click="scrollEditorToLine(item.node.pos)"
            >
              <span
                v-if="item.hasChildren"
                class="tree-toggle"
                @click.stop="toggleTreeNode(item.node.pos)"
              >{{ expandedNodes.has(item.node.pos) ? '&#9662;' : '&#9656;' }}</span>
              <span v-else class="tree-toggle tree-leaf">&nbsp;</span>
              <span class="tree-icon">{{ getNodeIcon(item.node.name) }}</span>
              <span class="tree-name">{{ item.node.name }}</span>
              <span v-if="getNodeSummary(item.node)" class="tree-summary">{{ getNodeSummary(item.node) }}</span>
              <span class="tree-actions" v-if="item.meshIndex >= 0">
                <button class="tree-eye-btn" :class="{ 'tree-eye-hidden': !isMeshVisible(item.meshIndex) }" @click.stop="toggleMeshVisibility(item.meshIndex)" :title="isMeshVisible(item.meshIndex) ? t('meshVisible') : t('meshHidden')">
                  <svg v-if="isMeshVisible(item.meshIndex)" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  <svg v-else width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                </button>
                <input type="color" class="tree-color-swatch" :value="getMeshColorHex(item.meshIndex)" @input="setObjectColorOverride(item.meshIndex, ($event.target as HTMLInputElement).value)" @click.stop :title="t('colorOverride')" />
              </span>
            </div>
            <div v-if="!flatTree.length" class="object-tree-empty">--</div>
          </div>
        </div>
        </transition>

        <!-- Statistics Panel (overlay) -->
        <transition name="overlay-fade">
        <div v-if="showStatistics" class="stats-panel" role="region" :aria-label="t('ariaStatistics')">
          <div class="stats-panel-header">
            <span class="stats-panel-title">{{ t('statistics') }}</span>
            <button class="stats-panel-close" @click="showStatistics = false" :aria-label="t('ariaCloseModal')">&times;</button>
          </div>
          <div class="stats-panel-body">
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('statsTriangles') }}</span>
              <span class="stats-panel-val">{{ fmtInt(triCount) }}</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('statsVertices') }}</span>
              <span class="stats-panel-val">{{ fmtInt(vertexCount) }}</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('statsVolume') }}</span>
              <span class="stats-panel-val">{{ formatNumber(statsVolume) }} mm³</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('statsSurfaceArea') }}</span>
              <span class="stats-panel-val">{{ formatNumber(statsSurfaceArea) }} mm²</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('statsBoundingBox') }}</span>
              <span class="stats-panel-val">{{ formatNumber(boundsSize[0]) }}&times;{{ formatNumber(boundsSize[1]) }}&times;{{ formatNumber(boundsSize[2]) }} mm</span>
            </div>
            <div class="stats-panel-row" style="margin-top:6px;border-top:1px solid var(--border);padding-top:6px">
              <span class="stats-panel-key">{{ t('material') }}</span>
              <span class="stats-panel-val">
                <select v-model="selectedMaterial" style="background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:3px;padding:1px 4px;font-size:11px">
                  <option v-for="mat in MATERIALS" :key="mat.id" :value="mat.id">{{ t(mat.nameKey) }}</option>
                </select>
              </span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('weight') }}</span>
              <span class="stats-panel-val">{{ formatNumber(estimatedWeight, 2) }} g</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('cost') }}</span>
              <span class="stats-panel-val">
                {{ currencySymbol }}{{ formatNumber(estimatedCost, 2) }}
                <select v-model="selectedCurrency" style="background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:3px;padding:1px 4px;font-size:11px;margin-left:4px">
                  <option v-for="c in CURRENCIES" :key="c" :value="c">{{ c }}</option>
                </select>
              </span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('costPerKg') }}</span>
              <span class="stats-panel-val">
                {{ currencySymbol }}<input type="number" v-model.number="materialCostPerKg" min="0" step="1" style="width:50px;background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:3px;padding:1px 4px;font-size:11px;text-align:right">
              </span>
            </div>
          </div>
        </div>
        </transition>

        <!-- Ghost Comparison Stats Panel -->
        <transition name="overlay-fade">
        <div v-if="ghostMode" class="ghost-stats-panel" role="region">
          <div class="stats-panel-header">
            <span class="stats-panel-title">{{ t('ghostStats') }}</span>
          </div>
          <div class="stats-panel-body">
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('ghostDeltaTriangles') }}</span>
              <span class="stats-panel-val" :class="{'ghost-stat-pos': triCount - ghostTriCount > 0, 'ghost-stat-neg': triCount - ghostTriCount < 0}">{{ (triCount - ghostTriCount) > 0 ? '+' : '' }}{{ fmtInt(triCount - ghostTriCount) }}</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('ghostDeltaVolume') }}</span>
              <span class="stats-panel-val" :class="{'ghost-stat-pos': statsVolume - ghostVolume > 0, 'ghost-stat-neg': statsVolume - ghostVolume < 0}">{{ (statsVolume - ghostVolume) > 0 ? '+' : '' }}{{ formatNumber(statsVolume - ghostVolume) }} mm&sup3;</span>
            </div>
            <div class="stats-panel-row">
              <span class="stats-panel-key">{{ t('ghostDeltaSize') }}</span>
              <span class="stats-panel-val">{{ formatNumber(boundsSize[0] - ghostBoundsSize[0]) }}&times;{{ formatNumber(boundsSize[1] - ghostBoundsSize[1]) }}&times;{{ formatNumber(boundsSize[2] - ghostBoundsSize[2]) }}</span>
            </div>
          </div>
        </div>
        </transition>

        <!-- Performance Panel (Feature 5) -->
        <transition name="overlay-fade">
        <div v-if="showPerfPanel" class="perf-panel" role="region">
          <div class="perf-panel-header">
            <span class="perf-panel-title">{{ t('perfPanel') }}</span>
            <button class="perf-panel-close" @click="showPerfPanel = false">&times;</button>
          </div>
          <div class="perf-panel-body">
            <div class="perf-section-title">Timing</div>
            <div class="perf-row"><span class="perf-key">{{ t('perfParseTime') }}</span><span class="perf-val">{{ fmtInt(perfParseTime) }}ms</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfGpuUploadTime') }}</span><span class="perf-val">{{ fmtInt(perfGpuUploadTime) }}ms</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('statRender') }}</span><span class="perf-val">{{ fmtInt(renderTime) }}ms</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('statFps') }}</span><span class="perf-val">{{ fmtInt(fpsVal) }}</span></div>
            <div class="perf-section-title">Geometry</div>
            <div class="perf-row"><span class="perf-key">{{ t('perfTriangles') }}</span><span class="perf-val">{{ fmtInt(triCount) }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfVertices') }}</span><span class="perf-val">{{ fmtInt(vertexCount) }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfVertexBuffer') }}</span><span class="perf-val">{{ perfBufferStats.totalVertexBytes ? (perfBufferStats.totalVertexBytes / 1024).toFixed(1) + ' KB' : '0' }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfIndexBuffer') }}</span><span class="perf-val">{{ perfBufferStats.totalIndexBytes ? (perfBufferStats.totalIndexBytes / 1024).toFixed(1) + ' KB' : '0' }}</span></div>
            <div class="perf-section-title">GPU</div>
            <div class="perf-row"><span class="perf-key">{{ t('perfGpuAdapter') }}</span><span class="perf-val perf-val-sm">{{ perfDeviceInfo.adapter || 'N/A' }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfGpuVendor') }}</span><span class="perf-val perf-val-sm">{{ perfDeviceInfo.vendor || 'N/A' }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfMaxBuffer') }}</span><span class="perf-val">{{ perfDeviceInfo.maxBufferSize ? (perfDeviceInfo.maxBufferSize / (1024*1024)).toFixed(0) + ' MB' : 'N/A' }}</span></div>
            <div class="perf-row"><span class="perf-key">{{ t('perfMaxTexture') }}</span><span class="perf-val">{{ perfDeviceInfo.maxTextureSize || 'N/A' }}px</span></div>
          </div>
        </div>
        </transition>

        <!-- Snapshot Gallery panel -->
        <div v-if="showSnapshotGallery" class="snapshot-gallery-panel" role="region" :aria-label="t('ariaSnapshotGallery')">
          <div class="perf-panel-header">
            <span class="perf-panel-title">{{ t('snapshotGallery') }}</span>
            <button class="perf-panel-close" @click="showSnapshotGallery = false">&times;</button>
          </div>
          <div class="snapshot-gallery-body">
            <div v-if="!snapshots.length" class="snapshot-empty">{{ t('noSnapshots') }}</div>
            <div v-else class="snapshot-grid">
              <div v-for="snap in [...snapshots].reverse()" :key="snap.id" class="snapshot-item">
                <img :src="snap.dataUrl" class="snapshot-thumb" @click="snapshotPreview = snap.dataUrl" :alt="formatSnapshotTime(snap.timestamp)" />
                <div class="snapshot-meta">
                  <span class="snapshot-time">{{ formatSnapshotTime(snap.timestamp) }}</span>
                  <button class="snapshot-del" @click="deleteSnapshot(snap.id)" :title="t('deleteSnapshot')">&times;</button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Snapshot Preview modal -->
        <Teleport to="body">
          <div v-if="snapshotPreview" class="modal-backdrop" @click.self="snapshotPreview = null">
            <div class="snapshot-preview-modal">
              <button class="modal-close snapshot-preview-close" @click="snapshotPreview = null">&times;</button>
              <img :src="snapshotPreview" class="snapshot-preview-img" alt="Snapshot preview" />
            </div>
          </div>
        </Teleport>

        <!-- Profile panel -->
        <div v-if="showProfilePanel" class="profile-panel" role="region">
          <div class="perf-panel-header">
            <span class="perf-panel-title">{{ t('profilePanel') }}</span>
            <button class="perf-panel-close" @click="showProfilePanel = false">&times;</button>
          </div>
          <div class="profile-panel-body">
            <div v-if="!sortedProfileEntries.length" class="snapshot-empty">{{ t('noProfileData') }}</div>
            <table v-else class="profile-table">
              <thead>
                <tr>
                  <th>{{ t('profileNode') }}</th>
                  <th>{{ t('profileLine') }}</th>
                  <th>{{ t('profileTime') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(entry, idx) in sortedProfileEntries" :key="idx" class="profile-row" @click="jumpToProfileLine(entry.line)">
                  <td class="profile-name">{{ entry.name }}</td>
                  <td class="profile-line">{{ entry.line }}</td>
                  <td class="profile-time" :class="{ 'profile-slow': entry.timeMs > 10 }">{{ entry.timeMs.toFixed(2) }}ms</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Device Lost overlay (Feature 3) -->
        <div v-if="deviceLost" class="device-lost-overlay">
          <div class="device-lost-content">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <p class="device-lost-msg">{{ t('deviceLostMsg') }}</p>
            <button class="btn btn-primary" @click="reinitializeWebGPU">{{ t('reinitialize') }}</button>
          </div>
        </div>

        <!-- Animation Timeline -->
        <div class="anim-timeline">
          <button class="anim-play-btn" @click="toggleAnimation" :title="animPlaying ? t('animPause') : t('animPlay')" :aria-label="t('ariaAnimPlay')">
            <svg v-if="!animPlaying" width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
            <svg v-else width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="3" width="6" height="18"/><rect x="14" y="3" width="6" height="18"/></svg>
          </button>
          <span class="anim-label">$t</span>
          <input
            type="range"
            class="anim-slider"
            min="0"
            max="1"
            step="0.001"
            :value="animT"
            @input="onAnimSliderInput"
          />
          <span class="anim-value">{{ animT.toFixed(3) }}</span>
          <label class="anim-dur-label">
            <span class="anim-dur-text">{{ t('animDuration') }}:</span>
            <input
              type="number"
              class="anim-dur-input"
              v-model.number="animDuration"
              min="0.5"
              max="60"
              step="0.5"
            />
            <span class="anim-dur-unit">s</span>
          </label>
        </div>

        <!-- Camera Info Overlay (bottom-left) -->
        <transition name="overlay-fade">
        <div v-if="showCameraInfo" class="camera-info-overlay">
          <div class="camera-info-title">{{ t('cameraInfo') }}</div>
          <div class="camera-info-row">
            <span class="camera-info-key">{{ t('cameraYaw') }}</span>
            <span class="camera-info-val">{{ formatNumber(cameraInfoData.yaw) }}&deg;</span>
          </div>
          <div class="camera-info-row">
            <span class="camera-info-key">{{ t('cameraPitch') }}</span>
            <span class="camera-info-val">{{ formatNumber(cameraInfoData.pitch) }}&deg;</span>
          </div>
          <div class="camera-info-row">
            <span class="camera-info-key">{{ t('cameraDist') }}</span>
            <span class="camera-info-val">{{ formatNumber(cameraInfoData.dist) }}</span>
          </div>
          <div class="camera-info-row">
            <span class="camera-info-key">{{ t('cameraTarget') }}</span>
            <span class="camera-info-val">{{ formatNumber(cameraInfoData.tx) }}, {{ formatNumber(cameraInfoData.ty) }}, {{ formatNumber(cameraInfoData.tz) }}</span>
          </div>
        </div>
        </transition>

        <!-- Reset View FAB for mobile -->
        <button class="mobile-reset-fab" @click="setView('reset')" :title="t('resetView')" :aria-label="t('resetView')">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9 9 9 0 0 0-6.4 2.6L3 8"/><path d="M3 3v5h5"/>
          </svg>
        </button>

        <div class="canvas-hint">{{ t('hint') }}</div>
      </div>
    </div>

    <!-- Session Restore Dialog -->
    <transition name="overlay-fade">
    <div v-if="showSessionRestore" class="session-restore-overlay">
      <div class="session-restore-dialog">
        <div class="session-restore-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9 9 9 0 0 0-6.4 2.6L3 8"/><path d="M3 3v5h5"/>
          </svg>
        </div>
        <p class="session-restore-msg">{{ t('sessionRestoreMsg') }}</p>
        <div class="session-restore-actions">
          <button class="btn btn-sm session-restore-yes" @click="restoreSession">{{ t('sessionRestoreYes') }}</button>
          <button class="btn btn-sm session-restore-no" @click="dismissSessionRestore">{{ t('sessionRestoreNo') }}</button>
        </div>
      </div>
    </div>
    </transition>

    <!-- Toast Notifications -->
    <div class="toast-container">
      <transition-group name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast-item"
          :class="'toast-' + toast.type"
        >
          <span class="toast-icon" v-if="toast.type === 'success'">&#10003;</span>
          <span class="toast-icon" v-else-if="toast.type === 'error'">&#10006;</span>
          <span class="toast-icon" v-else>&#9432;</span>
          <span class="toast-msg">{{ toast.message }}</span>
          <button
            v-if="toast.actionLabel"
            class="toast-action"
            @click="runToastAction(toast)"
          >{{ toast.actionLabel }}</button>
        </div>
      </transition-group>
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

  /* ── Design tokens (shared design-system scales) ──
   * Consolidated spacing / radius / font-size scales. These are theme-agnostic
   * (independent of light/dark) and are applied in prominent components
   * (toolbar, buttons, panels, modals, toasts) to keep spacing consistent.
   * Groundwork: not every rule is refactored to use them yet. */
  --sp-1: 2px;
  --sp-2: 4px;
  --sp-3: 8px;
  --sp-4: 12px;
  --sp-5: 16px;
  --sp-6: 24px;

  --r-sm: 5px;
  --r-md: 8px;
  --r-lg: 14px;

  --fz-xs: 0.72rem;
  --fz-sm: 0.8rem;
  --fz-md: 0.95rem;
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

/* ── Accessibility: keyboard focus ring ──
 * :focus-visible (not :focus) so the ring shows only for keyboard navigation,
 * never on plain mouse clicks. Applies globally to interactive elements,
 * including teleported modals (palette, prefs, go-to-line). */
button:focus-visible,
input:focus-visible,
select:focus-visible,
textarea:focus-visible,
a:focus-visible,
[tabindex]:focus-visible,
.tab-item:focus-visible,
.theme-option:focus-visible,
.command-palette-item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: 4px;
}
/* Suppress the default outline only when focus-visible is NOT in play
   (i.e. mouse focus), so the focus ring above remains the single source. */
button:focus:not(:focus-visible),
input:focus:not(:focus-visible),
select:focus:not(:focus-visible),
textarea:focus:not(:focus-visible),
a:focus:not(:focus-visible) {
  outline: none;
}
/* The code editor textarea keeps its seamless (outline-free) appearance —
   it overlays the highlight layer and is the editor's default focus target. */
textarea.code:focus-visible {
  outline: none;
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
  padding: var(--sp-2) 10px; border-radius: var(--r-sm); border: 1px solid var(--border);
  background: var(--surface); color: var(--text); cursor: pointer; font-size: var(--fz-sm);
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
  display: flex; align-items: center; gap: var(--sp-3); padding: var(--sp-3) var(--sp-4);
  border-bottom: 1px solid var(--border); flex-wrap: wrap;
}
.btn {
  padding: 5px var(--sp-4); border-radius: var(--r-md); border: 1px solid var(--border);
  background: var(--surface); color: var(--text); cursor: pointer; font-size: var(--fz-sm);
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
  width: 52px; flex-shrink: 0;
  padding: 12px 8px 12px 4px;
  font-family: var(--editor-font-family, 'JetBrains Mono', monospace);
  font-size: var(--editor-font-size, 0.82rem); line-height: 1.55;
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
  font-family: var(--editor-font-family, 'JetBrains Mono', monospace);
  font-size: var(--editor-font-size, 0.82rem); line-height: 1.55;
  color: var(--text);
  overflow: hidden;
  pointer-events: none;
  white-space: pre;
  tab-size: var(--editor-tab-size, 4);
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
  font-family: var(--editor-font-family, 'JetBrains Mono', monospace);
  font-size: var(--editor-font-size, 0.82rem); line-height: 1.55;
  padding: 12px; border: none; outline: none;
  background: transparent;
  color: transparent;
  caret-color: var(--text);
  tab-size: var(--editor-tab-size, 4);
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

/* ── View preset buttons (base styles reused by zoom) ── */
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
  border-radius: var(--r-lg);
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

/* ── Breadcrumb bar ── */
.breadcrumb-bar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px 10px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  color: var(--text-dim);
  flex-shrink: 0;
  overflow-x: auto;
  white-space: nowrap;
  min-height: 22px;
}
.breadcrumb-bar::-webkit-scrollbar { display: none; }
.breadcrumb-item {
  cursor: pointer;
  padding: 1px 4px;
  border-radius: 3px;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background 0.15s, color 0.15s;
}
.breadcrumb-item:hover {
  background: var(--hover);
  color: var(--text);
}
.breadcrumb-sep {
  color: var(--text-dim);
  opacity: 0.5;
  font-size: 13px;
  user-select: none;
  padding: 0 1px;
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
:deep(.occ-match) {
  background: rgba(74, 158, 255, 0.18);
  border-radius: 2px;
  outline: 1px solid rgba(74, 158, 255, 0.35);
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

/* ── Gear button ── */
.tb-btn-gear {
  display: inline-flex; align-items: center; justify-content: center;
  padding: 4px 7px;
}

/* ── Preferences modal ── */
.pref-modal { min-width: 360px; }
.pref-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 0;
  font-size: 0.82rem;
  border-bottom: 1px solid rgba(128,128,128,.12);
}
.pref-row:last-child { border-bottom: none; }
.pref-label { color: var(--text); font-weight: 500; }
.pref-control { display: flex; align-items: center; gap: 8px; }
.pref-slider {
  width: 120px; cursor: pointer;
  accent-color: var(--accent);
}
.pref-value {
  font-size: 0.72rem; color: var(--text-dim);
  min-width: 44px; text-align: right;
  font-family: 'JetBrains Mono', monospace;
}
.pref-radio-group { display: flex; gap: 12px; }
.pref-radio {
  display: flex; align-items: center; gap: 4px;
  font-size: 0.8rem; color: var(--text); cursor: pointer;
}
.pref-radio input { accent-color: var(--accent); cursor: pointer; }
.pref-checkbox { accent-color: var(--accent); cursor: pointer; width: 16px; height: 16px; }
.pref-footer {
  display: flex; justify-content: flex-end;
  padding-top: var(--sp-4);
  margin-top: var(--sp-2);
  border-top: 1px solid rgba(128,128,128,.12);
}
.pref-reset-btn {
  color: var(--danger);
  border-color: rgba(231,76,60,.4);
}
.pref-reset-btn:hover {
  background: rgba(231,76,60,.12);
  border-color: var(--danger);
}

/* ── Console panel ── */
.console-toggle-btn {
  display: flex; align-items: center; gap: 4px;
  background: none; border: 1px solid var(--border); color: var(--text-dim);
  font-size: 0.68rem; padding: 2px 8px; border-radius: 4px; cursor: pointer;
  transition: color 0.1s, border-color 0.1s;
  flex-shrink: 0;
}
.console-toggle-btn:hover { color: var(--text); }
.console-toggle-btn.active { color: var(--accent); border-color: var(--accent); }

.console-panel {
  border-top: 1px solid var(--border);
  background: var(--bg);
  display: flex; flex-direction: column;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
}
.console-drag-handle {
  height: 4px; cursor: ns-resize;
  background: var(--border);
  transition: background 0.12s;
  flex-shrink: 0;
}
.console-drag-handle:hover { background: var(--accent); }
.console-header {
  display: flex; align-items: center; gap: 8px;
  padding: 4px 10px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.console-title { font-size: 0.72rem; font-weight: 600; color: var(--text-dim); flex: 1; }
.console-clear-btn {
  background: none; border: 1px solid var(--border); color: var(--text-dim); cursor: pointer;
  font-size: 0.66rem; padding: 1px 8px; border-radius: 3px;
  transition: color 0.1s;
}
.console-clear-btn:hover { color: var(--text); }
.console-entries {
  flex: 1; overflow-y: auto; padding: 4px 0;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
}
.console-entry {
  display: flex; align-items: flex-start; gap: 6px;
  padding: 2px 10px;
  border-bottom: 1px solid rgba(128,128,128,.06);
}
.console-time { color: var(--text-dim); white-space: nowrap; opacity: 0.6; flex-shrink: 0; }
.console-icon { flex-shrink: 0; font-size: 0.72rem; line-height: 1.3; }
.console-info .console-icon { color: var(--accent); }
.console-warn .console-icon { color: var(--hl-number); }
.console-error .console-icon { color: var(--danger); }
.console-msg { color: var(--text); word-break: break-word; line-height: 1.4; }
.console-error .console-msg { color: var(--danger); }
.console-warn .console-msg { color: var(--hl-number); }
.console-empty { padding: 12px; color: var(--text-dim); text-align: center; font-size: 0.7rem; }

/* ── Copy as Image ── */
.copy-image-wrapper {
  position: relative;
  display: inline-flex;
}
.copy-image-wrapper .copied-tooltip {
  top: auto;
  left: auto;
  right: 110%;
  bottom: auto;
  transform: none;
  white-space: nowrap;
}

/* ── Object Tree Panel ── */
.object-tree-panel {
  position: absolute;
  top: 10px; left: 10px;
  width: 240px;
  max-height: 60%;
  background: rgba(30,30,34,.88);
  border: 1px solid rgba(255,255,255,.12);
  border-radius: 10px;
  backdrop-filter: blur(8px);
  display: flex; flex-direction: column;
  z-index: 10;
  overflow: hidden;
}
[data-theme="light"] .object-tree-panel {
  background: rgba(255,255,255,.88);
  border-color: rgba(0,0,0,.12);
}
.object-tree-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(255,255,255,.08);
  flex-shrink: 0;
}
[data-theme="light"] .object-tree-header {
  border-bottom-color: rgba(0,0,0,.08);
}
.object-tree-title { font-size: 0.72rem; font-weight: 600; color: var(--text-dim); }
.object-tree-close {
  background: none; border: none; color: var(--text-dim); cursor: pointer;
  font-size: 1rem; line-height: 1; padding: 0 2px;
}
.object-tree-close:hover { color: var(--text); }
.object-tree-body {
  flex: 1; overflow-y: auto;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  padding: 4px 0;
}
.tree-node {
  display: flex; align-items: center; gap: 4px;
  padding: 3px 8px; cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s;
  border-radius: 4px;
  margin: 0 4px;
}
.tree-node:hover { background: rgba(74,158,255,.15); }
.tree-toggle {
  width: 12px; text-align: center; cursor: pointer;
  color: var(--text-dim); flex-shrink: 0; font-size: 0.65rem;
}
.tree-leaf { visibility: hidden; }
.tree-icon { color: var(--accent); flex-shrink: 0; font-size: 0.72rem; }
.tree-name { color: var(--hl-keyword); font-weight: 500; }
.tree-summary { color: var(--text-dim); margin-left: 4px; font-size: 0.65rem; }
.object-tree-empty { padding: 12px; color: var(--text-dim); text-align: center; font-size: 0.7rem; }

/* ── Command Palette ── */
.command-palette-backdrop {
  position: fixed; inset: 0;
  background: rgba(0,0,0,.45);
  display: flex; align-items: flex-start; justify-content: center;
  padding-top: 15vh;
  z-index: 10000;
  backdrop-filter: blur(6px);
}
.command-palette {
  width: 480px; max-width: 90vw;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0,0,0,.45);
  overflow: hidden;
}
.command-palette-input-wrapper {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
}
.command-palette-icon {
  flex-shrink: 0; color: var(--text-dim);
}
.command-palette-input {
  flex: 1; border: none; outline: none;
  background: transparent; color: var(--text);
  font-size: 0.88rem;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
.command-palette-input::placeholder { color: var(--text-dim); }
.command-palette-list {
  max-height: 320px; overflow-y: auto;
  padding: 4px 0;
}
.command-palette-item {
  display: flex; align-items: center; justify-content: space-between;
  padding: 7px 14px;
  cursor: pointer;
  font-size: 0.82rem;
  color: var(--text);
  transition: background 0.08s;
}
.command-palette-item:hover,
.command-palette-item.active {
  background: rgba(74,158,255,.15);
}
.command-palette-label { flex: 1; }
.command-palette-shortcut {
  background: var(--hover);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 7px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.68rem;
  color: var(--text-dim);
  margin-left: 12px;
  white-space: nowrap;
}
.command-palette-empty {
  padding: 16px;
  text-align: center;
  font-size: 0.78rem;
  color: var(--text-dim);
}

/* ── Go to Line ── */
.go-to-line-backdrop {
  position: fixed; inset: 0;
  background: rgba(0,0,0,.3);
  display: flex; align-items: flex-start; justify-content: center;
  padding-top: 15vh;
  z-index: 10000;
  backdrop-filter: blur(4px);
}
.go-to-line-box {
  width: 320px; max-width: 85vw;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 36px rgba(0,0,0,.4);
  padding: 6px;
  overflow: hidden;
}
.go-to-line-input {
  width: 100%;
  border: none; outline: none;
  background: transparent; color: var(--text);
  font-size: 0.88rem;
  padding: 8px 10px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  -moz-appearance: textfield;
}
.go-to-line-input::-webkit-inner-spin-button,
.go-to-line-input::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.go-to-line-input::placeholder { color: var(--text-dim); }

/* ── Word Wrap ── */
.word-wrap-on .code {
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: hidden;
}
.word-wrap-on .highlight-layer {
  white-space: pre-wrap;
  word-break: break-all;
}

/* ── Selection Info ── */
.stat-selection {
  color: var(--accent);
  font-weight: 500;
}
.stat-wordwrap {
  color: var(--hl-special);
}

/* ── Indent Guides ── */
:deep(.indent-line) {
  position: relative;
}
:deep(.indent-guide) {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 0;
  border-left: 1px solid var(--border);
  opacity: 0.4;
  pointer-events: none;
}

/* ── Parameters panel ── */
.params-panel {
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  padding: 6px 12px;
  max-height: 180px;
  overflow-y: auto;
  flex-shrink: 0;
}
.params-empty {
  font-size: 0.72rem;
  color: var(--text-dim);
  text-align: center;
  padding: 8px;
}
.param-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
}
.param-name {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.72rem;
  color: var(--hl-keyword);
  min-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.param-slider {
  flex: 1;
  accent-color: var(--accent);
  cursor: pointer;
  height: 4px;
}
.param-value {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  color: var(--hl-number);
  min-width: 50px;
  text-align: right;
}
.param-select {
  flex: 1;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 2px 6px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.72rem;
  cursor: pointer;
}
.param-select:focus { border-color: var(--accent); outline: none; }
.param-checkbox {
  accent-color: var(--accent);
  cursor: pointer;
  width: 16px;
  height: 16px;
}
/* ── Diff mode indicators ── */
.diff-indicator {
  display: inline-block;
  width: 3px;
  height: 1em;
  margin-right: 2px;
  vertical-align: text-bottom;
}
.diff-ind-added { background: #4caf50; }
.diff-ind-modified { background: #ff9800; }
.diff-summary {
  display: inline-flex;
  gap: 6px;
  font-size: 0.7rem;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  margin-left: 4px;
}
.diff-stat { padding: 1px 4px; border-radius: 3px; }
.diff-stat-added { color: #4caf50; background: rgba(76,175,80,.12); }
.diff-stat-removed { color: #f44336; background: rgba(244,67,54,.12); }
.diff-stat-modified { color: #ff9800; background: rgba(255,152,0,.12); }
.tab-unsaved {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  margin-left: 4px;
  flex-shrink: 0;
}
.btn-active {
  background: rgba(74,158,255,.2) !important;
  border-color: var(--accent) !important;
  color: var(--accent) !important;
}

/* ── Lighting preset selector ── */
.lighting-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 4px;
}
.lighting-select {
  background: rgba(30,30,34,.7);
  color: rgba(255,255,255,.8);
  border: 1px solid rgba(255,255,255,.15);
  border-radius: 6px;
  font-size: 0.65rem;
  padding: 2px 4px;
  cursor: pointer;
  outline: none;
  backdrop-filter: blur(6px);
}
.lighting-select:focus {
  border-color: rgba(74,158,255,.5);
}
[data-theme="light"] .lighting-select {
  background: rgba(255,255,255,.75);
  color: rgba(0,0,0,.7);
  border-color: rgba(0,0,0,.12);
}

/* ── Clipping plane slider ── */
.clip-slider-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 4px;
}
.clip-slider {
  flex: 1;
  accent-color: var(--accent);
  cursor: pointer;
  height: 4px;
}
.clip-value {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.62rem;
  color: rgba(255,255,255,.6);
  min-width: 30px;
  text-align: right;
}
[data-theme="light"] .clip-value {
  color: rgba(0,0,0,.5);
}
.clip-axis-select {
  background: rgba(255,255,255,.08);
  color: inherit;
  border: 1px solid rgba(255,255,255,.15);
  border-radius: 3px;
  font-size: 0.65rem;
  padding: 1px 2px;
  cursor: pointer;
  outline: none;
}
[data-theme="light"] .clip-axis-select {
  background: rgba(0,0,0,.05);
  border-color: rgba(0,0,0,.15);
}

/* ── Welcome / Onboarding modal ── */
.welcome-modal { max-width: 460px; }
.welcome-modal .modal-title { display: flex; align-items: center; gap: 8px; }
.welcome-logo { color: var(--accent); flex-shrink: 0; }
.welcome-tagline {
  font-size: 0.86rem;
  color: var(--text-dim);
  line-height: 1.45;
  margin-bottom: 14px;
}
.welcome-list {
  list-style: none;
  display: flex; flex-direction: column; gap: 10px;
  margin-bottom: 18px;
}
.welcome-item {
  display: flex; align-items: center; gap: 10px;
  font-size: 0.86rem; color: var(--text);
}
.welcome-check { color: var(--accent); flex-shrink: 0; }
.welcome-start-btn {
  width: 100%;
  padding: 9px 0;
  font-size: 0.9rem;
}

/* ── Empty state overlay ── */
.empty-state {
  position: absolute; inset: 0;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 12px;
  pointer-events: none;
  z-index: 5;
  text-align: center;
  padding: 0 24px;
}
.empty-state-icon { color: rgba(255,255,255,.22); }
.empty-state-text {
  font-size: 0.82rem;
  color: rgba(255,255,255,.4);
  max-width: 280px;
  line-height: 1.4;
}
[data-theme="light"] .empty-state-icon { color: rgba(0,0,0,.18); }
[data-theme="light"] .empty-state-text { color: rgba(0,0,0,.4); }

/* ── PNG export scale selector ── */
.screenshot-row { display: flex; align-items: center; gap: 4px; }
.png-scale-select {
  background: rgba(30,30,34,.7);
  color: rgba(255,255,255,.8);
  border: 1px solid rgba(255,255,255,.15);
  border-radius: 8px;
  font-size: 0.62rem;
  padding: 3px 4px;
  cursor: pointer;
  outline: none;
  backdrop-filter: blur(6px);
}
.png-scale-select:focus { border-color: rgba(74,158,255,.5); }
[data-theme="light"] .png-scale-select {
  background: rgba(255,255,255,.75);
  color: rgba(0,0,0,.7);
  border-color: rgba(0,0,0,.12);
}

/* ── Build plate dimension inputs ── */
.bed-dims-row {
  display: flex; align-items: center; gap: 6px;
  padding: 2px 4px;
}
.bed-dim { display: flex; align-items: center; gap: 3px; }
.bed-dim-label {
  font-size: 0.6rem;
  color: rgba(255,255,255,.5);
}
[data-theme="light"] .bed-dim-label { color: rgba(0,0,0,.45); }
.bed-dim-input {
  width: 42px;
  background: rgba(30,30,34,.7);
  color: rgba(255,255,255,.85);
  border: 1px solid rgba(255,255,255,.15);
  border-radius: 6px;
  font-size: 0.62rem;
  padding: 2px 4px;
  outline: none;
  backdrop-filter: blur(6px);
}
.bed-dim-input:focus { border-color: rgba(74,158,255,.5); }
[data-theme="light"] .bed-dim-input {
  background: rgba(255,255,255,.75);
  color: rgba(0,0,0,.75);
  border-color: rgba(0,0,0,.12);
}

/* ── Statistics panel (overlay) ── */
.stats-panel {
  position: absolute; top: 10px; left: 10px;
  width: 230px;
  background: rgba(30,30,34,.85);
  border: 1px solid rgba(255,255,255,.12);
  border-radius: var(--r-md);
  box-shadow: 0 8px 28px rgba(0,0,0,.4);
  backdrop-filter: blur(10px);
  z-index: 12;
  overflow: hidden;
}
[data-theme="light"] .stats-panel {
  background: rgba(255,255,255,.9);
  border-color: rgba(0,0,0,.1);
}
.stats-panel-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(128,128,128,.18);
}
.stats-panel-title {
  font-size: 0.78rem; font-weight: 700; color: var(--text);
}
.stats-panel-close {
  background: none; border: none; color: var(--text-dim);
  font-size: 1.2rem; line-height: 1; cursor: pointer; padding: 0 2px;
}
.stats-panel-close:hover { color: var(--text); }
.stats-panel-body { padding: 6px 12px 10px; }
.stats-panel-row {
  display: flex; align-items: center; justify-content: space-between;
  gap: 10px;
  padding: 5px 0;
  font-size: 0.74rem;
  border-bottom: 1px solid rgba(128,128,128,.1);
}
.stats-panel-row:last-child { border-bottom: none; }
.stats-panel-key { color: var(--text-dim); }
.stats-panel-val {
  color: var(--text);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.72rem;
  text-align: right;
}

/* ── Animation Timeline ── */
.anim-timeline {
  position: absolute;
  bottom: 28px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 12px;
  background: rgba(30,30,34,.78);
  border: 1px solid rgba(255,255,255,.12);
  border-radius: 20px;
  backdrop-filter: blur(8px);
  z-index: 10;
  white-space: nowrap;
}
[data-theme="light"] .anim-timeline {
  background: rgba(255,255,255,.78);
  border-color: rgba(0,0,0,.12);
}
.anim-play-btn {
  background: none; border: none; cursor: pointer;
  color: var(--accent);
  display: flex; align-items: center; justify-content: center;
  width: 24px; height: 24px;
  border-radius: 50%;
  transition: background 0.12s;
  padding: 0;
}
.anim-play-btn:hover {
  background: rgba(74,158,255,.2);
}
.anim-label {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.72rem;
  color: var(--hl-special);
  font-weight: 600;
}
.anim-slider {
  width: 140px;
  accent-color: var(--accent);
  cursor: pointer;
  height: 4px;
}
.anim-value {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  color: var(--text);
  min-width: 40px;
  text-align: right;
}
.anim-dur-label {
  display: flex; align-items: center; gap: 3px;
  font-size: 0.66rem;
  color: var(--text-dim);
  margin-left: 4px;
}
.anim-dur-text {
  white-space: nowrap;
}
.anim-dur-input {
  width: 36px;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 4px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.66rem;
  text-align: center;
  -moz-appearance: textfield;
}
.anim-dur-input::-webkit-inner-spin-button,
.anim-dur-input::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.anim-dur-unit {
  font-size: 0.66rem;
  color: var(--text-dim);
}

/* ── Toast Notifications ── */
.toast-container {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 20000;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  pointer-events: none;
}
.toast-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 0.78rem;
  font-weight: 500;
  pointer-events: auto;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 16px rgba(0,0,0,.35);
  min-width: 180px;
  max-width: 320px;
}
.toast-icon {
  font-size: 0.85rem;
  flex-shrink: 0;
}
.toast-msg {
  flex: 1;
  line-height: 1.3;
}
.toast-action {
  flex-shrink: 0;
  margin-left: var(--sp-3);
  padding: var(--sp-1) var(--sp-3);
  border-radius: var(--r-sm);
  border: 1px solid rgba(255, 255, 255, 0.5);
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
  font-size: var(--fz-xs);
  font-weight: 700;
  cursor: pointer;
  transition: background 0.12s;
}
.toast-action:hover {
  background: rgba(255, 255, 255, 0.3);
}
.toast-success {
  background: rgba(46, 160, 67, 0.9);
  color: #fff;
  border: 1px solid rgba(46, 160, 67, 0.6);
}
.toast-info {
  background: rgba(74, 158, 255, 0.9);
  color: #fff;
  border: 1px solid rgba(74, 158, 255, 0.6);
}
.toast-error {
  background: rgba(231, 76, 60, 0.9);
  color: #fff;
  border: 1px solid rgba(231, 76, 60, 0.6);
}
/* Toast enter/leave animations */
.toast-enter-active {
  animation: toast-slide-in 0.3s ease-out;
}
.toast-leave-active {
  animation: toast-fade-out 0.3s ease-in forwards;
}
@keyframes toast-slide-in {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
@keyframes toast-fade-out {
  from { transform: translateX(0); opacity: 1; }
  to { transform: translateX(40px); opacity: 0; }
}

/* ── Example Button Tooltips ── */
.example-btn {
  position: relative;
}
.example-btn::after {
  content: attr(data-tooltip);
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 6px;
  padding: 4px 10px;
  background: var(--surface);
  color: var(--text-dim);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 0.66rem;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.15s;
  z-index: 300;
  box-shadow: 0 4px 12px rgba(0,0,0,.25);
}
.example-btn:hover::after {
  opacity: 1;
}

/* ── Canvas Gradient Background ── */
.canvas-gradient-bg {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
}
.canvas-transparent {
  background: transparent !important;
}
.gpu-canvas:focus {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

/* ════════════════════════════════════════════════════
   POLISH BATCH: gizmo, loading, context menu, status bar,
   color swatches, micro-interactions, scrollbars
   ════════════════════════════════════════════════════ */

/* ── App first-load fade-in ── */
.app {
  opacity: 0;
  transition: opacity 0.4s ease;
}
.app.app-loaded {
  opacity: 1;
}

/* ── Feature 1: Orientation cube / navigation gizmo ── */
.nav-gizmo {
  position: absolute;
  top: 10px;
  right: 130px; /* offset left of the viewport toolbar menus */
  z-index: 11;
  border-radius: 50%;
  background: rgba(30, 30, 34, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(6px);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
  transition: background 0.15s, border-color 0.15s;
  user-select: none;
}
.nav-gizmo:hover {
  background: rgba(30, 30, 34, 0.72);
  border-color: rgba(255, 255, 255, 0.22);
}
[data-theme="light"] .nav-gizmo {
  background: rgba(255, 255, 255, 0.6);
  border-color: rgba(0, 0, 0, 0.1);
}
[data-theme="light"] .nav-gizmo:hover {
  background: rgba(255, 255, 255, 0.8);
  border-color: rgba(0, 0, 0, 0.18);
}
.nav-gizmo-svg { display: block; overflow: visible; }
.gizmo-axis { cursor: pointer; }
.gizmo-dot { transition: r 0.1s ease, filter 0.1s ease; }
.gizmo-axis:hover .gizmo-dot {
  filter: brightness(1.35) drop-shadow(0 0 3px currentColor);
  r: 10;
}
.gizmo-text {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 9px;
  font-weight: 700;
  fill: #fff;
  pointer-events: none;
}

/* Viewport Compass */
.viewport-compass {
  position: absolute;
  bottom: 12px;
  left: 12px;
  width: 60px;
  height: 60px;
  pointer-events: none;
  opacity: 0.7;
  z-index: 5;
}
.viewport-compass:hover { opacity: 1; }
.viewport-compass text { user-select: none; pointer-events: none; }

/* Reference search */
.ref-search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
}
.ref-search-icon { color: var(--text-dim); flex-shrink: 0; }
.ref-search-input {
  flex: 1;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: var(--text);
  font-size: 13px;
  outline: none;
}
.ref-search-input:focus { border-color: var(--accent); }
.ref-search-input::placeholder { color: var(--text-dim); }
.ref-highlight { background: rgba(255, 200, 0, 0.3); color: inherit; border-radius: 2px; }
.ref-no-results { padding: 24px; text-align: center; color: var(--text-dim); font-size: 13px; }

/* Code Complexity indicator */
.stat-complexity { cursor: default; }

/* ── Feature 2: WebGPU loading overlay ── */
.webgpu-loading {
  position: absolute;
  inset: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  background: var(--canvas-bg);
  animation: fade-in 0.2s ease;
}
.webgpu-spinner {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 3px solid rgba(128, 128, 128, 0.25);
  border-top-color: var(--accent);
  animation: gizmo-spin 0.8s linear infinite;
}
.webgpu-loading-text {
  font-size: 0.82rem;
  color: var(--text-dim);
  font-weight: 500;
  letter-spacing: 0.2px;
}
@keyframes gizmo-spin {
  to { transform: rotate(360deg); }
}
@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* ── Feature 3: Right-click context menu ── */
.ctx-menu {
  position: absolute;
  z-index: 1000;
  min-width: 190px;
  padding: 5px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(10px);
  animation: ctx-menu-in 0.12s ease;
}
@keyframes ctx-menu-in {
  from { opacity: 0; transform: scale(0.96) translateY(-3px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
.ctx-menu-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  font-size: 0.8rem;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
}
.ctx-menu-item:hover { background: var(--hover); }
.ctx-menu-item:active { transform: scale(0.985); }
.ctx-menu-ico { flex-shrink: 0; color: var(--text-dim); }
.ctx-menu-item:hover .ctx-menu-ico { color: var(--accent); }

/* ── Feature 4: Status bar redesign (segmented) ── */
.stats {
  padding: 5px 12px;
}
.stat-seg {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}
.stat-ico {
  color: var(--text-dim);
  flex-shrink: 0;
  opacity: 0.85;
}
.stat-val {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.7rem;
  color: var(--text);
}
.stat-div {
  width: 1px;
  height: 12px;
  background: var(--border);
  flex-shrink: 0;
  opacity: 0.8;
}
.stat-render .stat-ico { color: var(--accent); }
.stat-render .stat-val { color: var(--accent); }
.stat-fps .stat-ico { color: var(--hl-special); }
.stat-fps .stat-val { color: var(--hl-special); }
.stat-selection .stat-ico { color: var(--accent); }
.stat-selection .stat-val { color: var(--accent); }
.stat-wordwrap .stat-ico { color: var(--hl-special); }

/* ── Feature 5: Inline color swatches in editor ── */
/* The swatch is rendered with zero layout width so the highlight layer stays
   character-aligned with the textarea underneath. The visible box is drawn via
   an absolutely-positioned pseudo-element that does not consume horizontal space. */
:deep(.color-swatch) {
  display: inline-block;
  position: relative;
  width: 0;
  height: 0;
  overflow: visible;
  z-index: 10;
  pointer-events: auto;
}
:deep(.color-swatch)::before {
  content: '';
  position: absolute;
  /* Sit over the trailing edge of the "(" so the color argument stays readable. */
  left: -0.55em;
  top: 0.18em;
  width: 0.72em;
  height: 0.72em;
  background: inherit;
  border-radius: 2px;
  border: 1px solid rgba(128, 128, 128, 0.55);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.15) inset;
  cursor: pointer;
  pointer-events: auto;
}

/* ── Code Folding ── */
:deep(.fold-marker) {
  cursor: pointer;
  display: inline-block;
  width: 1.2em;
  text-align: center;
  color: var(--text-dim);
  font-size: 0.7em;
  vertical-align: middle;
  user-select: none;
  opacity: 0.6;
  transition: opacity 0.15s;
}
:deep(.fold-marker):hover {
  opacity: 1;
  color: var(--accent);
}
:deep(.fold-placeholder) {
  background: var(--hover);
  color: var(--text-dim);
  border-radius: 3px;
  padding: 0 4px;
  margin-left: 4px;
  font-size: 0.85em;
  cursor: pointer;
  border: 1px solid var(--border);
}

/* ── Color Picker ── */
.color-picker-popup {
  position: absolute;
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}
.color-picker-input {
  width: 40px;
  height: 28px;
  border: none;
  padding: 0;
  cursor: pointer;
  background: transparent;
}
.color-picker-close {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  font-size: 16px;
  padding: 0 2px;
  line-height: 1;
}
.color-picker-close:hover {
  color: var(--text);
}
.color-palette-grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 3px;
  padding: 2px 0 4px;
}
.color-palette-swatch {
  width: 18px;
  height: 18px;
  border: 1px solid rgba(255,255,255,.2);
  border-radius: 3px;
  cursor: pointer;
  padding: 0;
  transition: transform 0.1s;
}
.color-palette-swatch:hover {
  transform: scale(1.25);
  z-index: 1;
  border-color: var(--accent);
}
[data-theme="light"] .color-palette-swatch {
  border-color: rgba(0,0,0,.15);
}
.color-picker-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

/* ── Feature 6: Micro-interactions ── */
/* Slide/fade transitions for collapsible panels */
.panel-slide-enter-active,
.panel-slide-leave-active {
  transition: max-height 0.22s ease, opacity 0.18s ease, transform 0.22s ease;
  overflow: hidden;
}
.panel-slide-enter-from,
.panel-slide-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-4px);
}
.panel-slide-enter-to,
.panel-slide-leave-from {
  max-height: 420px;
  opacity: 1;
}

/* Overlay panel fade/scale (object tree, console height stays manual) */
.overlay-fade-enter-active,
.overlay-fade-leave-active {
  transition: opacity 0.16s ease, transform 0.16s ease;
}
.overlay-fade-enter-from,
.overlay-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}

/* Console slide (height is set inline, so only fade + slide-up) */
.console-slide-enter-active,
.console-slide-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.console-slide-enter-from,
.console-slide-leave-to {
  opacity: 0;
  transform: translateY(12px);
}

/* Subtle active press states for buttons */
.btn:active,
.tb-btn:active,
.view-btn:active,
.find-btn:active,
.console-toggle-btn:active,
.console-clear-btn:active,
.tab-add:active,
.anim-play-btn:active {
  transform: scale(0.95);
}
.btn, .tb-btn, .view-btn, .find-btn {
  transition: background 0.12s, border-color 0.12s, transform 0.08s ease;
}

/* ── Themed custom scrollbars ── */
:deep(*)::-webkit-scrollbar {
  width: 9px;
  height: 9px;
}
:deep(*)::-webkit-scrollbar-track {
  background: transparent;
}
:deep(*)::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 6px;
  border: 2px solid transparent;
  background-clip: padding-box;
}
:deep(*)::-webkit-scrollbar-thumb:hover {
  background: var(--text-dim);
  background-clip: padding-box;
}
:deep(*)::-webkit-scrollbar-corner { background: transparent; }

/* ── Viewport toolbar (3 dropdown menus) ── */
.vp-toolbar {
  position: absolute; top: 10px; right: 10px;
  display: flex; gap: 4px;
  z-index: 15;
}
.vp-menu-wrapper {
  position: relative;
}
.vp-menu-btn {
  width: 34px; height: 34px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 10px;
  border: 1px solid rgba(255,255,255,.15);
  background: rgba(30,30,34,.7);
  color: rgba(255,255,255,.8);
  cursor: pointer;
  backdrop-filter: blur(6px);
  transition: background 0.12s, border-color 0.12s;
  padding: 0;
}
.vp-menu-btn:hover {
  background: rgba(74,158,255,.3);
  border-color: rgba(74,158,255,.5);
  color: #fff;
}
[data-theme="light"] .vp-menu-btn {
  background: rgba(255,255,255,.75);
  border-color: rgba(0,0,0,.12);
  color: rgba(0,0,0,.7);
}
[data-theme="light"] .vp-menu-btn:hover {
  background: rgba(43,125,233,.2);
  border-color: rgba(43,125,233,.4);
  color: var(--accent);
}

.vp-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  min-width: 190px;
  max-height: 70vh;
  overflow-y: auto;
  padding: 5px;
  background: rgba(30,30,34,.92);
  border: 1px solid rgba(255,255,255,.14);
  border-radius: 10px;
  box-shadow: 0 10px 30px rgba(0,0,0,.4);
  backdrop-filter: blur(12px);
  z-index: 100;
}
[data-theme="light"] .vp-dropdown {
  background: rgba(255,255,255,.94);
  border-color: rgba(0,0,0,.1);
  box-shadow: 0 10px 30px rgba(0,0,0,.15);
}
.vp-dropdown-enter-active {
  animation: vp-dd-in 0.14s ease;
}
.vp-dropdown-leave-active {
  animation: vp-dd-out 0.1s ease forwards;
}
@keyframes vp-dd-in {
  from { opacity: 0; transform: translateY(-4px) scale(0.97); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
@keyframes vp-dd-out {
  from { opacity: 1; transform: translateY(0) scale(1); }
  to { opacity: 0; transform: translateY(-4px) scale(0.97); }
}

.vp-dd-item {
  display: flex; align-items: center; gap: 8px;
  width: 100%;
  padding: 6px 10px;
  border: none; border-radius: 6px;
  background: transparent;
  color: rgba(255,255,255,.85);
  font-size: 0.76rem;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
  white-space: nowrap;
}
.vp-dd-item:hover { background: rgba(74,158,255,.2); }
[data-theme="light"] .vp-dd-item { color: var(--text); }
[data-theme="light"] .vp-dd-item:hover { background: rgba(43,125,233,.12); }

.vp-dd-check {
  color: var(--accent);
  font-size: 0.72rem;
  font-weight: 700;
  min-width: 14px;
  text-align: center;
}

.vp-dd-sep {
  height: 1px;
  background: rgba(255,255,255,.1);
  margin: 3px 6px;
}
[data-theme="light"] .vp-dd-sep {
  background: rgba(0,0,0,.08);
}

.vp-dd-label {
  font-size: 0.65rem;
  color: rgba(255,255,255,.4);
  padding: 4px 10px 2px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}
[data-theme="light"] .vp-dd-label {
  color: rgba(0,0,0,.4);
}

.vp-dd-swatches {
  display: flex; gap: 4px; padding: 4px 10px 2px;
  flex-wrap: wrap;
}

.vp-dd-select {
  margin: 4px 10px;
  width: calc(100% - 20px);
}

.vp-dd-slider-row {
  display: flex; align-items: center; gap: 6px;
  padding: 2px 10px;
}

.vp-dd-dims {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 10px;
}

.vp-dd-empty {
  font-size: 0.68rem;
  color: rgba(255,255,255,.35);
  padding: 4px 10px;
  font-style: italic;
}
[data-theme="light"] .vp-dd-empty { color: rgba(0,0,0,.35); }

.vp-dd-bookmark {
  display: flex; align-items: center;
}
.vp-dd-bk-name {
  flex: 1;
}
.vp-dd-bk-del {
  background: none; border: none; cursor: pointer;
  color: rgba(255,255,255,.35);
  font-size: 0.9rem; line-height: 1; padding: 2px 6px;
  border-radius: 4px;
  transition: color 0.1s, background 0.1s;
}
.vp-dd-bk-del:hover {
  color: var(--danger);
  background: rgba(231,76,60,.15);
}
[data-theme="light"] .vp-dd-bk-del { color: rgba(0,0,0,.3); }

/* ── Zoom controls (standalone overlay) ── */
.vp-zoom-controls {
  position: absolute;
  bottom: 60px;
  right: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  z-index: 10;
}
.vp-bookmark-btn {
  display: flex; align-items: center; justify-content: center;
  padding: 5px 8px;
}
.vp-bookmark-btn:hover {
  color: #ffd700;
  border-color: rgba(255,215,0,.5);
}

/* ── Canvas drop overlay for STL import ── */
/* ── Vignette overlay ── */
.vignette-overlay {
  position: absolute; inset: 0; z-index: 50;
  pointer-events: none;
  box-shadow: inset 0 0 120px 40px rgba(0, 0, 0, 0.55);
  border-radius: 0;
}
.canvas-drop-overlay {
  position: absolute; inset: 0; z-index: 200;
  background: rgba(74, 158, 255, 0.1);
  border: 2.5px dashed var(--accent);
  border-radius: 0;
  display: flex; align-items: center; justify-content: center;
  pointer-events: none;
}
.canvas-drop-content {
  display: flex; flex-direction: column; align-items: center; gap: 8px;
  color: var(--accent);
  font-size: 0.82rem; font-weight: 600;
}
.canvas-drop-content svg { opacity: 0.7; }

/* ── Pin Tab ── */
.tab-pin-icon {
  color: var(--accent);
  flex-shrink: 0;
  margin-right: 2px;
  opacity: 0.75;
}
.tab-item.pinned { border-left: 2px solid var(--accent); }

/* ── Tab context menu ── */
.tab-ctx-menu {
  position: fixed;
  z-index: 10001;
  min-width: 150px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,.35);
  animation: ctx-menu-in 0.12s ease;
}
.tab-ctx-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text);
  font-size: 0.76rem;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
}
.tab-ctx-item:hover { background: var(--hover); }

/* ── Hover documentation tooltip ── */
.hover-doc-tooltip {
  position: absolute;
  z-index: 200;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 6px 10px;
  box-shadow: 0 4px 16px rgba(0,0,0,.3);
  pointer-events: none;
  max-width: 320px;
  white-space: nowrap;
  animation: fade-in 0.12s ease;
}
.hover-doc-sig {
  font-family: var(--editor-font-family, 'JetBrains Mono', monospace);
  font-size: 0.76rem;
  color: var(--hl-keyword);
  font-weight: 600;
  margin-bottom: 2px;
}
.hover-doc-desc {
  font-size: 0.72rem;
  color: var(--text-dim);
}

/* ── Error squiggly underline ── */
:deep(.error-squiggly) {
  text-decoration: wavy underline var(--danger);
  text-decoration-skip-ink: none;
  text-underline-offset: 3px;
}

/* ── Smooth scrolling for editor ── */
.code {
  scroll-behavior: smooth;
}

/* ── Preferences: font select ── */
.pref-select {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 3px 8px;
  font-size: 0.78rem;
  cursor: pointer;
  outline: none;
}
.pref-select:focus { border-color: var(--accent); }

/* ── Hamburger menu (hidden on large screens) ── */
.hamburger-wrapper { display: none; position: relative; }
.hamburger-btn { padding: 4px 8px; }
.hamburger-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  min-width: 160px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,.35);
  z-index: 200;
  animation: ctx-menu-in 0.12s ease;
}
.hamburger-item {
  display: block;
  width: 100%;
  padding: 8px 12px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text);
  font-size: 0.8rem;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
}
.hamburger-item:hover { background: var(--hover); }

@media (max-width: 600px) {
  .topbar-brand-text { display: none; }
  .topbar-collapsible { display: none !important; }
  .hamburger-wrapper { display: block; }
}

@media (max-width: 800px) {
  .main { flex-direction: column; }
  .editor-panel { width: 100% !important; max-width: 100% !important; height: 40vh; border-right: none; border-bottom: 1px solid var(--border); }
  .divider { display: none; }
  .canvas-panel { height: 60vh; }
}

/* ── Performance Panel ── */
.perf-panel {
  position: absolute;
  top: 44px;
  right: 10px;
  width: 260px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: 0 8px 24px rgba(0,0,0,.35);
  z-index: 50;
  font-size: var(--fz-xs);
  animation: ctx-menu-in 0.12s ease;
}
.perf-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
}
.perf-panel-title {
  font-weight: 600;
  font-size: var(--fz-sm);
}
.perf-panel-close {
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 1.1rem;
  cursor: pointer;
  line-height: 1;
  padding: 0 4px;
}
.perf-panel-close:hover { color: var(--text); }
.perf-panel-body {
  padding: 6px 10px 10px;
  max-height: 400px;
  overflow-y: auto;
}
.perf-section-title {
  font-weight: 700;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--accent);
  margin-top: 8px;
  margin-bottom: 4px;
}
.perf-section-title:first-child { margin-top: 2px; }
.perf-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 2px 0;
}
.perf-key {
  color: var(--text-dim);
}
.perf-val {
  font-weight: 600;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}
.perf-val-sm {
  font-size: 0.68rem;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── Device Lost Overlay ── */
.device-lost-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,.65);
  z-index: 100;
  animation: fade-in 0.2s ease;
}
.device-lost-content {
  text-align: center;
  padding: 32px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: 0 8px 32px rgba(0,0,0,.5);
  color: var(--text);
}
.device-lost-content svg {
  color: var(--danger);
  margin-bottom: 12px;
}
.device-lost-msg {
  margin-bottom: 16px;
  font-size: var(--fz-sm);
  color: var(--text-dim);
}

/* ── Shortcuts modal improvements ── */
.shortcuts-modal {
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}
.shortcuts-modal .modal-header {
  flex-shrink: 0;
}
.modal-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.shortcuts-print-btn {
  font-size: 0.72rem;
  padding: 2px 10px;
}
.shortcuts-scroll {
  overflow-y: auto;
  max-height: 60vh;
  flex: 1;
}
.shortcut-category {
  font-weight: 700;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--accent);
  margin-top: 14px;
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}
.shortcut-category:first-child {
  margin-top: 4px;
}

/* ── OpenSCAD Reference Panel ── */
.scad-ref-modal {
  max-height: 85vh;
  max-width: 620px;
  width: 90vw;
  display: flex;
  flex-direction: column;
}
.scad-ref-scroll {
  overflow-y: auto;
  max-height: 72vh;
  flex: 1;
  padding: 6px 0;
}
.scad-ref-section {
  margin-bottom: 12px;
}
.scad-ref-section-title {
  font-weight: 700;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--accent);
  margin-top: 10px;
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}
.scad-ref-entry {
  display: grid;
  grid-template-columns: 120px 1fr;
  grid-template-rows: auto auto;
  gap: 0 10px;
  padding: 4px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
}
.scad-ref-name {
  font-weight: 600;
  font-size: 0.82rem;
  color: var(--hl-keyword, var(--accent));
  grid-row: 1 / 3;
  align-self: center;
}
.scad-ref-sig {
  font-size: 0.78rem;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: var(--hl-string, #6ec87a);
  background: color-mix(in srgb, var(--surface) 80%, transparent);
  padding: 1px 6px;
  border-radius: 3px;
  white-space: nowrap;
  overflow-x: auto;
}
.scad-ref-desc {
  font-size: 0.76rem;
  color: var(--text-dim);
  margin-top: 1px;
}

/* ── Split Editor Container ── */
.editor-split-container {
  display: flex;
  flex-direction: column;
  flex: 1 1 0;
  min-height: 0;
  overflow: hidden;
}
.editor-split-container.split-active {
  flex-direction: row;
}
.editor-split-container.split-active > .code-editor {
  flex: 1 1 50%;
  min-width: 0;
}

/* ── Split Editor ── */
.split-editor-pane {
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border);
  flex: 1 1 0;
  min-width: 0;
  overflow: hidden;
}
.split-pane-header {
  display: flex;
  align-items: center;
  padding: 2px 6px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.split-pane-select {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 2px 6px;
  font-size: 11px;
  width: 100%;
}
.split-code-editor {
  flex: 1 1 0;
  min-height: 0;
}
.split-code-editor .code-area {
  height: 100%;
}
.split-code-editor .code {
  height: 100%;
}

/* ── History Panel ── */
.history-panel {
  background: var(--surface);
  border-top: 1px solid var(--border);
  max-height: 200px;
  overflow-y: auto;
  flex-shrink: 0;
}
.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  background: var(--surface);
  z-index: 1;
}
.history-title {
  font-weight: 600;
  font-size: 12px;
  color: var(--text);
}
.history-close {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  font-size: 16px;
  padding: 0 4px;
}
.history-close:hover { color: var(--text); }
.history-body {
  padding: 4px 8px;
}
.history-empty {
  color: var(--text-dim);
  font-size: 12px;
  padding: 8px 0;
  text-align: center;
}
.history-entry {
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.history-entry:last-child { border-bottom: none; }
.history-entry-info {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-dim);
  margin-bottom: 2px;
}
.history-entry-time { font-weight: 600; }
.history-entry-size { opacity: 0.7; }
.history-entry-preview {
  font-size: 11px;
  color: var(--text);
  font-family: var(--editor-font-family, monospace);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
  opacity: 0.7;
}
.history-restore-btn {
  font-size: 10px;
  padding: 1px 8px;
}
.history-timeline {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}
.timeline-bar {
  position: relative;
  height: 20px;
  background: var(--border);
  border-radius: 10px;
  margin: 8px 0;
}
.timeline-dot {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 12px;
  height: 12px;
  background: var(--text-dim);
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.15s;
  z-index: 1;
}
.timeline-dot:hover {
  width: 16px;
  height: 16px;
  background: var(--accent);
}
.timeline-dot-current {
  width: 16px;
  height: 16px;
  background: var(--accent);
  box-shadow: 0 0 6px var(--accent);
}
.timeline-tooltip {
  display: none;
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  white-space: nowrap;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  margin-bottom: 4px;
  pointer-events: none;
}
.timeline-dot:hover .timeline-tooltip {
  display: block;
}

/* ── Example Gallery Modal ── */
.example-gallery-modal {
  max-width: 700px;
  width: 90vw;
}
.example-gallery-search {
  width: 100%;
  padding: 8px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  color: var(--text);
  font-size: var(--fz-sm);
  margin-bottom: 12px;
  outline: none;
}
.example-gallery-search:focus {
  border-color: var(--accent);
}
.example-gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  max-height: 55vh;
  overflow-y: auto;
  padding-right: 4px;
}
.example-card {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.example-card:hover {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}
.example-card-name {
  font-weight: 700;
  font-size: var(--fz-sm);
  color: var(--text);
}
.example-card-desc {
  font-size: 0.72rem;
  color: var(--text-dim);
}
.example-card-code {
  font-family: var(--editor-font-family, monospace);
  font-size: 0.68rem;
  color: var(--text-dim);
  background: rgba(0,0,0,.15);
  padding: 4px 6px;
  border-radius: 3px;
  margin: 4px 0;
  overflow: hidden;
  white-space: pre;
  max-height: 3.2em;
  line-height: 1.4;
}
.example-card-load {
  align-self: flex-end;
  font-size: 0.68rem;
  padding: 2px 10px;
  opacity: 0;
  transition: opacity 0.15s;
}
.example-card:hover .example-card-load {
  opacity: 1;
}

/* ── Snippet Dropdown ── */
.snippet-panel-wrapper {
  position: relative;
  display: inline-flex;
}
.snippet-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: 0 6px 20px rgba(0,0,0,.35);
  z-index: 100;
  min-width: 260px;
  max-height: 320px;
  overflow-y: auto;
  animation: ctx-menu-in 0.1s ease;
}
.snippet-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  background: none;
  color: var(--text);
  font-size: var(--fz-sm);
  cursor: pointer;
  text-align: left;
  border-bottom: 1px solid var(--border);
}
.snippet-item:last-child { border-bottom: none; }
.snippet-item:hover {
  background: var(--hover);
}
.snippet-name {
  font-weight: 600;
  font-size: 0.78rem;
}
.snippet-preview {
  font-family: var(--editor-font-family, monospace);
  font-size: 0.68rem;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── Export Dropdown (toolbar) ── */
.export-dropdown-wrapper {
  position: relative;
  display: inline-flex;
}
.export-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: 0 6px 20px rgba(0,0,0,.35);
  z-index: 100;
  min-width: 180px;
  animation: ctx-menu-in 0.1s ease;
}
.export-dd-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  border: none;
  background: none;
  color: var(--text);
  font-size: var(--fz-sm);
  cursor: pointer;
  text-align: left;
  white-space: nowrap;
}
.export-dd-item:hover {
  background: var(--hover);
}
.export-dd-sep {
  height: 1px;
  background: var(--border);
  margin: 2px 0;
}

/* ── Camera Info Overlay ── */
.camera-info-overlay {
  position: absolute;
  bottom: 40px;
  left: 10px;
  background: rgba(0,0,0,.65);
  backdrop-filter: blur(6px);
  border: 1px solid rgba(255,255,255,.1);
  border-radius: var(--r-md);
  padding: 8px 12px;
  font-size: 0.72rem;
  font-family: var(--editor-font-family, monospace);
  color: rgba(255,255,255,.85);
  z-index: 20;
  min-width: 160px;
  pointer-events: none;
  animation: ctx-menu-in 0.12s ease;
}
.camera-info-title {
  font-weight: 700;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--accent);
  margin-bottom: 4px;
}
.camera-info-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 12px;
  padding: 1px 0;
}
.camera-info-key {
  color: rgba(255,255,255,.5);
}
.camera-info-val {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

/* ── Mobile Reset View FAB ── */
.mobile-reset-fab {
  display: none;
  position: absolute;
  bottom: 50px;
  right: 14px;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  border: none;
  box-shadow: 0 4px 14px rgba(0,0,0,.4);
  cursor: pointer;
  z-index: 20;
  align-items: center;
  justify-content: center;
}

/* ── Responsive: touch / mobile enhancements ── */
@media (hover: none) and (pointer: coarse) {
  .mobile-reset-fab {
    display: flex;
  }
  .vp-zoom-controls .zoom-btn {
    width: 40px;
    height: 40px;
    font-size: 1.2rem;
  }
  .vp-toolbar .vp-menu-btn {
    width: 36px;
    height: 36px;
  }
  .vp-dropdown {
    font-size: 0.82rem;
  }
  .vp-dd-item {
    padding: 8px 14px;
  }
  /* Hide text on dropdown menus, keep icons */
  .canvas-hint {
    display: none;
  }
}

@media (max-width: 600px) {
  .mobile-reset-fab {
    display: flex;
  }
  .vp-zoom-controls .zoom-btn {
    width: 38px;
    height: 38px;
    font-size: 1.1rem;
  }
}

/* ── Measurement Tool ── */
.measure-overlay {
  pointer-events: none;
  position: absolute;
  inset: 0;
  z-index: 20;
}
.measure-hint {
  position: absolute;
  bottom: 60px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0,0,0,0.7);
  color: #fff;
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 12px;
  white-space: nowrap;
  pointer-events: none;
}
.measure-result {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: rgba(0,0,0,0.85);
  color: #4af;
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 10px;
  pointer-events: auto;
}
.measure-clear-btn {
  background: none;
  border: none;
  color: #aaa;
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}
.measure-clear-btn:hover {
  color: #fff;
}
.measure-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  background: var(--accent, #4a9eff);
  color: #fff;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 11px;
  display: flex;
  align-items: center;
  gap: 4px;
  pointer-events: none;
}

/* ── Ghost Controls ── */
.ghost-controls {
  position: absolute;
  bottom: 42px;
  right: 8px;
  background: var(--surface, #1e1e22);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 6px;
  padding: 4px 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  z-index: 15;
  font-size: 11px;
  color: var(--text-dim, #888);
}
.ghost-label {
  white-space: nowrap;
}
.ghost-select {
  background: var(--bg, #141416);
  color: var(--text, #e4e4e8);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 11px;
}

/* ── Notification Badge ── */
.notif-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #f44;
  pointer-events: none;
}
.tb-btn.has-badge {
  position: relative;
}

/* ── What's New ── */
.whats-new-section {
  margin-top: 16px;
  border-top: 1px solid var(--border, #2e2e34);
  padding-top: 12px;
}
.whats-new-title {
  font-size: 13px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: var(--text, #e4e4e8);
}
.whats-new-release {
  margin-bottom: 8px;
}
.whats-new-version {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent, #4a9eff);
  margin-bottom: 2px;
}
.whats-new-list {
  margin: 0;
  padding-left: 18px;
  font-size: 12px;
  color: var(--text-dim, #888);
  line-height: 1.5;
}
.whats-new-list li {
  margin: 1px 0;
}

/* ── 3D Annotations ── */
.annotation-label {
  position: absolute;
  pointer-events: none;
  z-index: 22;
  transform: translate(-50%, -100%);
  display: flex;
  flex-direction: column;
  align-items: center;
}
.annotation-pin {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent, #4a9eff);
  box-shadow: 0 0 6px rgba(74, 158, 255, 0.6);
  margin-top: 2px;
}
.annotation-callout {
  background: rgba(0, 0, 0, 0.82);
  color: #fff;
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 4px;
  white-space: nowrap;
  margin-bottom: 4px;
  border: 1px solid rgba(74, 158, 255, 0.4);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  position: relative;
}
.annotation-callout::after {
  content: '';
  position: absolute;
  bottom: -5px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 5px solid transparent;
  border-right: 5px solid transparent;
  border-top: 5px solid rgba(0, 0, 0, 0.82);
}

/* ── Bookmark Thumbnails ── */
.vp-dd-bookmark {
  position: relative;
}
.bk-thumb-popup {
  display: none;
  position: absolute;
  left: 100%;
  top: 50%;
  transform: translateY(-50%);
  margin-left: 4px;
  z-index: 100;
  border: 1px solid var(--border, #2e2e34);
  border-radius: 4px;
  overflow: hidden;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  background: var(--bg, #141416);
}
.vp-dd-bookmark:hover .bk-thumb-popup {
  display: block;
}
.bk-thumb-img {
  width: 80px;
  height: 60px;
  display: block;
}

/* ── Viewport Watermark ── */
.viewport-watermark {
  position: absolute;
  z-index: 5;
  font-size: 14px;
  font-weight: 600;
  color: var(--text, #e4e4e8);
  pointer-events: none;
  user-select: none;
  letter-spacing: 0.03em;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
}

/* ── Session Restore Dialog ── */
.session-restore-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}
.session-restore-dialog {
  background: var(--surface, #1e1e22);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 10px;
  padding: 24px 28px;
  text-align: center;
  max-width: 340px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}
.session-restore-icon {
  color: var(--accent, #4a9eff);
  margin-bottom: 12px;
}
.session-restore-msg {
  color: var(--text, #e4e4e8);
  font-size: 13px;
  margin: 0 0 16px 0;
  line-height: 1.5;
}
.session-restore-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}
.session-restore-yes {
  background: var(--accent, #4a9eff) !important;
  color: #fff !important;
  border: none !important;
  padding: 6px 16px !important;
  border-radius: 6px !important;
  font-weight: 600;
  cursor: pointer;
}
.session-restore-yes:hover {
  filter: brightness(1.1);
}
.session-restore-no {
  background: var(--hover, #28282e) !important;
  color: var(--text-dim, #888) !important;
  border: 1px solid var(--border, #2e2e34) !important;
  padding: 6px 16px !important;
  border-radius: 6px !important;
  cursor: pointer;
}
.session-restore-no:hover {
  background: var(--border, #2e2e34) !important;
}

/* ── Preferences text input ── */
.pref-text-input {
  background: var(--bg, #141416);
  color: var(--text, #e4e4e8);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 12px;
  width: 160px;
}
.pref-text-input:focus {
  outline: none;
  border-color: var(--accent, #4a9eff);
}

/* ── Ghost Comparison Statistics Panel ── */
.ghost-stats-panel {
  position: absolute; top: 10px; left: 250px;
  width: 230px;
  background: rgba(30,30,34,.85);
  border: 1px solid rgba(74, 158, 255, 0.25);
  border-radius: var(--r-md);
  box-shadow: 0 8px 28px rgba(0,0,0,.4);
  backdrop-filter: blur(10px);
  z-index: 12;
  overflow: hidden;
}
[data-theme="light"] .ghost-stats-panel {
  background: rgba(255,255,255,.9);
  border-color: rgba(74, 158, 255, 0.25);
}
.ghost-stat-pos { color: #4caf50 !important; }
.ghost-stat-neg { color: #f44336 !important; }

/* ── Preset Configurations ── */
.pref-section-divider {
  border-top: 1px solid var(--border, #2e2e34);
  margin: 12px 0 8px;
}
.pref-section-title {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--accent, #4a9eff);
  margin-bottom: 8px;
}
.preset-btns {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.preset-btn {
  font-size: 11px !important;
  padding: 3px 8px !important;
}
.preset-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.preset-item {
  display: flex;
  align-items: center;
  gap: 4px;
}
.preset-del-btn {
  color: #f44 !important;
  font-size: 14px !important;
  padding: 2px 6px !important;
  line-height: 1;
}
.preset-save-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

/* ── Welcome Tour ── */
.tour-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.55);
  pointer-events: auto;
}
.tour-spotlight {
  position: fixed;
  box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.55);
  border: 2px solid var(--accent, #4a9eff);
  border-radius: 8px;
  pointer-events: none;
  z-index: 10000;
  transition: all 0.3s ease;
}
.tour-tooltip {
  position: fixed;
  width: 300px;
  background: var(--surface, #1e1e22);
  border: 1px solid var(--accent, #4a9eff);
  border-radius: 10px;
  padding: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  z-index: 10001;
  transition: all 0.3s ease;
}
.tour-tooltip-step {
  font-size: 11px;
  color: var(--accent, #4a9eff);
  font-weight: 700;
  margin-bottom: 6px;
}
.tour-tooltip-text {
  font-size: 13px;
  color: var(--text, #e4e4e8);
  line-height: 1.5;
  margin-bottom: 14px;
}
.tour-tooltip-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.tour-btn {
  font-size: 12px !important;
  padding: 4px 12px !important;
}
.welcome-tour-btn {
  margin-top: 8px;
  display: block;
  width: 100%;
  background: transparent;
  color: var(--accent, #4a9eff);
  border: 1px solid var(--accent, #4a9eff);
}
.welcome-tour-btn:hover {
  background: rgba(74, 158, 255, 0.1);
}

/* ── Undo/Redo buttons ── */
.undo-redo-btn {
  position: relative;
  min-width: 28px;
  padding: 2px 6px !important;
}
.undo-redo-btn.btn-disabled {
  opacity: 0.35;
  pointer-events: auto;
  cursor: default;
}
.undo-redo-icon {
  font-size: 14px;
  line-height: 1;
}
.undo-redo-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  background: var(--accent, #4a9eff);
  color: #fff;
  font-size: 9px;
  min-width: 14px;
  height: 14px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 3px;
  font-weight: 600;
  line-height: 1;
}

/* ── Gutter decorations ── */
:deep(.gutter-icon) {
  font-size: 9px;
  margin-right: 2px;
  opacity: 0.4;
  vertical-align: baseline;
}
:deep(.gutter-icon-module) {
  color: var(--hl-keyword, #5c9eff);
}
:deep(.gutter-icon-loop) {
  color: var(--hl-special, #56c8d8);
}
:deep(.gutter-icon-cond) {
  color: var(--hl-boolean, #c678dd);
  font-weight: bold;
  font-style: italic;
}

/* ── Snapshot Gallery ── */
.snapshot-gallery-panel {
  position: absolute;
  top: 10px;
  left: 10px;
  width: 340px;
  max-height: 400px;
  background: var(--surface, #1e1e22);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 10px;
  z-index: 20;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.snapshot-gallery-body {
  padding: 8px;
  overflow-y: auto;
  max-height: 350px;
}
.snapshot-empty {
  color: var(--text-dim, #888);
  text-align: center;
  padding: 20px;
  font-size: 12px;
}
.snapshot-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.snapshot-item {
  border: 1px solid var(--border, #2e2e34);
  border-radius: 6px;
  overflow: hidden;
  background: var(--bg, #141416);
}
.snapshot-thumb {
  width: 100%;
  height: auto;
  display: block;
  cursor: pointer;
  transition: opacity 0.15s;
}
.snapshot-thumb:hover {
  opacity: 0.8;
}
.snapshot-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 6px;
  font-size: 10px;
  color: var(--text-dim, #888);
}
.snapshot-time {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.snapshot-del {
  background: none;
  border: none;
  color: #f44;
  cursor: pointer;
  font-size: 14px;
  padding: 0 2px;
  line-height: 1;
}
.snapshot-del:hover {
  color: #ff6666;
}
.snapshot-preview-modal {
  position: relative;
  max-width: 90vw;
  max-height: 90vh;
}
.snapshot-preview-close {
  position: absolute;
  top: -10px;
  right: -10px;
  z-index: 1;
  background: var(--surface, #1e1e22);
  border-radius: 50%;
  width: 28px;
  height: 28px;
  font-size: 16px;
}
.snapshot-preview-img {
  max-width: 90vw;
  max-height: 85vh;
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.5);
}

/* ── Profile panel ── */
.profile-panel {
  position: absolute;
  bottom: 50px;
  left: 10px;
  width: 350px;
  max-height: 300px;
  background: var(--surface, #1e1e22);
  border: 1px solid var(--border, #2e2e34);
  border-radius: 10px;
  z-index: 20;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.profile-panel-body {
  padding: 8px;
  overflow-y: auto;
  max-height: 260px;
}
.profile-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}
.profile-table th {
  text-align: left;
  padding: 3px 6px;
  border-bottom: 1px solid var(--border, #2e2e34);
  color: var(--text-dim, #888);
  font-weight: 500;
  font-size: 10px;
  text-transform: uppercase;
}
.profile-row {
  cursor: pointer;
  transition: background 0.1s;
}
.profile-row:hover {
  background: var(--hover, #28282e);
}
.profile-row td {
  padding: 3px 6px;
  border-bottom: 1px solid rgba(255,255,255,0.04);
}
.profile-name {
  color: var(--hl-keyword, #5c9eff);
  font-family: var(--editor-font-family, monospace);
}
.profile-line {
  color: var(--text-dim, #888);
  text-align: center;
}
.profile-time {
  text-align: right;
  color: var(--text, #e4e4e8);
  font-family: var(--editor-font-family, monospace);
}
.profile-slow {
  color: #ff6b5a;
  font-weight: 600;
}

/* ── Zen Mode ── */
.zen-mode .topbar { display: none !important; }
.zen-mode .editor-panel { display: none !important; }
.zen-mode .divider { display: none !important; }
.zen-mode .main { display: flex !important; }
.zen-mode .canvas-panel { flex: 1 !important; width: 100% !important; }
.zen-mode .vp-toolbar { opacity: 0.3; transition: opacity 0.2s; }
.zen-mode .vp-toolbar:hover { opacity: 1; }
.zen-mode .vp-zoom-controls { opacity: 0.3; transition: opacity 0.2s; }
.zen-mode .vp-zoom-controls:hover { opacity: 1; }
.zen-exit-btn {
  position: fixed;
  top: 10px;
  right: 10px;
  z-index: 9999;
  padding: 6px 16px;
  border-radius: 6px;
  background: rgba(0,0,0,0.7);
  color: #fff;
  border: 1px solid rgba(255,255,255,0.2);
  font-size: 12px;
  cursor: pointer;
  opacity: 0.3;
  transition: opacity 0.2s;
}
.zen-exit-btn:hover {
  opacity: 1;
  background: rgba(0,0,0,0.9);
}

/* ── Tip of the Day ── */
.tip-of-day-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  background: var(--accent, #4a9eff);
  color: #fff;
  font-size: 12px;
  flex-shrink: 0;
}
.tip-icon { font-size: 14px; }
.tip-label { font-weight: 600; white-space: nowrap; }
.tip-text { flex: 1; }
.tip-next-btn, .tip-dismiss-btn, .tip-close-btn {
  background: rgba(255,255,255,0.2);
  border: none;
  color: #fff;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
  white-space: nowrap;
}
.tip-next-btn:hover, .tip-dismiss-btn:hover, .tip-close-btn:hover {
  background: rgba(255,255,255,0.35);
}
.tip-close-btn { font-size: 14px; padding: 0 4px; }
.tip-slide-enter-active, .tip-slide-leave-active {
  transition: max-height 0.3s ease, opacity 0.3s ease;
  overflow: hidden;
}
.tip-slide-enter-from, .tip-slide-leave-to {
  max-height: 0;
  opacity: 0;
}
.tip-slide-enter-to, .tip-slide-leave-from {
  max-height: 50px;
  opacity: 1;
}

/* ── Object Tree: Eye Icon & Color Swatch ── */
.tree-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.tree-eye-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-dim, #888);
  padding: 1px 2px;
  line-height: 1;
  display: flex;
  align-items: center;
  opacity: 0.7;
  transition: opacity 0.15s;
}
.tree-eye-btn:hover { opacity: 1; }
.tree-eye-hidden { opacity: 0.3 !important; }
.tree-eye-hidden:hover { opacity: 0.5 !important; }
.tree-color-swatch {
  width: 16px;
  height: 16px;
  border: 1px solid var(--border, #2e2e34);
  border-radius: 3px;
  padding: 0;
  cursor: pointer;
  -webkit-appearance: none;
  appearance: none;
}
.tree-color-swatch::-webkit-color-swatch-wrapper { padding: 0; }
.tree-color-swatch::-webkit-color-swatch { border: none; border-radius: 2px; }
.tree-node {
  display: flex;
  align-items: center;
}

/* ── Section Box axis label ── */
.clip-axis-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim, #888);
  min-width: 14px;
  text-align: center;
}

/* ── Error Explanation Panel ── */
.error-main {
  display: flex;
  align-items: center;
  gap: 8px;
}
.error-main span {
  flex: 1;
}
.error-help-btn {
  background: rgba(255,255,255,0.15);
  border: 1px solid rgba(255,255,255,0.25);
  color: inherit;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
.error-help-btn:hover {
  background: rgba(255,255,255,0.25);
}
.error-explanation {
  margin-top: 8px;
  padding: 8px 10px;
  background: rgba(0,0,0,0.2);
  border-radius: 6px;
  font-size: 12px;
  line-height: 1.5;
}
.error-expl-row {
  margin-bottom: 4px;
}
.error-expl-row:last-child {
  margin-bottom: 0;
}
.error-expl-row strong {
  color: #ffcc00;
}

/* ── Custom Theme Editor ── */
.pref-color-input {
  width: 48px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: transparent;
  cursor: pointer;
  padding: 1px;
  -webkit-appearance: none;
  appearance: none;
}
.pref-color-input::-webkit-color-swatch-wrapper { padding: 0; }
.pref-color-input::-webkit-color-swatch { border: none; border-radius: 3px; }

/* ── Playground Modal ── */
.playground-modal {
  max-width: 500px;
}
.playground-progress {
  display: flex;
  gap: 8px;
  justify-content: center;
  margin-bottom: 16px;
}
.playground-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--border);
  transition: background 0.2s;
}
.playground-dot.active {
  background: var(--accent);
  box-shadow: 0 0 6px var(--accent);
}
.playground-dot.completed {
  background: #4caf50;
}
.playground-challenge-num {
  font-size: 16px;
  font-weight: 700;
  margin-bottom: 12px;
  color: var(--accent);
}
.playground-goal {
  margin-bottom: 10px;
  font-size: 14px;
  line-height: 1.5;
}
.playground-hint {
  margin-bottom: 16px;
  font-size: 13px;
  color: var(--text-dim);
}
.playground-hint code {
  background: var(--surface);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}
.playground-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}
.playground-complete {
  font-size: 18px;
  font-weight: 700;
  color: #4caf50;
  text-align: center;
  margin-bottom: 16px;
}
</style>
