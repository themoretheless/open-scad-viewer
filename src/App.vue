<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { parseOpenSCADWithAST } from './services/openscadParser'
import type { MeshData, ASTNode } from './services/openscadParser'
import { WebGPURenderer } from './services/webgpuRenderer'
import { exportSTL } from './services/stlExport'
import { exportOBJ } from './services/objExport'
import { export3MF } from './services/threemfExport'
import { parseSTL } from './services/stlImport'

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
  {
    id: 'high-contrast', name: { ru: 'Высокий контраст', en: 'High Contrast' }, dark: true,
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

const COLOR_GRADING_FILTERS: Record<string, string> = {
  none: '',
  warm: 'sepia(0.15) saturate(1.2)',
  cool: 'hue-rotate(10deg) saturate(0.9) brightness(1.05)',
  vintage: 'sepia(0.3) contrast(1.1) brightness(0.95)',
  noir: 'grayscale(1) contrast(1.3)',
  vivid: 'saturate(1.6) contrast(1.1)',
}

const canvasFilter = computed(() => COLOR_GRADING_FILTERS[colorGrading.value] || '')

/* ── Code Folding ── */
const foldedLines = ref<Set<number>>(new Set())
const foldRanges = ref<Map<number, number>>(new Map()) // startLine -> endLine

/* ── Color Picker ── */
const colorPickerVisible = ref(false)
const colorPickerX = ref(0)
const colorPickerY = ref(0)
const colorPickerValue = ref('#ffffff')
let colorPickerMatch: { start: number; end: number } | null = null

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
  addToast(t('prefsReset'), 'success')
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
  try {
    const raw = localStorage.getItem(HISTORY_STORAGE_KEY)
    if (raw) return JSON.parse(raw)
  } catch { /* ignore */ }
  return {}
}

function saveHistories(h: Record<string, TabHistoryEntry[]>) {
  try {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(h))
  } catch { /* quota exceeded - trim old entries */ }
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
}

const flatTree = computed(() => {
  const result: FlatTreeNode[] = []
  function walk(nodes: ASTNode[], depth: number) {
    for (const n of nodes) {
      if (n.name === '__assign') continue
      const hasChildren = n.children.length > 0
      result.push({ node: n, depth, hasChildren })
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

  hoverDocContent.value = { sig: doc.sig, desc: doc.desc[lang.value] || doc.desc.en }
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
    if (i === errL) {
      nums.push(`${diffPrefix}${prefix}<span class="line-error">${i}</span>`)
    } else {
      nums.push(`${diffPrefix}${prefix}${i}`)
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
  renderer.screenshot()
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
  const bindings = SHORTCUT_PRESETS[shortcutPreset.value]
  // Ctrl+Shift+P or F1 or preset command palette: Command Palette
  if (e.key === 'F1' || ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'P') || matchesBinding(e, bindings.commandPalette)) {
    e.preventDefault()
    if (showCommandPalette.value) closeCommandPalette()
    else openCommandPalette()
    return
  }
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
  // Escape to close modal or exit fullscreen
  if (e.key === 'Escape') {
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
  // Initialize undo stack with current code
  pushUndoSnapshot(code.value)
  doRender()

  // Start axis label updates
  updateAxisLabels()

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
})

onUnmounted(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  document.removeEventListener('click', onDocClick)
  if (debounce) clearTimeout(debounce)
  if (minimapDebounce) clearTimeout(minimapDebounce)
  if (statsInterval) clearInterval(statsInterval)
  if (axisLabelRAF) cancelAnimationFrame(axisLabelRAF)
  if (gizmoRAF) cancelAnimationFrame(gizmoRAF)
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

function doRender() {
  if (!renderer) return
  error.value = ''
  errorLine.value = -1
  errorCharPos.value = -1
  try {
    const t0 = performance.now()
    // Inject $t animation variable
    const codeWithT = code.value.replace(/\$t\b/g, String(animT.value))
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
    const tParsed = performance.now()
    perfParseTime.value = Math.round(tParsed - t0)
    meshCount.value = meshes.length
    triCount.value = meshes.reduce((s: number, m: MeshData) => s + m.indices.length / 3, 0)
    lastParsedMeshes = meshes
    const tMeshStart = performance.now()
    renderer.setMeshes(meshes)
    const tMeshEnd = performance.now()
    perfMeshGenTime.value = Math.round(tParsed - t0)
    perfGpuUploadTime.value = Math.round(tMeshEnd - tMeshStart)
    const t1 = performance.now()
    renderTime.value = Math.round(t1 - t0)
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

  const editorEl = e.target as HTMLTextAreaElement

  // Duplicate line: Ctrl+D
  if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
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

  if (key === 'w' || key === 'ц') {
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
  win.document.write(`<!DOCTYPE html><html><head><title>${t('shortcutsTitle')}</title><style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 24px; color: #222; }
    h1 { font-size: 1.3rem; margin-bottom: 16px; }
    .shortcut-category { font-weight: 700; font-size: 0.9rem; margin-top: 16px; margin-bottom: 6px; color: #555; text-transform: uppercase; letter-spacing: 0.04em; }
    .shortcut-row { display: flex; justify-content: space-between; padding: 3px 0; font-size: 0.85rem; border-bottom: 1px solid #eee; }
    kbd { background: #f0f0f0; border: 1px solid #ccc; border-radius: 3px; padding: 1px 6px; font-size: 0.78rem; font-family: monospace; }
    @media print { body { padding: 0; } }
  </style></head><body><h1>${t('shortcutsTitle')}</h1>${el.innerHTML}</body></html>`)
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
  { id: 'cameraInfo', label: () => t('showCameraInfo'), action: () => toggleCameraInfo() },
  { id: 'scadReference', label: () => t('cmdScadReference'), action: () => toggleScadReference() },
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

/* ── Feature 5: Custom-Resolution PNG Export ── */
const pngScale = ref(parseInt(localStorage.getItem('scad-png-scale') || '1') || 1)

function exportPng(scale: number) {
  if (!renderer) return
  pngScale.value = scale
  localStorage.setItem('scad-png-scale', String(scale))
  if (scale <= 1) {
    renderer.screenshot()
    addToast(t('screenshot'), 'success')
  } else {
    renderer.screenshotScaled(scale)
    addToast(t('exportPng') + ' ' + scale + '×', 'success')
  }
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
}
function toggleRenderMenu() {
  const was = renderMenuOpen.value
  closeAllMenus()
  renderMenuOpen.value = !was
}
function toggleExportMenu() {
  const was = exportMenuOpen.value
  closeAllMenus()
  exportMenuOpen.value = !was
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
}

function loadBookmarks(): CameraBookmark[] {
  try {
    const raw = localStorage.getItem('scad-bookmarks')
    if (raw) return JSON.parse(raw) as CameraBookmark[]
  } catch { /* ignore */ }
  return []
}

const cameraBookmarks = ref<CameraBookmark[]>(loadBookmarks())

function saveBookmarksToStorage() {
  localStorage.setItem('scad-bookmarks', JSON.stringify(cameraBookmarks.value))
}

function saveCameraBookmark() {
  if (!renderer) return
  const bk: CameraBookmark = {
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 4),
    name: `Cam ${cameraBookmarks.value.length + 1}`,
    yaw: renderer.yaw,
    pitch: renderer.pitch,
    dist: renderer.dist,
    tx: renderer.tx,
    ty: renderer.ty,
    tz: renderer.tz,
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

/* ── Feature: Export All Formats Dropdown (toolbar) ── */
const showExportDropdown = ref(false)

function toggleExportDropdown() {
  showExportDropdown.value = !showExportDropdown.value
}

function closeExportDropdown() {
  showExportDropdown.value = false
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
}
</script>

<template>
  <div class="app" :class="[isDark ? 'dark' : 'light', { 'app-loaded': appLoaded }]">
    <nav class="topbar">
      <div class="topbar-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
        </svg>
        <span class="brand topbar-brand-text">{{ t('title') }}</span>
      </div>
      <div class="topbar-right">
        <!-- Always visible: Render-relevant & essential controls -->
        <button class="tb-btn" @click="toggleLang" :aria-label="t('ariaToggleLang')">{{ lang === 'ru' ? 'RU' : 'EN' }}</button>
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
        <button class="tb-btn tb-btn-help topbar-collapsible" @click="showShortcuts = true" :title="t('shortcuts')" :aria-label="t('ariaHelp')">?</button>
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
          </div>
        </div>
      </div>
    </nav>

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
          <div class="modal-body scad-ref-scroll">
            <div v-for="section in SCAD_REF_SECTIONS" :key="section.key" class="scad-ref-section">
              <div class="scad-ref-section-title">{{ t(section.key) }}</div>
              <div v-for="entry in section.entries" :key="entry.name" class="scad-ref-entry">
                <div class="scad-ref-name">{{ entry.name }}</div>
                <code class="scad-ref-sig">{{ entry.sig }}</code>
                <div class="scad-ref-desc">{{ entry.desc[lang] || entry.desc.en }}</div>
              </div>
            </div>
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
            <div class="pref-footer">
              <button class="btn btn-sm pref-reset-btn" @click="resetPreferences">{{ t('resetPrefs') }}</button>
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
            <button class="btn btn-primary welcome-start-btn" @click="dismissWelcome">{{ t('welcomeStart') }}</button>
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
              <input
                type="color"
                :value="colorPickerValue"
                @input="onColorPickerChange"
                class="color-picker-input"
              />
              <button class="color-picker-close" @click="closeColorPicker">&times;</button>
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
          <div v-if="error" class="error">{{ error }}</div>
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
          @contextmenu="onCanvasContextMenu"
          @dragenter="onCanvasDragEnter"
          @dragover="onCanvasDragOver"
          @dragleave="onCanvasDragLeave"
          @drop="onCanvasDrop"
        />

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
            <button class="vp-menu-btn" @click="toggleViewMenu" :title="t('menuView')" :aria-label="t('menuView')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="viewMenuOpen" class="vp-dropdown">
              <button class="vp-dd-item" @click="setView('top'); closeAllMenus()">{{ t('top') }}</button>
              <button class="vp-dd-item" @click="setView('front'); closeAllMenus()">{{ t('front') }}</button>
              <button class="vp-dd-item" @click="setView('right'); closeAllMenus()">{{ t('right') }}</button>
              <button class="vp-dd-item" @click="setView('iso'); closeAllMenus()">{{ t('iso') }}</button>
              <button class="vp-dd-item" @click="setView('reset'); closeAllMenus()">{{ t('reset') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" @click="toggleProjection(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isOrthographic">&#10003;</span>
                {{ t('ortho') }}
              </button>
              <button class="vp-dd-item" @click="toggleFullscreen(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isFullscreen">&#10003;</span>
                {{ t('fullscreen') }}
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
              <div v-for="bk in cameraBookmarks" :key="bk.id" class="vp-dd-bookmark">
                <button class="vp-dd-item vp-dd-bk-name" @click="restoreBookmark(bk); closeAllMenus()">{{ bk.name }}</button>
                <button class="vp-dd-bk-del" @click.stop="deleteBookmark(bk.id)" :title="t('deleteBookmark')">&times;</button>
              </div>
            </div>
            </transition>
          </div>

          <!-- Render menu -->
          <div class="vp-menu-wrapper" @click.stop>
            <button class="vp-menu-btn" @click="toggleRenderMenu" :title="t('menuRender')" :aria-label="t('menuRender')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 16V8a2 2 0 0 0-1-1.7l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.7l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
                <path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="renderMenuOpen" class="vp-dropdown">
              <div class="vp-dd-label">{{ t('renderMode') }}</div>
              <select class="lighting-select vp-dd-select" :value="activeRenderMode" @change="setRenderMode(($event.target as HTMLSelectElement).value)">
                <option value="solid">{{ t('modeSolid') }}</option>
                <option value="solid+edges">{{ t('modeSolidEdges') }}</option>
                <option value="wireframe">{{ t('modeWireframe') }}</option>
                <option value="xray">{{ t('modeXray') }}</option>
              </select>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" @click="toggleFlatShading()">
                <span class="vp-dd-check" v-if="flatShadingEnabled">&#10003;</span>
                {{ t('flatShading') }}
              </button>
              <button class="vp-dd-item" @click="toggleGrid(); closeAllMenus()">
                <span class="vp-dd-check" v-if="showGrid">&#10003;</span>
                {{ t('grid') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleReflection(); closeAllMenus()">
                <span class="vp-dd-check" v-if="reflectionEnabled">&#10003;</span>
                {{ t('reflection') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleFog(); closeAllMenus()">
                <span class="vp-dd-check" v-if="fogEnabled">&#10003;</span>
                {{ t('fog') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleSSAO(); closeAllMenus()">
                <span class="vp-dd-check" v-if="ssaoEnabled">&#10003;</span>
                {{ t('ssao') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleOutline(); closeAllMenus()">
                <span class="vp-dd-check" v-if="outlineEnabled">&#10003;</span>
                {{ t('outline') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleNormalSmoothing(); closeAllMenus()">
                <span class="vp-dd-check" v-if="smoothNormalsEnabled">&#10003;</span>
                {{ t('smoothNormals') }}
              </button>
              <div class="vp-dd-sep" v-show="!simpleMode"></div>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleClip()">
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
              <button class="vp-dd-item" @click="toggleAutoRotate(); closeAllMenus()">
                <span class="vp-dd-check" v-if="isAutoRotate">&#10003;</span>
                {{ t('autoRotate') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleBuildPlate()">
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
              <button class="vp-dd-item" v-show="!simpleMode" @click="showObjectTree = !showObjectTree; closeAllMenus()">
                <span class="vp-dd-check" v-if="showObjectTree">&#10003;</span>
                {{ t('objectTree') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="toggleStatistics(); closeAllMenus()">
                <span class="vp-dd-check" v-if="showStatistics">&#10003;</span>
                {{ t('statistics') }}
              </button>
              <button class="vp-dd-item" v-show="!simpleMode" @click="showPerfPanel = !showPerfPanel; closeAllMenus()">
                <span class="vp-dd-check" v-if="showPerfPanel">&#10003;</span>
                {{ t('perfPanel') }}
              </button>
            </div>
            </transition>
          </div>

          <!-- Export menu -->
          <div class="vp-menu-wrapper" @click.stop>
            <button class="vp-menu-btn" @click="toggleExportMenu" :title="t('menuExport')" :aria-label="t('menuExport')">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
            <transition name="vp-dropdown">
            <div v-if="exportMenuOpen" class="vp-dropdown">
              <button class="vp-dd-item" @click="exportPng(1); closeAllMenus()">{{ t('screenshot') }} (1×)</button>
              <button class="vp-dd-item" @click="exportPng(2); closeAllMenus()">{{ t('screenshot') }} (2×)</button>
              <button class="vp-dd-item" @click="exportPng(4); closeAllMenus()">{{ t('screenshot') }} (4×)</button>
              <button class="vp-dd-item" @click="copyCanvasToClipboard(); closeAllMenus()">{{ t('copyImage') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" @click="doExportSTL(); closeAllMenus()">{{ t('exportStl') }}</button>
              <button class="vp-dd-item" @click="doExportOBJ(); closeAllMenus()">{{ t('exportObj') }}</button>
              <button class="vp-dd-item" @click="doExport3MF(); closeAllMenus()">{{ t('export3mf') }}</button>
              <div class="vp-dd-sep"></div>
              <button class="vp-dd-item" @click="openImportSTL(); closeAllMenus()">{{ t('importStl') }}</button>
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
  width: 44px; flex-shrink: 0;
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
  align-items: center;
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
</style>
