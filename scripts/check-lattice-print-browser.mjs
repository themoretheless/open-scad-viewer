import assert from 'node:assert/strict'
import {createServer} from 'vite'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const server = await createServer({logLevel: 'silent', server: {host: '127.0.0.1', port: 0}, plugins: [{
  name: 'lattice-print-probe',
  resolveId(id) { if (id === '/__lattice-host.js') return '\0lattice-print-probe' },
  load(id) { if (id === '\0lattice-print-probe') return `
    import {createApp} from 'vue';
    import Panel from '/src/features/CadWorkbenchPanel.vue';
    import {warmGeometryKernel} from '/src/services/geometry/kernel.ts';
    await warmGeometryKernel();
    createApp(Panel,{meshes:[],selection:[],hit:null,source:'',ready:true,locale:'en',initialAction:'lighten'}).mount('#app');
  ` },
}]})
let browser
try {
  await server.listen()
  const {playwright} = await loadQualificationPlaywrightPackage()
  browser = await playwright.chromium.launch({headless: true,
    ...(process.env.CHROMIUM_EXECUTABLE ? {executablePath: process.env.CHROMIUM_EXECUTABLE} : {})})
  const reports = []
  for (const [name, viewport] of [['desktop',{width:1280,height:900}], ['mobile',{width:390,height:844}]]) {
    const page = await browser.newPage({viewport}), errors = []
    page.on('pageerror', error => errors.push(error.message))
    const origin = server.resolvedUrls.local[0]
    await page.route(`${origin}__probe`, route => route.fulfill({contentType:'text/html',body:`
      <!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">
      <title>Lattice print fit check</title><style>
      :root{--surface:#fff;--surface-raised:#f4f5f6;--text:#222;--text-dim:#666;--border:#aaa;--danger:#a21;}
      body{font-family:Arial,sans-serif;background:#e5e7e9;margin:0}</style>
      <div id="app"></div><script type="module" src="/__lattice-host.js"></script>`}))
    await page.goto(`${origin}__probe`)
    await page.getByText('FDM print settings',{exact:true}).click()
    await page.getByLabel('Limit nominal opening',{exact:true}).check()
    const bridge = page.getByLabel('Bridge limit, mm (printer test)',{exact:true})
    const cell = page.getByLabel('Cell, mm',{exact:true})
    const fit = page.getByRole('button',{name:'Fit geometry to print settings',exact:true})
    await bridge.fill('5'); await fit.click()
    assert.ok(Math.abs(Number(await cell.inputValue()) - 6.35) < 1e-12)
    await bridge.fill('1'); await fit.click()
    await page.getByRole('alert').filter({hasText:'incompatible'}).waitFor()
    assert.ok(Math.abs(Number(await cell.inputValue()) - 6.35) < 1e-12)
    await bridge.fill(''); await bridge.fill('4'); await fit.click()
    assert.ok(Math.abs(Number(await cell.inputValue()) - 5.35) < 1e-12)
    assert.equal(await page.getByRole('alert').count(),0)
    const bounds = await page.locator('.cad-workbench').boundingBox()
    assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= viewport.width)
    await page.getByLabel('Limit nominal opening',{exact:true}).scrollIntoViewIfNeeded()
    const screenshot = `/private/tmp/osv-lattice-opening-${name}.png`
    await page.screenshot({path:screenshot})
    assert.deepEqual(errors,[])
    reports.push({name,viewport,cellMm:Number(await cell.inputValue()),bounds,screenshot,errors})
    await page.close()
  }
  console.log(JSON.stringify({browser:browser.version(),scope:'Actual Vue component and embedded geometry WASM; fit, refusal, blank-input recovery and desktop/mobile layout smoke, not complete lightening or printing validation',reports},null,2))
} finally { await browser?.close(); await server.close() }
