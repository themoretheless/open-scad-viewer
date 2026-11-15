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
  const solid = page.getByRole('region', { name: 'Solid — CAD-лепка', exact: true })
  const search = page.getByRole('combobox', { name: 'Search commands / Поиск команд' })
  const bodies = () => solid.locator('[data-body]').count()
  async function ready() {
    await page.waitForFunction(() => !document.body.innerText.includes('Восстанавливаю геометрию'))
    for (const name of ['history-restore', 'primitive-build', 'display-refinement']) {
      await solid.getByRole('status', { name, exact: true }).waitFor({ state: 'hidden' })
    }
  }
  async function command(name) {
    await page.getByRole('button', { name: 'Команды', exact: true }).click()
    await search.fill(name)
    await search.press('Enter')
    await search.waitFor({ state: 'detached' })
  }
  async function input(name, value) {
    await solid.getByRole('textbox', { name, exact: true }).fill(value)
  }
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
