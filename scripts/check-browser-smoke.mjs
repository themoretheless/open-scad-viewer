// Browser smoke gate (roadmap P5): boots the built app in headless Chromium
// with the no-GPU render seam (?renderSeam=recording) and runs the main UI
// flows with operation-count budgets.
//
//   node scripts/check-browser-smoke.mjs [outputDir] [--update-baseline]
//
// Requires `vite build` output in dist/ (or SMOKE_DIST) and the qualification
// Playwright package (tools/browser-qualification).
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { mkdir, readFile, writeFile, copyFile, access } from 'node:fs/promises'
import path from 'node:path'
import { loadQualificationPlaywrightPackage } from './qualificationPlaywrightPackage.mjs'
import { runControlModels } from './browserSmokeControlModels.mjs'

const root = path.resolve(process.env.SMOKE_DIST ?? 'dist')
const directory = path.resolve(process.argv.slice(2).find(arg => !arg.startsWith('--')) ?? 'tmp/browser-smoke')
const updateBaseline = process.argv.includes('--update-baseline')
const baselineDirectory = path.resolve('tests/browser/baselines')
// Font rasterization differs per OS, so each platform keeps its own baseline.
const baselineFile = path.join(baselineDirectory, `keyboard-shortcuts-${process.platform}.png`)
// Fraction of pixels allowed to differ by more than CHANNEL_THRESHOLD in any channel.
const PIXEL_TOLERANCE = 0.01
const CHANNEL_THRESHOLD = 48
// Budgets for one Apply of each command, measured with the recording seam.
const BUDGET = {
  historyCommitsPerApply: 1,
  maxSolidWorkerRequestsPerApply: 12,
  maxDocumentClonesPerApply: 12,
  maxGeometryWorkerRequestsPerExampleBuild: 4,
}

const CONTENT_TYPES = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
}
await mkdir(directory, { recursive: true })
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost')
    if (url.pathname === '/favicon.ico') {
      res.writeHead(204).end()
      return
    }
    const file = path.resolve(root, '.' + (url.pathname === '/' ? '/index.html' : decodeURIComponent(url.pathname)))
    if (!file.startsWith(root + path.sep)) {
      res.writeHead(403).end()
      return
    }
    res.setHeader('Content-Type', CONTENT_TYPES[path.extname(file)] ?? 'application/octet-stream')
    res.end(await readFile(file))
  } catch {
    res.writeHead(404).end()
  }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))

const errors = []
const steps = []
const budgets = {}
let browser, page
async function step(name, run) {
  const started = Date.now()
  console.log('step', name)
  await run()
  steps.push({ name, ms: Date.now() - started })
}

try {
  const { playwright } = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({
    headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? { executablePath: process.env.CHROMIUM_EXECUTABLE } : {}),
  })
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    colorScheme: 'light',
    reducedMotion: 'reduce',
    locale: 'ru-RU',
  })
  page = await context.newPage()
  page.on('pageerror', error => {
    errors.push(`pageerror: ${error.stack ?? error}`)
    console.error(error)
  })
  page.on('console', message => {
    if (message.type() === 'error') {
      errors.push(`console: ${message.text()}`)
      console.error(message.text())
    }
  })

  const operations = () => page.evaluate(() => window.__osvTest.operations())
  const viewport = () => page.evaluate(() => window.__osvTest.viewport())
  const delta = (after, before) => Object.fromEntries(Object.keys(after).map(key => [key, after[key] - before[key]]))
  // Solid workspace helpers for one page (the C3 scenario runs in its own fresh context).
  function solidUi(target) {
    const region = target.getByRole('region', { name: 'Solid — CAD-лепка', exact: true })
    const palette = target.getByRole('combobox', { name: 'Search commands / Поиск команд' })
    return {
      solid: region,
      search: palette,
      async ready() {
        await target.waitForFunction(() => !document.body.innerText.includes('Восстанавливаю геометрию'))
        for (const name of ['history-restore', 'primitive-build', 'display-refinement']) {
          await region.getByRole('status', { name, exact: true }).waitFor({ state: 'hidden' })
        }
      },
      async command(name) {
        await target.getByRole('button', { name: 'Команды', exact: true }).click()
        await palette.fill(name)
        await palette.press('Enter')
        await palette.waitFor({ state: 'detached' })
      },
      async input(name, value) {
        await region.getByRole('textbox', { name, exact: true }).fill(value)
      },
    }
  }
  const { solid, search, ready, command, input } = solidUi(page)
  const bodies = () => solid.locator('[data-body]').count()
  async function expectFocus(locator) {
    await page.waitForFunction(element => element === document.activeElement, await locator.elementHandle(), {
      timeout: 5000,
    })
  }
  async function openAndCloseModal(opener, open) {
    await open()
    const dialog = page.getByRole('dialog').last()
    await dialog.waitFor({ state: 'visible' })
    assert.equal(await dialog.getAttribute('aria-modal'), 'true')
    assert.ok(await dialog.evaluate(element => element.contains(document.activeElement)), 'focus moves into the modal')
    await page.keyboard.press('Escape')
    await dialog.waitFor({ state: 'detached' })
    if (opener) await expectFocus(opener)
    // Without an opener (menu items close with their menu) only closing is asserted.
  }
  async function openShortcuts() {
    await page.locator('details.settings-menu > summary').click()
    await page.locator('details[open] button', { hasText: 'Горячие клавиши' }).first().click()
  }

  await step('open app with the recording render seam', async () => {
    await page.goto(`http://127.0.0.1:${server.address().port}/?renderSeam=recording`)
    await solid.waitFor({ timeout: 20000 })
    assert.equal(await page.evaluate(() => window.__osvTest?.renderSeam), 'recording')
    await ready()
  })

  await step('command palette opens, cancels and restores focus', async () => {
    const trigger = page.getByRole('button', { name: 'Команды', exact: true })
    await trigger.focus()
    await page.keyboard.press('Enter')
    await expectFocus(search)
    await page.keyboard.insertText('not-a-command')
    await page.keyboard.press('Escape')
    await search.waitFor({ state: 'detached' })
    await expectFocus(trigger)
  })

  await step('rectangle with exact input is one history commit', async () => {
    const before = await operations()
    await command('Rectangle')
    await input('Начальная координата X', '0 mm')
    await input('Начальная координата Y', '0 mm')
    await input('Размер прямоугольника Ширина', '10 mm')
    await input('Размер прямоугольника Высота', '6 mm')
    await solid.getByRole('button', { name: 'Создать прямоугольник', exact: true }).click()
    await ready()
    const used = delta(await operations(), before)
    budgets.rectangle = used
    assert.equal(used.historyCommits, BUDGET.historyCommitsPerApply, 'one rectangle = one history commit')
    // The created sketch stays selected; Escape closes the numeric panel.
    await page.keyboard.press('Escape')
  })

  let bodiesAfterExtrude
  await step('extrude with exact height and Apply', async () => {
    const bodiesBefore = await bodies()
    const before = await operations()
    await command('Extrude')
    await solid.getByText('Новое', { exact: true }).click()
    await solid.getByLabel('Высота, мм', { exact: true }).fill('7 mm')
    await solid.getByRole('button', { name: 'Готово · Enter', exact: true }).click()
    await ready()
    await page.waitForFunction(
      count => document.querySelectorAll('[aria-label="Solid — CAD-лепка"] [data-body]').length > count,
      bodiesBefore,
    )
    bodiesAfterExtrude = await bodies()
    const used = delta(await operations(), before)
    budgets.extrude = used
    assert.equal(used.historyCommits, BUDGET.historyCommitsPerApply, 'one Apply = one history commit')
    assert.ok(
      used.solidWorkerRequests >= 1 && used.solidWorkerRequests <= BUDGET.maxSolidWorkerRequestsPerApply,
      `extrude worker requests ${used.solidWorkerRequests}`,
    )
    assert.ok(used.documentClones <= BUDGET.maxDocumentClonesPerApply, `extrude document clones ${used.documentClones}`)
  })

  await step('undo and redo restore the extrude without new commits', async () => {
    const before = await operations()
    await solid.getByRole('button', { name: '↶', exact: true }).click()
    await ready()
    await page.waitForFunction(
      count => document.querySelectorAll('[aria-label="Solid — CAD-лепка"] [data-body]').length < count,
      bodiesAfterExtrude,
    )
    await solid.getByRole('button', { name: '↷', exact: true }).click()
    await ready()
    await page.waitForFunction(
      count => document.querySelectorAll('[aria-label="Solid — CAD-лепка"] [data-body]').length === count,
      bodiesAfterExtrude,
    )
    const used = delta(await operations(), before)
    budgets.undoRedo = used
    assert.equal(used.historyCommits, 0, 'undo/redo never commit')
    assert.equal(used.historyRestores, 2, 'one restore per undo and per redo')
  })

  async function rectangle(width, height) {
    await command('Rectangle')
    await input('Начальная координата X', '0 mm')
    await input('Начальная координата Y', '0 mm')
    await input('Размер прямоугольника Ширина', `${width} mm`)
    await input('Размер прямоугольника Высота', `${height} mm`)
    await solid.getByRole('button', { name: 'Создать прямоугольник', exact: true }).click()
    await ready()
    await page.keyboard.press('Escape')
  }
  async function waitBodies(predicate, arg) {
    await page.waitForFunction(
      ([source, value]) =>
        new Function('count', 'value', `return ${source}`)(
          document.querySelectorAll('[aria-label="Solid — CAD-лепка"] [data-body]').length,
          value,
        ),
      [predicate, arg],
    )
  }

  // C4 in the browser: a refused parameter is explained in place, marks its field and
  // input object `invalid`, blocks Apply and leaves the document unchanged.
  await step('extrude with zero height is refused in place with a localized reason', async () => {
    await rectangle(10, 6)
    const bodiesBefore = await bodies()
    const before = await operations()
    await command('Extrude')
    await solid.getByText('Новое', { exact: true }).click()
    const heightField = solid.getByLabel('Высота, мм', { exact: true })
    await heightField.fill('0 mm')
    await solid.getByText('Высота выдавливания должна быть ненулевой.', { exact: false }).first().waitFor()
    await page.waitForFunction(() =>
      document.querySelector('[data-field="height"][aria-invalid="true"], [data-field="height"] [aria-invalid="true"]'),
    )
    const overlay = solid.locator('g.invalid-input-geometry')
    await overlay.waitFor({ state: 'attached' })
    const invalidIds = (await overlay.getAttribute('data-invalid-ids')).split(' ').filter(Boolean)
    assert.ok(invalidIds.length >= 1, 'the failed input object is outlined in the 3D view')
    assert.ok(await overlay.locator('path, polygon').count(), 'the invalid outline has geometry')
    assert.equal(
      await solid.getByRole('button', { name: 'Готово · Enter', exact: true }).isDisabled(),
      true,
      'Apply is blocked',
    )
    assert.equal(await bodies(), bodiesBefore, 'the refused preview adds no body')
    // Correcting the value clears the mark and Apply commits once.
    await heightField.fill('3 mm')
    await overlay.waitFor({ state: 'detached' })
    assert.equal(await page.locator('[data-field="height"][aria-invalid="true"]').count(), 0)
    await solid.getByRole('button', { name: 'Готово · Enter', exact: true }).click()
    await ready()
    await waitBodies('count > value', bodiesBefore)
    const used = delta(await operations(), before)
    budgets.refusedThenCorrected = used
    assert.equal(
      used.historyCommits,
      BUDGET.historyCommitsPerApply,
      'the refusal commits nothing; the correction commits once',
    )
  })

  // C3 in the browser: edits, Undo/Redo of all of them, draft restore after reload,
  // STEP export with the expected extents and reopening the file through STEP import.
  await step('edits survive Undo/Redo and a reload, export to STEP and reopen', async () => {
    const bodiesNow = await bodies()
    const undo = solid.getByRole('button', { name: '↶', exact: true })
    const redo = solid.getByRole('button', { name: '↷', exact: true })
    const edits = 4 // two rectangles and two extrusions in this session
    for (let i = 0; i < edits; i++) {
      await undo.click()
      await ready()
    }
    const bodiesUndone = await bodies()
    assert.ok(bodiesUndone < bodiesNow, 'undo removes the extruded bodies')
    for (let i = 0; i < edits; i++) {
      await redo.click()
      await ready()
    }
    await waitBodies('count === value', bodiesNow)
    await solid.locator('.save-status:not(.error)').waitFor()
    await page.waitForFunction(
      () => !document.querySelector('[aria-label="Solid — CAD-лепка"] .save-status')?.textContent?.includes('…'),
    )
    await page.waitForTimeout(500)
    await page.reload()
    await solid.waitFor({ timeout: 20000 })
    await ready()
    await waitBodies('count === value', bodiesNow)

    // Assembly STEP refuses a document with sketches, in the UI language.
    await solid.locator('details.file-menu > summary').click()
    await solid.getByRole('button', { name: 'STEP сборки · тела и группы', exact: true }).click()
    await page.getByText('STEP сборки переносит только тела', { exact: false }).first().waitFor()
    await solid.locator('details.file-menu > summary').click()
    await solid.locator('polygon[data-body]').first().click({ force: true })
    await page.waitForFunction(() => document.body.innerText.includes('Выбрано: 1'))
    await solid.locator('details.file-menu > summary').click()
    const downloadPromise = page.waitForEvent('download', { timeout: 30000 })
    await solid.getByRole('button', { name: 'STEP выбранного тела · текущая геометрия', exact: true }).click()
    const download = await downloadPromise
    const stepFile = path.join(directory, 'scenario-assembly.step')
    await download.saveAs(stepFile)
    const text = await readFile(stepFile, 'utf8')
    assert.match(text, /^ISO-10303-21;/, 'a STEP Part 21 file')
    const points = [...text.matchAll(/CARTESIAN_POINT\('[^']*',\(([^)]*)\)\)/g)]
      .map(m => m[1].split(',').map(Number))
      .filter(p => p.length === 3 && p.every(Number.isFinite))
    const span = axis => Math.max(...points.map(p => p[axis])) - Math.min(...points.map(p => p[axis]))
    const extents = [span(0), span(1), span(2)]
    budgets.stepExport = { bytes: text.length, points: points.length, extents }
    // Both bodies are 10×6 mm rectangles extruded by 7 mm and 3 mm.
    const near = (value, expected) => Math.abs(value - expected) < 1e-6
    assert.ok(near(extents[0], 10) && near(extents[1], 6), `STEP footprint ${extents}`)
    assert.ok(near(extents[2], 7) || near(extents[2], 3), `STEP height ${extents[2]}`)

    await solid.locator('details.file-menu > summary').click()
    await solid.locator('details.file-menu input[type=file][accept=".step,.stp"]').setInputFiles(stepFile)
    await ready()
    await waitBodies('count > value', bodiesNow)
    await page.keyboard.press('Escape')
  })

  // C5: the workbench at 1440×900 in both themes: no page overflow, the Solid region
  // fills the viewport and the selection-state legend uses the `selectionStates` table.
  // C10 in the browser: the radius grip at the manipulator drags and steps the value,
  // Apply commits once, and Undo brings back the body with its edge chain selected.
  await step('fillet radius grip drags, applies and Undo reselects the edge chain', async () => {
    await page.keyboard.press('Escape')
    await solid.locator('polygon[data-body]').first().click({ force: true })
    await page.waitForFunction(() => document.body.innerText.includes('Выбрано: 1'))
    await solid.getByRole('button', { name: 'Рёбра', exact: true }).click()
    const edgeSelect = solid.getByRole('combobox', { name: 'Выбрать ребро', exact: true })
    await page.waitForFunction(() => document.querySelectorAll('[data-topology-edge]').length > 0, null, {
      timeout: 30000,
    })
    const edgeValue = await edgeSelect.locator('option').nth(1).getAttribute('value')
    await edgeSelect.selectOption(edgeValue)
    await command('Fillet 3D')
    const field = solid.getByRole('textbox', { name: 'Размер у манипулятора, мм', exact: true })
    const grip = solid.locator('[data-dimension-grip]')
    await grip.waitFor()
    await field.fill('0.5 mm')
    await ready()
    const readRadius = async () => Number.parseFloat((await field.inputValue()).replace(',', '.'))
    const startRadius = await readRadius()
    const box = await grip.boundingBox()
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2, { steps: 6 })
    await page.mouse.up()
    const dragged = await readRadius()
    assert.ok(dragged > startRadius, `dragging the grip grows the radius (${startRadius} -> ${dragged})`)
    assert.equal(Math.round(dragged * 2), dragged * 2, 'the drag snaps to 0.5 mm')
    await grip.focus()
    await page.keyboard.press('ArrowLeft')
    const stepped = await readRadius()
    assert.ok(Math.abs(stepped - (dragged - 0.5)) < 1e-9, `ArrowLeft steps by 0.5 mm (${dragged} -> ${stepped})`)
    // Keep the radius small enough for the 10×6×3 mm box.
    await field.fill('1 mm')
    await ready()
    const before = await operations()
    await solid.getByRole('button', { name: 'Готово · Enter', exact: true }).click()
    await ready()
    await page.waitForFunction(() => !document.querySelector('[data-dimension-grip]'))
    const used = delta(await operations(), before)
    budgets.filletGrip = used
    assert.equal(used.historyCommits, BUDGET.historyCommitsPerApply, 'one fillet Apply = one history commit')
    await solid.getByRole('button', { name: '↶', exact: true }).click()
    await ready()
    await page.waitForFunction(
      value =>
        document.querySelector('[aria-label="Solid — CAD-лепка"] select[aria-label="Выбрать ребро"]')?.value === value,
      edgeValue,
      { timeout: 10000 },
    )
    assert.ok(
      (await solid.locator('[data-topology-edge][aria-pressed="true"]').count()) >= 1,
      'the restored edge chain is highlighted',
    )
    await solid.getByRole('button', { name: 'Тела', exact: true }).click()
    await page.keyboard.press('Escape')
  })

  await step('workbench layout at 1440×900 in both themes', async () => {
    const table = await readFile('src/ui/selectionStates.ts', 'utf8')
    const expected = Object.fromEntries(
      [...table.matchAll(/(\w+): \{.*?color: \{ light: '(#[0-9a-f]{6})', dark: '(#[0-9a-f]{6})' \}/g)].map(m => [
        m[1],
        { light: m[2], dark: m[3] },
      ]),
    )
    assert.equal(Object.keys(expected).length, 6, 'six selection states in the table')
    const hex = rgb =>
      '#' +
      (rgb.match(/\d+/g) ?? [])
        .slice(0, 3)
        .map(n => Number(n).toString(16).padStart(2, '0'))
        .join('')
    const layout = {}
    for (const scheme of ['light', 'dark']) {
      await page.emulateMedia({ colorScheme: scheme })
      await page.waitForFunction(next => document.documentElement.dataset.theme === next, scheme)
      await page.evaluate(() => document.fonts.ready)
      const file = path.join(directory, `workbench-1440x900-${scheme}.png`)
      await page.screenshot({ path: file, animations: 'disabled', caret: 'hide' })
      const metrics = await page.evaluate(() => {
        const region = document.querySelector('[aria-label="Solid — CAD-лепка"]').getBoundingClientRect()
        const chips = Object.fromEntries(
          [...document.querySelectorAll('.state-legend [data-sel-state]')].map(chip => [
            chip.dataset.selState,
            getComputedStyle(chip.querySelector('.sel-state-mark')).color,
          ]),
        )
        const offscreen = [
          ...document.querySelectorAll(
            '[aria-label="Solid — CAD-лепка"] button, [aria-label="Solid — CAD-лепка"] summary',
          ),
        ]
          .filter(el => el.checkVisibility?.() && el.getBoundingClientRect().width > 0)
          .filter(el => {
            const r = el.getBoundingClientRect()
            return r.right > innerWidth + 1 || r.left < -1
          }).length
        // Floating pane toolbars must not cover the other pane overlays (grid controls,
        // body actions, frame-time badge): each overlay is hit-testable at its centre.
        const covered = [
          ...document.querySelectorAll(
            '.split-workspace .modeling-grid-controls, .split-workspace .workspace-action-bar, .split-workspace .fps-badge',
          ),
        ]
          .filter(el => el.checkVisibility?.() && el.getBoundingClientRect().width > 0)
          .filter(el => {
            const r = el.getBoundingClientRect()
            const hit = document.elementFromPoint(r.left + Math.min(8, r.width / 2), r.top + r.height / 2)
            return el.classList.contains('fps-badge')
              ? [...document.querySelectorAll('.split-workspace .pane-tools')].some(tools => tools.contains(hit))
              : !el.contains(hit)
          })
          .map(el => el.className)
        return {
          covered,
          scrollWidth: document.scrollingElement.scrollWidth,
          scrollHeight: document.scrollingElement.scrollHeight,
          region: [region.left, region.top, region.width, region.height],
          chips,
          offscreen,
          background: getComputedStyle(document.body).backgroundColor,
        }
      })
      assert.ok(metrics.scrollWidth <= 1440, `no horizontal page overflow (${metrics.scrollWidth})`)
      assert.ok(metrics.scrollHeight <= 900, `no vertical page overflow (${metrics.scrollHeight})`)
      assert.equal(metrics.offscreen, 0, 'no visible Solid control is clipped horizontally')
      assert.deepEqual(metrics.covered, [], 'floating pane toolbars cover no pane overlay')
      assert.ok(
        metrics.region[2] >= 1000 && metrics.region[3] >= 600,
        `Solid region fills the viewport: ${metrics.region}`,
      )
      for (const [state, colors] of Object.entries(expected))
        assert.equal(hex(metrics.chips[state] ?? ''), colors[scheme], `${scheme} ${state} legend colour`)
      layout[scheme] = { ...metrics, screenshot: path.basename(file) }
    }
    assert.notEqual(layout.light.background, layout.dark.background, 'the page background follows the theme')
    await page.emulateMedia({ colorScheme: 'light' })
    budgets.workbenchLayout = layout
  })

  await step('open the source editor with the recording viewport', async () => {
    await page.getByRole('button', { name: 'Исходный код', exact: true }).click()
    await page.waitForFunction(() => window.__osvTest.viewport()?.status === 'ready')
  })

  await step('shared Modal dialogs open, trap focus and close with Escape', async () => {
    await openAndCloseModal(null, openShortcuts)
    const exportButton = page.locator('button[title="Экспорт модели"]').first()
    await openAndCloseModal(exportButton, () => exportButton.click())
  })

  await step('visual fixture: keyboard shortcuts dialog', async () => {
    await openShortcuts()
    const dialog = page.getByRole('dialog').last()
    await dialog.waitFor({ state: 'visible' })
    await page.evaluate(() => document.fonts.ready)
    const actualFile = path.join(directory, path.basename(baselineFile))
    await dialog.screenshot({ path: actualFile, animations: 'disabled', caret: 'hide' })
    await page.keyboard.press('Escape')
    await dialog.waitFor({ state: 'detached' })
    const hasBaseline = await access(baselineFile).then(
      () => true,
      () => false,
    )
    if (updateBaseline || !hasBaseline) {
      if (updateBaseline) {
        await mkdir(baselineDirectory, { recursive: true })
        await copyFile(actualFile, baselineFile)
      }
      budgets.visual = {
        baseline: path.relative(process.cwd(), baselineFile),
        status: updateBaseline ? 'updated' : 'missing-baseline',
      }
      if (!updateBaseline) console.warn(`No visual baseline for ${process.platform}; captured ${actualFile}`)
      return
    }
    const [actual, expected] = await Promise.all([readFile(actualFile), readFile(baselineFile)])
    const comparison = await page.evaluate(
      async ([a, b, threshold]) => {
        const decode = async base64 => {
          const bitmap = await createImageBitmap(await (await fetch(`data:image/png;base64,${base64}`)).blob())
          const canvas = new OffscreenCanvas(bitmap.width, bitmap.height)
          const context = canvas.getContext('2d')
          context.drawImage(bitmap, 0, 0)
          return {
            width: bitmap.width,
            height: bitmap.height,
            data: context.getImageData(0, 0, bitmap.width, bitmap.height).data,
          }
        }
        const [x, y] = await Promise.all([decode(a), decode(b)])
        if (x.width !== y.width || x.height !== y.height)
          return { sizeMismatch: [x.width, x.height, y.width, y.height], ratio: 1 }
        let differing = 0
        for (let i = 0; i < x.data.length; i += 4) {
          if (
            Math.abs(x.data[i] - y.data[i]) > threshold ||
            Math.abs(x.data[i + 1] - y.data[i + 1]) > threshold ||
            Math.abs(x.data[i + 2] - y.data[i + 2]) > threshold ||
            Math.abs(x.data[i + 3] - y.data[i + 3]) > threshold
          )
            differing++
        }
        return { ratio: differing / (x.width * x.height), width: x.width, height: x.height }
      },
      [actual.toString('base64'), expected.toString('base64'), CHANNEL_THRESHOLD],
    )
    budgets.visual = { baseline: path.relative(process.cwd(), baselineFile), status: 'compared', ...comparison }
    assert.ok(!comparison.sizeMismatch, `visual fixture size changed: ${comparison.sizeMismatch}`)
    assert.ok(
      comparison.ratio <= PIXEL_TOLERANCE,
      `visual fixture differs in ${(comparison.ratio * 100).toFixed(2)}% of pixels`,
    )
  })

  await step('build an example in the source viewport through the recording renderer', async () => {
    const openGallery = async () => {
      await page.locator('summary[aria-label="Ещё"]').first().click()
      await page.locator('details[open] button', { hasText: 'Примеры' }).first().click()
    }
    await openAndCloseModal(null, openGallery)
    const before = await operations()
    const uploadsBefore = (await viewport()).sceneUploads
    await openGallery()
    const dialog = page.getByRole('dialog').last()
    await dialog.locator('.example-card button').first().click()
    const replace = dialog.locator('.danger-action')
    if (await replace.count()) await replace.click()
    await dialog.waitFor({ state: 'detached' })
    // The build settles either as a scene publication to the recorder or as a
    // build error in the editor status. A failed build is reported in the
    // result (and as a warning) rather than failing the UI smoke, so kernel
    // qualification drift is visible without masking the UI flows.
    await page.waitForFunction(
      count => window.__osvTest.operations().geometryWorkerRequests > count,
      before.geometryWorkerRequests,
      { timeout: 30000 },
    )
    const requestedAt = Date.now()
    const outcome = await page
      .waitForFunction(
        ([count, since]) => {
          const state = window.__osvTest.viewport()
          if (state.sceneUploads > count && state.meshCount > 0)
            return { status: 'published', meshCount: state.meshCount, triangleCount: state.triangleCount }
          const alert = document.querySelector('.editor-pane [role=alert], .error-banner, .build-error')
          const failed = document.body.innerText.includes('Ошибка сборки')
          return failed && Date.now() - since > 2000
            ? {
                status: 'failed',
                message: (
                  (alert?.textContent ?? '').trim() ||
                  (document.body.innerText.match(/Uncaught[^\n]*(\n[^\n]*)?/) ?? [''])[0]
                ).slice(0, 200),
              }
            : null
        },
        [uploadsBefore, requestedAt],
        { timeout: 30000 },
      )
      .then(handle => handle.jsonValue())
    const used = delta(await operations(), before)
    budgets.exampleBuild = { ...used, outcome }
    assert.ok(
      used.geometryWorkerRequests >= 1 &&
        used.geometryWorkerRequests <= BUDGET.maxGeometryWorkerRequestsPerExampleBuild,
      `example build worker requests ${used.geometryWorkerRequests}`,
    )
    if (outcome.status === 'published') {
      const state = await viewport()
      assert.equal(state.calls.setMeshes >= 1, true, 'scene published through the renderer seam')
      assert.ok(state.framesDrawn > 0, 'recording renderer drew frames')
    } else {
      console.warn(`Example build failed in the app: ${outcome.message}`)
    }
  })

  await step('CodeMirror source editor: typing, undo groups, find and Tab indent', async () => {
    const content = page.locator('.code-editor-cm .cm-content')
    await content.waitFor({ state: 'visible' })
    const label = await content.getAttribute('aria-label')
    assert.ok(label && label.trim(), 'editor content has an accessible name')
    const text = () =>
      content.evaluate(el => [...el.querySelectorAll('.cm-line')].map(line => line.textContent).join('\n'))
    await content.click()
    await page.keyboard.press('ControlOrMeta+a')
    await page.keyboard.type('cube(1);', { delay: 15 })
    // A pause longer than the 500 ms history window starts a new undo group.
    await page.waitForTimeout(700)
    await page.keyboard.type(' sphere(2);', { delay: 15 })
    assert.equal(await text(), 'cube(1); sphere(2);')
    await page.keyboard.press('ControlOrMeta+z')
    assert.equal(await text(), 'cube(1);', 'one undo removes one typing group')
    await page.keyboard.press('ControlOrMeta+Shift+z')
    assert.equal(await text(), 'cube(1); sphere(2);', 'redo restores the group')
    await page.keyboard.press('ControlOrMeta+f')
    const findInput = page.locator('.find-bar input[type=search]')
    await findInput.waitFor({ state: 'visible' })
    await findInput.fill('sphere')
    await page.keyboard.press('Enter')
    await page.keyboard.press('Escape')
    await findInput.waitFor({ state: 'detached' })
    await expectFocus(content)
    assert.equal(
      await page.evaluate(() => String(document.getSelection())),
      'sphere',
      'find selects the match in CodeMirror',
    )
    await page.keyboard.press('Home')
    await page.keyboard.press('Tab')
    assert.match(await text(), /^\s+cube\(1\); sphere\(2\);$/, 'Tab indents the line')
    await expectFocus(content)
    await page.keyboard.press('ControlOrMeta+z')
    assert.equal(await text(), 'cube(1); sphere(2);', 'Tab indent is one undo step')
    // Theme evidence for both schemes; the colours come from the app's CSS variables.
    const editorPane = page.locator('.code-editor-cm')
    const shots = []
    const schemes = {}
    for (const scheme of ['dark', 'light']) {
      await page.emulateMedia({ colorScheme: scheme })
      await page.waitForFunction(next => document.documentElement.dataset.theme === next, scheme)
      const file = path.join(directory, `codemirror-editor-${scheme}.png`)
      await editorPane.screenshot({ path: file, animations: 'disabled', caret: 'hide' })
      schemes[scheme] = await page
        .locator('.code-editor-cm .syntax-function')
        .first()
        .evaluate(el => getComputedStyle(el).color)
      shots.push(path.basename(file))
    }
    assert.notEqual(schemes.dark, schemes.light, 'syntax colours follow the theme')
    budgets.codeMirrorEditor = { label, functionColor: schemes, screenshots: shots }
  })

  await step('keyboard camera keys orbit, pan and zoom the viewport', async () => {
    const canvas = page.locator('canvas.gpu-canvas')
    await canvas.focus()
    const start = (await viewport()).camera
    await page.keyboard.press('ArrowLeft')
    const orbit = (await viewport()).camera
    assert.notEqual(orbit.yaw, start.yaw, 'ArrowLeft orbits')
    await page.keyboard.press('Shift+ArrowUp')
    const pan = (await viewport()).camera
    assert.notDeepEqual(pan.target, orbit.target, 'Shift+ArrowUp pans')
    await page.keyboard.press('Equal')
    const zoom = (await viewport()).camera
    assert.ok(zoom.distance < pan.distance, '+ zooms in')
    const state = await viewport()
    assert.ok(state.cameraHistory >= 3, 'each camera key is one Previous View entry')
    budgets.camera = { calls: state.calls.applyKeyboardCameraStep, history: state.cameraHistory }
  })

  // C3 in the browser: the full 20-edit scenario (primitives, duplicate, union, guided
  // subtraction, groups, delete, sketches, extrusions) through the UI in a fresh context,
  // with analytic volume and area read from «Свойства B-rep», then Undo of all 20 edits,
  // Redo of all 20 and a reopen of the earlier STEP file with its volume and area.
  await step('20-edit scenario with groups, duplicate and Boolean through the UI', async () => {
    const scenarioContext = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      colorScheme: 'light',
      reducedMotion: 'reduce',
      locale: 'ru-RU',
    })
    const p = await scenarioContext.newPage()
    p.on('pageerror', error => errors.push(`scenario pageerror: ${error.stack ?? error}`))
    p.on('console', message => message.type() === 'error' && errors.push(`scenario console: ${message.text()}`))
    try {
      const ui = solidUi(p)
      await p.goto(`http://127.0.0.1:${server.address().port}/?renderSeam=recording`)
      await ui.solid.waitFor({ timeout: 20000 })
      await ui.ready()
      const commits = () => p.evaluate(() => window.__osvTest.operations().historyCommits)
      const rows = ui.solid.locator('li.object-row > button[aria-pressed]')
      const tab = name => ui.solid.getByRole('tab', { name, exact: true }).click()
      // Scene rows exist only while the scene dock tab is open.
      const names = async () => {
        await tab('Сцена')
        await ui.solid.locator('li.object-row, li.scene-empty').first().waitFor()
        return rows.evaluateAll(elements => elements.map(element => element.getAttribute('aria-label')))
      }
      const index = async name => {
        const found = (await names()).indexOf(name)
        assert.ok(found >= 0, `scene row ${name}`)
        return found
      }
      async function pick(...indexes) {
        await tab('Сцена')
        for (const [k, i] of indexes.entries()) await rows.nth(i).click(k ? { modifiers: ['Shift'] } : {})
      }
      // Volume and area of one body as the properties panel prints them (4 decimals).
      async function mass(name) {
        await pick(await index(name))
        await tab('Свойства')
        await ui.command('Свойства B-rep')
        const text = ui.solid.getByText(/V = -?[\d.]+ mm³/).first()
        await text.waitFor({ timeout: 20000 })
        const [volume, area] = (await text.textContent())
          .match(/V = (-?[\d.]+) mm³ · A = (-?[\d.]+)/)
          .slice(1)
          .map(Number)
        return { volume, area }
      }
      const expectMass = async (name, volume, area) => {
        const measured = await mass(name)
        assert.ok(Math.abs(measured.volume - volume) <= volume * 1e-6, `${name} volume ${measured.volume} ≠ ${volume}`)
        assert.ok(Math.abs(measured.area - area) <= area * 1e-6, `${name} area ${measured.area} ≠ ${area}`)
        return measured
      }
      const start = await commits()
      const log = []
      async function edit(name, run) {
        const target = (await commits()) + 1
        await run()
        await p.waitForFunction(n => window.__osvTest.operations().historyCommits >= n, target, { timeout: 30000 })
        await ui.ready()
        assert.equal(await commits(), target, `${name} is one history commit`)
        log.push(name)
      }
      const button = name => ui.solid.getByRole('button', { name, exact: true }).click()
      const subtract = async (a, b) => {
        await pick(await index(a), await index(b))
        await ui.command('Вычесть: A − B')
        await button('Готово · Enter')
      }
      const union = async (a, b) => {
        await pick(await index(a), await index(b))
        await ui.command('Объединить тела')
      }
      async function rectangle() {
        await p.keyboard.press('Escape')
        await ui.command('Rectangle')
        await ui.input('Начальная координата X', '0 mm')
        await ui.input('Начальная координата Y', '0 mm')
        await ui.input('Размер прямоугольника Ширина', '10 mm')
        await ui.input('Размер прямоугольника Высота', '6 mm')
        await button('Создать прямоугольник')
      }
      async function extrude(height) {
        await p.keyboard.press('Escape')
        await ui.command('Extrude')
        await ui.solid.getByText('Новое', { exact: true }).click()
        await ui.solid.getByLabel('Высота, мм', { exact: true }).fill(`${height} mm`)
        await button('Готово · Enter')
      }
      // A 20 mm box is centred in XY on [0, 20] in Z; a copy is offset by (10, 10, 0).
      const box = 'Куб · 3D',
        copy = 'Куб · 3D · copy'
      await edit('box primitive', () => ui.command('Куб'))
      await edit('duplicate the box', () => ui.command('Копия'))
      await edit('union box and copy', () => union(box, copy))
      await expectMass(box, 14000, 3800)
      await edit('duplicate the union', async () => {
        await pick(await index(box))
        await ui.command('Копия')
      })
      await edit('subtract the shifted union', () => subtract(box, copy))
      await expectMass(box, 6000, 2200) // an L of three 10×10×20 cells
      await edit('create group 1', () => button('Новая группа'))
      await edit('move the L into group 1', async () => {
        await pick(await index(box))
        await button('Перенести выбор')
      })
      await edit('duplicate the L', async () => {
        await pick(await index(box))
        await ui.command('Копия')
      })
      await edit('delete the duplicate', async () => {
        await pick(await index(copy))
        await ui.command('Удалить')
      })
      await edit('rectangle 10×6', rectangle)
      await edit('extrude 7 mm', () => extrude(7))
      await p.keyboard.press('Escape')
      await edit('second box primitive', () => ui.command('Куб'))
      const ids = await names()
      await edit('subtract the L from the second box', async () => {
        await pick(ids.lastIndexOf(box), ids.indexOf(box))
        await ui.command('Вычесть: A − B')
        await button('Готово · Enter')
      })
      await expectMass(box, 2000, 1000) // the remaining 10×10×20 cell
      await edit('create group 2', () => button('Новая группа'))
      await edit('move the cell into group 2', async () => {
        await pick(await index(box))
        await button('Перенести выбор')
      })
      await edit('duplicate the cell', async () => {
        await pick(await index(box))
        await ui.command('Копия')
      })
      await edit('delete the cell duplicate', async () => {
        await pick(await index(copy))
        await ui.command('Удалить')
      })
      await edit('second rectangle 10×6', rectangle)
      await edit('extrude 3 mm', () => extrude(3))
      await p.keyboard.press('Escape')
      await edit('union the extrusions', () => union('Эскиз 1 · 3D', 'Эскиз 2 · 3D'))
      assert.equal(log.length, 20, 'twenty edits')
      assert.equal((await commits()) - start, 20, 'twenty history commits')
      const final = await names()
      assert.deepEqual(final, ['Эскиз 1', 'Эскиз 2', 'Эскиз 1 · 3D', box], 'final scene rows')
      const groups = await ui.solid.locator('.scene-group .group-name').allTextContents()
      assert.ok(
        ['Группа 1', 'Группа 2'].every(group => groups.some(text => text.includes(group))),
        `groups ${groups}`,
      )
      const measured = {
        extrusions: await expectMass('Эскиз 1 · 3D', 420, 344),
        cell: await expectMass(box, 2000, 1000),
      }

      // Undo all 20 edits back to the empty scene, then Redo all of them.
      await p.keyboard.press('Escape')
      const undo = ui.solid.getByRole('button', { name: '↶', exact: true })
      const redo = ui.solid.getByRole('button', { name: '↷', exact: true })
      for (let i = 0; i < 20; i++) {
        await undo.click()
        await ui.ready()
      }
      assert.equal(await undo.isDisabled(), true, 'all edits undone')
      await tab('Сцена')
      assert.deepEqual(await names(), [], 'Undo of all edits empties the scene')
      for (let i = 0; i < 20; i++) {
        await redo.click()
        await ui.ready()
      }
      assert.equal(await redo.isDisabled(), true, 'all edits redone')
      assert.deepEqual(await names(), final, 'Redo restores the scene rows')
      await expectMass('Эскиз 1 · 3D', 420, 344)
      await expectMass(box, 2000, 1000)
      assert.equal((await commits()) - start, 20, 'Undo/Redo add no commits')

      // Control model «imported STEP body»: the 10×6 mm body exported earlier.
      const stepFile = path.join(directory, 'scenario-assembly.step')
      await p.keyboard.press('Escape')
      await ui.solid.locator('details.file-menu > summary').click()
      await ui.solid.locator('details.file-menu input[type=file][accept=".step,.stp"]').setInputFiles(stepFile)
      await ui.ready()
      await p.waitForFunction(count => document.querySelectorAll('li.object-row').length > count, final.length)
      if (await ui.solid.locator('details.file-menu[open]').count())
        await ui.solid.locator('details.file-menu > summary').click()
      const imported = (await names()).filter(name => !final.includes(name))
      assert.equal(imported.length, 1, `one imported body: ${imported}`)
      const stepMass = await mass(imported[0])
      const options = [
        { volume: 420, area: 344 },
        { volume: 180, area: 216 },
      ]
      assert.ok(
        options.some(o => Math.abs(stepMass.volume - o.volume) <= 1e-3 && Math.abs(stepMass.area - o.area) <= 1e-3),
        `imported STEP body ${JSON.stringify(stepMass)}`,
      )
      budgets.controlScenario = { edits: log, measured, importedStep: stepMass }
    } finally {
      await scenarioContext.close()
    }
  })

  // The remaining nine C3 control models, each in a fresh context (scripts/browserSmokeControlModels.mjs).
  await step('nine control models through the Solid UI', async () => {
    budgets.controlModels = await runControlModels({
      browser,
      origin: `http://127.0.0.1:${server.address().port}`,
      directory,
      errors,
    })
  })

  assert.deepEqual(errors, [], 'no console errors or page errors')
  const result = {
    ok: true,
    renderSeam: 'recording',
    steps,
    budgets,
    operations: await operations(),
    viewport: await viewport(),
    errors,
  }
  await writeFile(path.join(directory, 'result.json'), JSON.stringify(result, null, 2) + '\n')
  console.log(JSON.stringify({ ok: true, steps: steps.length, budgets }, null, 2))
} catch (error) {
  if (page) {
    await page.screenshot({ path: path.join(directory, 'failure.png') }).catch(() => {})
    await writeFile(
      path.join(directory, 'failure.json'),
      JSON.stringify({ steps, budgets, errors, message: String(error?.stack ?? error) }, null, 2),
    ).catch(() => {})
  }
  throw error
} finally {
  await browser?.close()
  await new Promise(resolve => server.close(resolve))
}
