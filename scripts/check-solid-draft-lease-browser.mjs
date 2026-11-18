// Draft writes without Web Locks (IndexedDB lease) and the conflict copies dialog in headless Chromium.
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { loadQualificationPlaywrightPackage } from './qualificationPlaywrightPackage.mjs'
const root = path.resolve('dist'),
  directory = path.resolve(process.argv[2] ?? '/tmp/solid-draft-lease')
await mkdir(directory, { recursive: true })
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://localhost'),
      file = path.resolve(root, '.' + (url.pathname === '/' ? '/index.html' : decodeURIComponent(url.pathname)))
    if (!file.startsWith(root + path.sep)) {
      res.writeHead(403).end()
      return
    }
    res.setHeader(
      'Content-Type',
      file.endsWith('.html')
        ? 'text/html'
        : file.endsWith('.js')
          ? 'text/javascript'
          : file.endsWith('.css')
            ? 'text/css'
            : file.endsWith('.wasm')
              ? 'application/wasm'
              : 'application/octet-stream',
    )
    res.end(await readFile(file))
  } catch {
    res.writeHead(404).end()
  }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
let browser, page
const errors = []
try {
  const { playwright } = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({ headless: true, channel: 'chromium' })
  const context = await browser.newContext({ acceptDownloads: true, viewport: { width: 1440, height: 1100 } })
  // Every page runs as a browser without Web Locks, so draft writes take the IndexedDB lease path.
  await context.addInitScript(() => {
    Object.defineProperty(Navigator.prototype, 'locks', { configurable: true, get: () => undefined })
  })
  const origin = `http://127.0.0.1:${server.address().port}`
  const open = async () => {
    const p = await context.newPage()
    p.on('pageerror', e => errors.push(String(e)))
    await p.goto(origin)
    return p
  }
  page = await open()
  assert.equal(await page.evaluate(() => navigator.locks), undefined)
  const region = p => p.getByRole('region', { name: 'Solid — CAD-лепка', exact: true })
  const menu = p => region(p).locator('summary[title="Файл"]')
  async function closeMenu(p) {
    if (await menu(p).evaluate(e => e.parentElement.open)) await menu(p).click()
  }
  async function openMenu(p) {
    if (await menu(p).evaluate(e => !e.parentElement.open)) await menu(p).click()
  }
  async function command(p, name) {
    await closeMenu(p)
    await p.getByRole('button', { name: 'Команды', exact: true }).click()
    const search = p.getByRole('combobox', { name: 'Search commands / Поиск команд' })
    await search.fill(name)
    await search.press('Enter')
  }
  async function saved(p) {
    await region(p).getByRole('status', { name: 'history-restore', exact: true }).waitFor({ state: 'hidden' })
    await region(p).getByRole('status', { name: 'primitive-build', exact: true }).waitFor({ state: 'hidden' })
    await region(p).getByRole('status', { name: 'Сохранено в браузере', exact: true }).waitFor()
    await p.waitForFunction(() => !document.body.innerText.includes('Восстанавливаю геометрию'))
  }
  async function exportDoc(p, file) {
    await openMenu(p)
    const download = p.waitForEvent('download')
    await region(p).getByRole('button', { name: 'Скачать проект JSON', exact: true }).click()
    const item = await download
    await item.saveAs(path.join(directory, file))
    await closeMenu(p)
    return JSON.parse(await readFile(path.join(directory, file), 'utf8'))
  }
  const normalize = d => ({ ...d, curves: d.curves ?? [], surfaces: d.surfaces ?? [] })
  await openMenu(page)
  await region(page)
    .locator('input[accept=".json,application/json"]')
    .setInputFiles({
      name: 'empty.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify({ version: 1, sketches: [], bodies: [] })),
    })
  await closeMenu(page)
  await saved(page)
  const key = await page.evaluate(() =>
    ['scad-main-modeler-v1', 'scad-solid-modeler-v1'].find(k => localStorage.getItem(k)),
  )
  assert.ok(key)
  const other = await open()
  await saved(other)
  // A foreign lease row stands in for a live holder; both tabs must wait on it, not write past it.
  const lease = (p, expires) =>
    p.evaluate(
      ([name, expires]) =>
        new Promise((resolve, reject) => {
          const r = indexedDB.open('scad-solid-draft-leases-v1', 1)
          r.onupgradeneeded = () => r.result.createObjectStore('leases', { keyPath: 'name' })
          r.onerror = () => reject(r.error)
          r.onsuccess = () => {
            const db = r.result,
              tx = db.transaction('leases', 'readwrite')
            if (expires === null) tx.objectStore('leases').delete(name)
            else tx.objectStore('leases').put({ name, owner: 'test-holder', expires: Date.now() + expires })
            tx.oncomplete = () => {
              db.close()
              resolve()
            }
            tx.onerror = () => reject(tx.error)
          }
        }),
      ['solid-draft:' + key, expires],
    )
  await lease(page, 60_000)
  const headBefore = await page.evaluate(key => localStorage.getItem(key), key)
  await Promise.all([command(page, 'Box'), command(other, 'Sphere')])
  await Promise.all([
    region(page).getByRole('status', { name: 'Сохраняю…', exact: true }).waitFor(),
    region(other).getByRole('status', { name: 'Сохраняю…', exact: true }).waitFor(),
  ])
  await page.waitForTimeout(500)
  assert.equal(await page.evaluate(key => localStorage.getItem(key), key), headBefore, 'a write passed a held lease')
  await lease(page, null)
  await Promise.all([
    page.waitForFunction(() => !document.querySelector('[aria-label="Сохраняю…"]')),
    other.waitForFunction(() => !document.querySelector('[aria-label="Сохраняю…"]')),
  ])
  const conflict = async p => (await p.locator('body').innerText()).includes('Документ изменён в другой вкладке')
  const firstConflict = await conflict(page),
    secondConflict = await conflict(other)
  assert.notEqual(firstConflict, secondConflict, 'exactly one writer must conflict')
  const loser = firstConflict ? page : other,
    winner = firstConflict ? other : page
  const canonical = normalize(JSON.parse(await page.evaluate(key => localStorage.getItem(key), key)))
  assert.deepEqual(await exportDoc(winner, 'winner.json'), canonical)
  const local = await exportDoc(loser, 'loser-local.json')
  assert.notDeepEqual(local, canonical)
  assert.equal(local.bodies.length, 1)
  // An abandoned lease (crashed holder) expires and the next write goes through.
  await lease(winner, 1500)
  const winnerBodies = canonical.bodies.length
  await command(winner, 'Cylinder')
  await saved(winner)
  const afterExpiry = normalize(JSON.parse(await winner.evaluate(key => localStorage.getItem(key), key)))
  assert.equal(afterExpiry.bodies.length, winnerBodies + 1)
  // The rejected text of the loser is listed in the conflict copies dialog.
  await openMenu(loser)
  await region(loser).getByRole('button', { name: 'Конфликтные копии черновика…', exact: true }).click()
  const dialog = loser.getByRole('dialog', { name: 'Конфликтные копии черновика' })
  await dialog.waitFor()
  await loser.waitForFunction(() => !document.querySelector('.draft-conflict-copies')?.innerText.includes('Загрузка…'))
  const copies = dialog.locator('[data-conflict-copy]')
  assert.ok((await copies.count()) >= 1, 'conflict copy missing')
  assert.match(await copies.first().innerText(), /\d/)
  const download = loser.waitForEvent('download')
  await copies.first().getByRole('button', { name: 'Скачать JSON', exact: true }).click()
  const item = await download
  await item.saveAs(path.join(directory, 'conflict-copy.json'))
  const copyDoc = normalize(JSON.parse(await readFile(path.join(directory, 'conflict-copy.json'), 'utf8')))
  assert.equal(copyDoc.bodies.length, local.bodies.length)
  await loser.screenshot({ path: path.join(directory, 'conflict-copies.png') })
  await dialog.getByRole('button', { name: 'Закрыть', exact: true }).last().click()
  await dialog.waitFor({ state: 'hidden' })
  // Restore makes the copy the open document through local history; Undo returns to the previous model.
  await openMenu(loser)
  await region(loser).getByRole('button', { name: 'Конфликтные копии черновика…', exact: true }).click()
  await dialog.waitFor()
  await loser.waitForFunction(() => !document.querySelector('.draft-conflict-copies')?.innerText.includes('Загрузка…'))
  await copies.first().getByRole('button', { name: 'Восстановить', exact: true }).click()
  await loser.waitForFunction(() => !document.querySelector('[aria-label="history-restore"]'))
  if (await dialog.isVisible()) await dialog.getByRole('button', { name: 'Закрыть', exact: true }).last().click()
  const restored = await exportDoc(loser, 'restored.json')
  assert.equal(restored.bodies.length, copyDoc.bodies.length)
  assert.deepEqual(
    restored.bodies.map(b => b.kind ?? b.type),
    copyDoc.bodies.map(b => b.kind ?? b.type),
  )
  // Delete removes the copy from the list.
  await openMenu(loser)
  await region(loser).getByRole('button', { name: 'Конфликтные копии черновика…', exact: true }).click()
  await dialog.waitFor()
  await loser.waitForFunction(() => !document.querySelector('.draft-conflict-copies')?.innerText.includes('Загрузка…'))
  const count = await copies.count()
  await copies.first().getByRole('button', { name: 'Удалить', exact: true }).click()
  await loser.waitForFunction(n => document.querySelectorAll('[data-conflict-copy]').length === n - 1, count)
  await dialog.getByRole('button', { name: 'Закрыть', exact: true }).last().click()
  assert.deepEqual(errors, [])
  const report = {
    browser: browser.version(),
    key,
    webLocks: false,
    leaseExcludesWriters: true,
    exactlyOneConflict: true,
    expiredLeaseTakenOver: true,
    conflictCopyListed: true,
    conflictCopyDownloaded: true,
    conflictCopyRestored: true,
    conflictCopyDeleted: true,
  }
  await writeFile(path.join(directory, 'draft-lease-browser.json'), JSON.stringify(report, null, 2) + '\n')
  console.log(report)
} catch (error) {
  if (page && !page.isClosed()) {
    await page.screenshot({ path: path.join(directory, 'failure.png') }).catch(() => {})
    await writeFile(
      path.join(directory, 'failure.txt'),
      await page
        .locator('body')
        .innerText()
        .catch(() => ''),
    )
  }
  throw error
} finally {
  await browser?.close()
  await new Promise(resolve => server.close(resolve))
}
