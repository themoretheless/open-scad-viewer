// C3 control models through the Solid UI (headless Chromium, `?renderSeam=recording`).
// Each model runs in a fresh browser context, is built only with palette commands, numeric
// panels, scene rows and the keyboard-focusable edge buttons of the 3D view, and is checked
// against the analytic volume / area of tests/controlModels.test.ts through «Свойства B-rep».
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { nurbsSurfaceMass } from './browserSmokeSurfaceOracle.mjs'

const PI = Math.PI

function solidUi(page) {
  const solid = page.getByRole('region', { name: 'Solid — CAD-лепка', exact: true })
  const palette = page.getByRole('combobox', { name: 'Search commands / Поиск команд' })
  const rows = solid.locator('li.object-row > button[aria-pressed]')
  const ui = {
    solid,
    async ready() {
      await page.waitForFunction(() => !document.body.innerText.includes('Восстанавливаю геометрию'))
      for (const name of ['history-restore', 'primitive-build', 'display-refinement'])
        await solid.getByRole('status', { name, exact: true }).waitFor({ state: 'hidden' })
    },
    async command(name) {
      await page.getByRole('button', { name: 'Команды', exact: true }).click()
      await palette.fill(name)
      await palette.press('Enter')
      await palette.waitFor({ state: 'detached' })
    },
    input: (name, value) => solid.getByRole('textbox', { name, exact: true }).fill(value),
    button: name => solid.getByRole('button', { name, exact: true }).click(),
    tab: name => solid.getByRole('tab', { name, exact: true }).click(),
    commits: () => page.evaluate(() => window.__osvTest.operations().historyCommits),
    async names() {
      await ui.tab('Сцена')
      await solid.locator('li.object-row, li.scene-empty').first().waitFor()
      return rows.evaluateAll(elements => elements.map(element => element.getAttribute('aria-label')))
    },
    // Select scene rows by name; later names are added with Shift.
    async pick(...names) {
      const all = await ui.names()
      for (const [k, name] of names.entries()) {
        const i = all.indexOf(name)
        assert.ok(i >= 0, `scene row ${name} in ${all}`)
        await rows.nth(i).click(k ? { modifiers: ['Shift'] } : {})
      }
    },
    // One user action must be exactly one history commit.
    async edit(name, run) {
      const target = (await ui.commits()) + 1
      await run()
      await page.waitForFunction(n => window.__osvTest.operations().historyCommits >= n, target, { timeout: 30000 })
      await ui.ready()
      assert.equal(await ui.commits(), target, `${name} is one history commit`)
    },
    async rectangle(x, y, w, h) {
      await page.keyboard.press('Escape')
      await ui.command('Rectangle')
      await ui.input('Начальная координата X', `${x} mm`)
      await ui.input('Начальная координата Y', `${y} mm`)
      await ui.input('Размер прямоугольника Ширина', `${w} mm`)
      await ui.input('Размер прямоугольника Высота', `${h} mm`)
      await ui.button('Создать прямоугольник')
    },
    async circle(x, y, r) {
      await page.keyboard.press('Escape')
      await ui.command('Circle')
      await ui.input('Координата точки X', `${x} mm`)
      await ui.input('Координата точки Y', `${y} mm`)
      await ui.input('Радиус круга', `${r} mm`)
      await ui.button('Создать круг')
    },
    async extrude(height, offset = 0) {
      await ui.command('Extrude')
      await solid.getByText('Новое', { exact: true }).click()
      await solid.getByLabel('Высота, мм', { exact: true }).fill(`${height} mm`)
      if (offset) await solid.getByLabel('Смещение от плоскости, мм', { exact: true }).fill(`${offset} mm`)
      await ui.button('Готово · Enter')
    },
    // Volume and area of one body as the properties panel prints them.
    async mass(name) {
      await page.keyboard.press('Escape')
      await ui.pick(name)
      await ui.tab('Свойства')
      await ui.command('Свойства B-rep')
      const text = solid.getByText(/V = -?[\d.]+ mm³/).first()
      await text.waitFor({ timeout: 20000 })
      const [volume, area] = (await text.textContent())
        .match(/V = (-?[\d.]+) mm³ · A = (-?[\d.]+)/)
        .slice(1)
        .map(Number)
      return { volume, area }
    },
  }
  return ui
}

const near = (value, expected, tolerance) => Math.abs(value - expected) <= tolerance * Math.max(1, Math.abs(expected))

// Each entry builds one model of tests/controlModels.test.ts and names the measured row.
const models = [
  {
    name: 'plate with two through holes',
    expected: { volume: (2400 - 32 * PI) * 5, area: 2 * (2400 - 32 * PI) + 200 * 5 + 2 * 8 * PI * 5 },
    async build(ui) {
      await ui.edit('rectangle 60×40', () => ui.rectangle(0, 0, 60, 40))
      await ui.edit('hole circle A', () => ui.circle(15, 20, 4))
      await ui.edit('hole circle B', () => ui.circle(45, 20, 4))
      const sketches = await ui.names()
      await ui.pick(...sketches)
      await ui.edit('extrude the plate with holes', () => ui.extrude(5))
      return (await ui.names()).find(name => !sketches.includes(name))
    },
  },
  {
    name: 'dimension-driven cylinder profile',
    expected: { volume: PI * 49 * 8, area: 2 * PI * 49 + 2 * PI * 7 * 8 },
    async build(ui, page) {
      await ui.edit('circle r 5', () => ui.circle(0, 0, 5))
      const [sketch] = await ui.names()
      await page.keyboard.press('Escape')
      await ui.pick(sketch)
      await ui.command('Размеры эскиза')
      const panel = ui.solid.getByRole('region', { name: 'Sketch dimensions', exact: true })
      await panel.getByRole('combobox', { name: 'Dimension kind', exact: true }).selectOption('diameter')
      await ui.edit('add a diameter dimension', () =>
        panel.getByRole('button', { name: 'Добавить размер', exact: true }).click(),
      )
      await ui.edit('diameter 14 mm', async () => {
        const field = panel.getByRole('textbox', { name: 'Dimension 0', exact: true })
        await field.fill('14 mm')
        await field.press('Enter')
      })
      await page.keyboard.press('Escape')
      await ui.pick(sketch)
      await ui.edit('extrude 8 mm', () => ui.extrude(8))
      return (await ui.names()).find(name => name !== sketch)
    },
  },
  {
    name: 'bracket block with filleted vertical edges',
    // Each fillet trades two 3 × 20 mm side strips and two (1 − π/4)·9 mm² corners for a quarter cylinder.
    expected: { volume: 24000 - (4 - PI) * 9 * 20, area: 5200 - 4 * (120 - 30 * PI + (2 - PI / 2) * 9) },
    async build(ui, page) {
      const body = await prism(ui, page, 40, 30, 20)
      await ui.pick(body)
      await ui.command('Выбирать рёбра')
      await pickVerticalEdges(ui, page, 4)
      await ui.command('Скруглить 3D')
      await ui.edit('fillet four vertical edges r 3', async () => {
        await ui.solid.getByRole('textbox', { name: 'Радиус / размер, мм', exact: true }).fill('3 mm')
        await ui.button('Готово · Enter')
      })
      return body
    },
  },
  {
    name: 'chamfered block',
    expected: { volume: 477, area: 376 - 12 + 6 * Math.SQRT2 - 1 },
    async build(ui, page) {
      const body = await prism(ui, page, 10, 8, 6)
      await ui.pick(body)
      await ui.command('Выбирать рёбра')
      await pickVerticalEdges(ui, page, 1)
      await ui.command('Фаска 3D')
      await ui.edit('chamfer one vertical edge 1 mm', async () => {
        await ui.solid.getByRole('textbox', { name: 'Радиус / размер, мм', exact: true }).fill('1 mm')
        await ui.button('Готово · Enter')
      })
      return body
    },
  },
  {
    name: 'shelled box (open-top enclosure)',
    expected: { volume: 7152, area: 5200 + 2 * (36 + 26) * 18 },
    async build(ui, page) {
      const outer = await prism(ui, page, 40, 30, 20)
      await ui.edit('cavity rectangle 36×26', () => ui.rectangle(2, 2, 36, 26))
      const before = await ui.names()
      const sketch = before.filter(name => name.startsWith('Эскиз') && !name.includes('3D')).at(-1)
      await page.keyboard.press('Escape')
      await ui.pick(sketch)
      await ui.edit('cavity extrude 20 mm from z 2', () => ui.extrude(20, 2))
      await page.keyboard.press('Escape')
      const cavity = (await ui.names()).find(name => !before.includes(name))
      await ui.pick(outer, cavity)
      await ui.edit('subtract the cavity', async () => {
        await ui.command('Вычесть: A − B')
        await ui.button('Готово · Enter')
      })
      return outer
    },
  },
  {
    name: 'revolved ring',
    expected: { volume: PI * 27 * 4, area: 2 * PI * 27 + 2 * PI * 9 * 4 },
    async build(ui, page) {
      await ui.edit('profile rectangle', () => ui.rectangle(3, 0, 3, 4))
      const [sketch] = await ui.names()
      await page.keyboard.press('Escape')
      await ui.pick(sketch)
      await ui.edit('revolve 360° about Y', async () => {
        await ui.command('Вращение')
        await ui.solid.getByText('Новое', { exact: true }).click()
        await ui.solid.getByLabel('Угол, °', { exact: true }).fill('360')
        await ui.solid.getByRole('combobox', { name: 'Поверхности вращения', exact: true }).selectOption('exact')
        await ui.button('Готово · Enter')
      })
      await page.keyboard.press('Escape')
      return (await ui.names()).find(name => name !== sketch)
    },
  },
  {
    // The live grid pattern has no palette command (only document transactions), so the six
    // pins come from six exact circles extruded together and are subtracted as one Boolean.
    name: 'plate with a 2 x 3 pattern of holes',
    expected: { volume: 60 * 40 * 4 - 6 * PI * 4 * 4, area: 2 * (2400 - 6 * 4 * PI) + 200 * 4 + 6 * 4 * PI * 4 },
    async build(ui, page) {
      const plate = await prism(ui, page, 60, 40, 4)
      for (const y of [10, 30])
        for (const x of [10, 30, 50]) await ui.edit(`pin circle ${x},${y}`, () => ui.circle(x, y, 2))
      await page.keyboard.press('Escape')
      const before = await ui.names()
      const circles = before.filter(name => name.startsWith('Эскиз') && !name.includes('3D')).slice(1)
      assert.equal(circles.length, 6)
      await ui.pick(...circles)
      await ui.edit('extrude six pins 6 mm from z −1', () => ui.extrude(6, -1))
      await page.keyboard.press('Escape')
      const pins = (await ui.names()).filter(name => !before.includes(name))
      assert.ok(pins.length >= 1, 'pin bodies')
      await ui.pick(plate, ...pins)
      await ui.edit('subtract the pins', async () => {
        await ui.command('Вычесть: A − B')
        await ui.button('Готово · Enter')
      })
      return plate
    },
  },
  {
    // Frustum of a 10 mm and a 6 mm square 10 mm apart: the top section is sketched on the
    // top face of a 10 mm helper prism chosen in the face list.
    name: 'lofted square transition',
    // Four planar trapezoids (10 and 6 mm sides, slant height √104 mm) close the two squares.
    expected: { volume: (10 / 3) * (100 + 36 + 60), area: 136 + 32 * Math.sqrt(104) },
    async build(ui, page) {
      const helper = await prism(ui, page, 10, 10, 10)
      await ui.pick(helper)
      await ui.solid.getByRole('button', { name: 'Грани', exact: true }).click()
      const faces = ui.solid.getByRole('combobox', { name: 'Выбрать грань', exact: true })
      const topFace = faces.locator('option', { hasText: '· 5, 5, 10 mm' })
      await topFace.waitFor({ state: 'attached' })
      const top = await topFace.textContent()
      assert.ok(top, 'top face in the face list')
      await faces.selectOption({ label: top })
      await ui.command('Эскиз на грани')
      await ui.solid.getByText('Плоскость: XY', { exact: true }).waitFor({ state: 'detached' })
      // The face workplane is anchored at a boundary corner with u along a boundary edge, so u
      // runs into the face and v = n × u points out of it: the 6 mm square sits 2 mm inside both
      // edges at local x ∈ [2, 8], y ∈ [−8, −2]. The ruled loft re-corresponds section winding
      // and start vertex itself, so the point order is free.
      await ui.edit('top square 6×6 on the face', async () => {
        await ui.command('Polyline')
        for (const [x, y] of [
          [8, -2],
          [8, -8],
          [2, -8],
          [2, -2],
        ]) {
          await ui.input('Координата точки X', `${x} mm`)
          await ui.input('Координата точки Y', `${y} mm`)
          await ui.button('Добавить точку')
        }
        await ui.button('Замкнуть контур')
      })
      await page.keyboard.press('Escape')
      const sketches = (await ui.names()).filter(name => name.startsWith('Эскиз') && !name.includes('3D'))
      assert.equal(sketches.length, 2)
      const before = await ui.names()
      await ui.pick(...sketches)
      await ui.edit('B-rep loft', async () => {
        await ui.command('B-rep loft')
        await ui.button('Готово · Enter')
      })
      await page.keyboard.press('Escape')
      return (await ui.names()).find(name => !before.includes(name))
    },
  },
  {
    name: 'swept rail surface',
    expected: {
      area: 500,
      bounds: [
        [0, 0, 0],
        [50, 10, 0],
      ],
    },
    async build(ui, page) {
      // Two default NURBS curves edited into straight lines through their control points.
      const lines = [
        [
          [0, 0, 0],
          [0, 2, 0],
          [0, 8, 0],
          [0, 10, 0],
        ],
        [
          [0, 0, 0],
          [10, 0, 0],
          [40, 0, 0],
          [50, 0, 0],
        ],
      ]
      const rows = ui.solid.locator('li.object-row > button[aria-pressed]')
      for (const [k, points] of lines.entries()) {
        await page.keyboard.press('Escape')
        await ui.edit(`NURBS curve ${k + 1}`, () => ui.command('NURBS-кривая'))
        for (const [i, point] of points.entries())
          await ui.edit(`curve ${k + 1} CV ${i}`, async () => {
            await ui.solid.getByRole('combobox', { name: 'U / CV' }).selectOption(String(i))
            for (const [axis, value] of ['X', 'Y', 'Z'].map((a, j) => [a, point[j]]))
              await ui.input(`CV ${axis}`, `${value} mm`)
            await ui.button('Применить CV')
          })
      }
      await page.keyboard.press('Escape')
      const before = await ui.names()
      assert.equal(before.length, 2, `two curves ${before}`)
      await rows.nth(0).click()
      await rows.nth(1).click({ modifiers: ['Shift'] })
      await ui.edit('sweep the profile along the path', async () => {
        await ui.command('Перенос профиля по пути')
        await ui.button('Готово · Enter')
      })
      await page.keyboard.press('Escape')
      return null
    },
    // A NURBS surface has no B-rep properties; the saved project is measured instead.
    async measure(ui, page) {
      await ui.solid.locator('details.file-menu > summary').click()
      const download = page.waitForEvent('download')
      await ui.button('Скачать проект JSON')
      const document = JSON.parse(await readFile(await (await download).path(), 'utf8'))
      await ui.solid.locator('details.file-menu > summary').click()
      assert.equal(document.surfaces?.length, 1, 'one swept surface')
      return nurbsSurfaceMass(document.surfaces[0].surface)
    },
  },
]

// A box as a B-rep prism: rectangle at the origin extruded by `h`; returns the body row.
async function prism(ui, page, w, d, h) {
  await ui.edit(`rectangle ${w}×${d}`, () => ui.rectangle(0, 0, w, d))
  const [sketch] = await ui.names()
  await page.keyboard.press('Escape')
  await ui.pick(sketch)
  await ui.edit(`extrude ${h} mm`, () => ui.extrude(h))
  await page.keyboard.press('Escape')
  return (await ui.names()).find(name => name !== sketch)
}

// Edge buttons of the 3D view; a vertical model edge projects to a vertical screen segment.
async function pickVerticalEdges(ui, page, count) {
  const edges = ui.solid.locator('polyline[data-topology-edge][role=button]')
  await edges.first().waitFor()
  const vertical = await edges.evaluateAll(list =>
    list.flatMap((element, i) => {
      const points = element
        .getAttribute('points')
        .trim()
        .split(/\s+/)
        .map(pair => pair.split(',').map(Number))
      const xs = points.map(p => p[0]),
        ys = points.map(p => p[1])
      const dx = Math.max(...xs) - Math.min(...xs),
        dy = Math.max(...ys) - Math.min(...ys)
      return dy > 0 && dx < dy * 1e-3 ? [i] : []
    }),
  )
  assert.ok(vertical.length >= count, `vertical edges ${vertical.length}`)
  for (const [k, i] of vertical.slice(0, count).entries()) {
    await edges.nth(i).focus()
    await page.keyboard.press(k ? 'Shift+Enter' : 'Enter')
  }
  assert.equal(await ui.solid.locator('polyline[data-topology-edge][aria-pressed=true]').count(), count)
}

export async function runControlModels({ browser, origin, directory, errors, only }) {
  const results = {}
  for (const model of models) {
    if (only && !model.name.includes(only)) continue
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      colorScheme: 'light',
      reducedMotion: 'reduce',
      locale: 'ru-RU',
    })
    const page = await context.newPage()
    page.on('pageerror', error => errors.push(`${model.name} pageerror: ${error.stack ?? error}`))
    page.on('console', m => m.type() === 'error' && errors.push(`${model.name} console: ${m.text()}`))
    try {
      const ui = solidUi(page)
      await page.goto(`${origin}/?renderSeam=recording`)
      await ui.solid.waitFor({ timeout: 20000 })
      await ui.ready()
      const row = await model.build(ui, page)
      assert.ok(row || model.measure, `${model.name}: result row`)
      const measured = model.measure ? await model.measure(ui, page) : await ui.mass(row)
      const { volume, area } = model.expected
      const tolerance = model.tolerance ?? 1e-6
      if (volume !== undefined)
        assert.ok(near(measured.volume, volume, tolerance), `${model.name} volume ${measured.volume} ≠ ${volume}`)
      if (area !== undefined)
        assert.ok(near(measured.area, area, tolerance), `${model.name} area ${measured.area} ≠ ${area}`)
      if (model.expected.bounds)
        model.expected.bounds.forEach((corner, side) =>
          corner.forEach((value, k) =>
            assert.ok(near(measured.bounds[side][k], value, 1e-9), `${model.name} bounds ${measured.bounds}`),
          ),
        )
      results[model.name] = measured
    } catch (error) {
      const file = path.join(directory, `control-${model.name.replace(/\W+/g, '-')}`)
      await page.screenshot({ path: `${file}.png` }).catch(() => {})
      error.message += `\n${await page
        .locator('.operation-card, [role=alert]')
        .allInnerTexts()
        .catch(() => '')}`
      throw error
    } finally {
      await context.close()
    }
  }
  return results
}
