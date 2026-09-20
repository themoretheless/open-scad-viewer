import assert from 'node:assert/strict'
import {readFile} from 'node:fs/promises'
import {createServer} from 'vite'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const server=await createServer({logLevel:'silent',server:{host:'127.0.0.1',port:0},plugins:[{
  name:'nominal-truss-probe',
  resolveId(id){if(id==='/__truss-host.js')return '\0nominal-truss-probe'},
  load(id){if(id==='\0nominal-truss-probe')return `
    import {createApp,reactive,h} from 'vue';
    import Panel from '/src/features/CadWorkbenchPanel.vue';
    import Tools from '/src/features/MainModelingTools.vue';
    import {warmGeometryKernel} from '/src/services/geometry/kernel.ts';
    import {extrudeDirectSketch} from '/src/services/directModeling.ts';
    import {previewMeshes} from '/src/services/mainModeling.ts';
    import {WebGPURenderer} from '/src/services/webgpuRenderer.ts';
    const RealWorker=window.Worker;
    window.__trussProbe={hold:false,held:[],created:0,terminated:0};
    window.Worker=class {
      constructor(url,options){
        this.worker=new RealWorker(url,options);window.__trussProbe.created++;
        this.worker.onmessage=event=>{
          const callback=this.onmessage;
          if(window.__trussProbe.hold&&event.data.kind==='truss')window.__trussProbe.held.push(()=>callback?.(event));
          else callback?.(event);
        };
        this.worker.onerror=event=>this.onerror?.(event);
        this.worker.onmessageerror=event=>this.onmessageerror?.(event);
      }
      postMessage(value){this.worker.postMessage(value)}
      terminate(){window.__trussProbe.terminated++;this.worker.terminate()}
    };
    await warmGeometryKernel();
    const body=extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'0');
    const props=reactive({meshes:previewMeshes({version:1,sketches:[],bodies:[body]}),selection:[0],hit:null,source:'cube(10);',ready:true,locale:'en',initialAction:'lighten'});
    let renderer;
    if(${process.env.TRUSS_VIEWPORT==='1'}){
      const canvas=document.createElement('canvas');canvas.id='field-viewport';canvas.style.cssText='width:min(640px,100vw);height:400px;position:fixed;right:0;top:0';document.body.append(canvas);
      renderer=new WebGPURenderer();if(!await renderer.init(canvas))throw Error('WebGPU renderer unavailable');
      renderer.setGridVisible(false);renderer.setBackgroundColor([0.9,0.9,0.9]);renderer.setMeshes(props.meshes);renderer.fitView();
    }
    const onPreview=meshes=>{window.__trussProbe.preview=meshes===null?null:meshes.map(mesh=>({color:mesh.color,triangles:mesh.indices.length/3,finite:mesh.vertices.every(Number.isFinite)}));renderer?.setMeshes(meshes??props.meshes)};
    let tools;
    const app=createApp({render:()=>${process.env.TRUSS_TOOLS==='1'}?h(Tools,{...props,ref:value=>{tools=value},selected:0,selectedIndices:props.selection,canUndo:false,canRedo:false,project:()=>null,ray:()=>null,cameraRevision:0,onPreview}):h(Panel,{...props,onPreview})});app.mount('#app');
    if(tools){tools.execute('lighten');window.__trussProbe.command=id=>tools.execute(id)}
    window.__trussProbe.props=props;window.__trussProbe.unmount=()=>{app.unmount();renderer?.destroy()};
  `},
}]})
let browser
try{
  await server.listen()
  const {playwright}=await loadQualificationPlaywrightPackage()
  browser=await playwright.chromium.launch({headless:true,args:process.env.TRUSS_VIEWPORT==='1'?['--enable-unsafe-webgpu']:[],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
  const reports=[]
  for(const [name,viewport] of [['desktop',{width:1280,height:900}],['mobile',{width:390,height:844}]]){
    const page=await browser.newPage({viewport}),errors=[]
    const viewportEvidence=[]
    const captureViewport=async stage=>{
      if(process.env.TRUSS_VIEWPORT!=='1')return
      await page.locator('.cad-workbench').evaluate(element=>element.style.visibility='hidden')
      const screenshot=`/private/tmp/osv-truss-viewport-${name}-${stage}.png`
      const png=await page.locator('#field-viewport').screenshot({path:screenshot})
      const pixels=await page.evaluate(async bytes=>{
        const source=await createImageBitmap(new Blob([new Uint8Array(bytes)],{type:'image/png'})),copy=document.createElement('canvas');copy.width=source.width;copy.height=source.height;
        const context=copy.getContext('2d');context.drawImage(source,0,0);
        source.close();
        const data=context.getImageData(0,0,copy.width,copy.height).data;
        let blue=0,red=0,opaque=0;
        for(let i=0;i<data.length;i+=4){if(data[i+3]>0)opaque++;if(data[i+2]>data[i]+35&&data[i+2]>data[i+1]+15)blue++;if(data[i]>data[i+2]+35&&data[i]>data[i+1]+15)red++}
        return {blue,red,opaque,width:copy.width,height:copy.height}
      },Array.from(png))
      await page.locator('.cad-workbench').evaluate(element=>element.style.visibility='')
      viewportEvidence.push({stage,pixels,screenshot})
      return pixels
    }
    page.on('pageerror',error=>{errors.push(error.message);console.error(error.stack)})
    const origin=server.resolvedUrls.local[0]
    await page.route(`${origin}__probe`,route=>route.fulfill({contentType:'text/html',body:`
      <!doctype html><meta name="viewport" content="width=device-width,initial-scale=1">
      <title>Nominal truss workflow check</title><style>
      :root{--surface:#fff;--surface-raised:#f4f5f6;--text:#222;--text-dim:#666;--border:#aaa;--danger:#a21;--accent:#176f5d;}
      body{font-family:Arial,sans-serif;background:#e5e7e9;margin:0}</style>
      <div id="app"></div><script type="module" src="/__truss-host.js"></script>`}))
    await page.goto(`${origin}__probe`)
    await page.locator('select:has(option[value="octet"])').selectOption('octet')
    const panel=page.locator('.nominal-truss')
    await panel.locator('summary').first().click()
    await panel.getByRole('button',{name:'Generate graph',exact:true}).click()
    await panel.getByLabel('Young modulus, MPa',{exact:true}).waitFor()
    assert.match(await panel.textContent(),/14 nodes.*36 members/s)
    assert.equal(await panel.locator('input[aria-label$="restrained"]:checked').count(),0)
    await panel.getByLabel('Young modulus, MPa',{exact:true}).fill('2000')
    await panel.getByLabel('Member area, mm²',{exact:true}).fill('2')
    await panel.getByLabel('Bounding plane',{exact:true}).selectOption('z-max')
    await panel.getByRole('button',{name:'Set load nodes',exact:true}).click()
    await panel.getByRole('button',{name:'Use node centroid',exact:true}).click()
    await panel.getByLabel('Force, N Z',{exact:true}).fill('-100')
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    await panel.getByRole('alert').filter({hasText:/TRUSS_SINGULAR|singular|mechanism|unrestrained/i}).waitFor()
    assert.equal(await panel.getByRole('region',{name:'Axial graph results'}).count(),0)
    await panel.getByLabel('Bounding plane',{exact:true}).selectOption('z-min')
    await panel.getByRole('button',{name:'Set restraints',exact:true}).click()
    await panel.getByLabel('Bounding plane',{exact:true}).scrollIntoViewIfNeeded()
    const controlsScreenshot=`/private/tmp/osv-nominal-truss-controls-${name}.png`
    await page.screenshot({path:controlsScreenshot})
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    const results=panel.getByRole('region',{name:'Axial graph results'})
    await results.waitFor()
    const downloadReport=async suffix=>{
      const pending=page.waitForEvent('download')
      await results.getByRole('link',{name:'nominal-truss.json',exact:true}).click()
      const download=await pending,path=`/private/tmp/osv-nominal-truss-${name}-${suffix}.json`
      await download.saveAs(path)
      return JSON.parse(await readFile(path,'utf8'))
    }
    const first=await downloadReport('single')
    assert.equal(first.modelKind,'nominal-bounding-box-axial')
    assert.equal(first.model.nodesMm.length,14)
    const freeNode=first.model.restrained.findIndex(mask=>!mask[0])
    assert.ok(freeNode>=0)
    assert.ok(Math.abs(first.result.reactionsN.reduce((sum,force)=>sum+force[2],0)-100)<1e-8)
    assert.equal(await page.evaluate(()=>window.__trussProbe.created),1)
    await results.getByLabel('Preview axial force sign',{exact:true}).check()
    const markers=await page.evaluate(()=>window.__trussProbe.preview)
    assert.ok(markers.length>0&&markers.length<=3)
    assert.equal(markers.reduce((sum,mesh)=>sum+mesh.triangles,0),36*8)
    assert.ok(markers.every(mesh=>mesh.finite))
    if(process.env.TRUSS_TOOLS==='1'){
      await page.evaluate(()=>window.__trussProbe.command('box-select'))
      assert.equal(await page.evaluate(()=>window.__trussProbe.preview),null)
      assert.equal(await results.getByLabel('Preview axial force sign',{exact:true}).isChecked(),false)
      await page.evaluate(()=>window.__trussProbe.command('box-select'))
      await results.getByLabel('Preview axial force sign',{exact:true}).check()
    }
    const fieldPixels=await captureViewport('field')
    if(fieldPixels)assert.ok(fieldPixels.blue+fieldPixels.red>20,JSON.stringify(fieldPixels))
    if(fieldPixels){
      await page.locator('.cad-workbench').evaluate(element=>element.style.visibility='hidden')
      const box=await page.locator('#field-viewport').boundingBox()
      await page.mouse.move(box.x+box.width/2,box.y+box.height/2)
      await page.mouse.down();await page.mouse.move(box.x+box.width/2+45,box.y+box.height/2+25,{steps:8});await page.mouse.up()
      await page.locator('.cad-workbench').evaluate(element=>element.style.visibility='')
      const orbited=await captureViewport('orbited')
      assert.ok(orbited.blue+orbited.red>20)
      assert.notDeepEqual(orbited,fieldPixels)
    }
    await results.getByLabel('Marker size, mm',{exact:true}).fill('0')
    assert.equal(await page.evaluate(()=>window.__trussProbe.preview),null)
    const restoredPixels=await captureViewport('restored')
    if(restoredPixels)assert.notDeepEqual(restoredPixels,fieldPixels)
    assert.equal(await results.getByLabel('Preview axial force sign',{exact:true}).isChecked(),false)
    await results.getByLabel('Preview axial force sign',{exact:true}).click()
    assert.equal(await results.getByLabel('Preview axial force sign',{exact:true}).isChecked(),false)
    await results.getByLabel('Marker size, mm',{exact:true}).fill('1')
    await results.getByLabel('Preview axial force sign',{exact:true}).check()
    await panel.getByRole('button',{name:'Duplicate case',exact:true}).click()
    assert.equal(await page.evaluate(()=>window.__trussProbe.preview),null)
    await panel.getByLabel('Force, N Z',{exact:true}).fill('-50')
    await panel.getByLabel('Result mode',{exact:true}).selectOption('combination')
    await panel.getByLabel('Factor case-1',{exact:true}).fill('1.2')
    await panel.getByLabel('Factor case-2',{exact:true}).fill('-0.5')
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    await results.waitFor()
    const combined=await downloadReport('combined')
    // A new solve does not resurrect an invalid or previously cleared field.
    assert.equal(await results.getByLabel('Preview axial force sign',{exact:true}).isChecked(),false)
    assert.deepEqual(combined.terms,[{caseId:'case-1',factor:1.2},{caseId:'case-2',factor:-.5}])
    assert.ok(Math.abs(combined.result.reactionsN.reduce((sum,force)=>sum+force[2],0)-95)<1e-8)
    await panel.getByLabel(`Node ${freeNode} X restrained`,{exact:true}).check()
    assert.equal(await results.count(),0)
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    await panel.getByRole('alert').filter({hasText:'identical XYZ restraints'}).waitFor()
    await panel.getByLabel(`Node ${freeNode} X restrained`,{exact:true}).uncheck()
    await page.evaluate(()=>{window.__trussProbe.hold=true})
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    await page.waitForFunction(()=>window.__trussProbe.held.length===1)
    await panel.getByLabel('Young modulus, MPa',{exact:true}).fill('4000')
    await page.evaluate(()=>{window.__trussProbe.hold=false;window.__trussProbe.held.splice(0).forEach(deliver=>deliver())})
    assert.equal(await results.count(),0)
    assert.equal(await page.evaluate(()=>window.__trussProbe.terminated),1)
    await panel.getByRole('button',{name:'Solve model',exact:true}).click()
    await results.waitFor()
    const edited=await downloadReport('edited')
    await results.getByLabel('Preview axial force sign',{exact:true}).check()
    assert.ok(Math.abs(edited.result.maxDeflectionMm*2-combined.result.maxDeflectionMm)<1e-10)
    assert.equal(await page.evaluate(()=>window.__trussProbe.created),2)
    await results.scrollIntoViewIfNeeded()
    const screenshot=`/private/tmp/osv-nominal-truss-${name}.png`
    await page.screenshot({path:screenshot})
    const bounds=await page.locator('.cad-workbench').boundingBox()
    assert.ok(bounds.x>=0&&bounds.x+bounds.width<=viewport.width)
    const overflowing=await panel.locator('input,select,button').evaluateAll(elements=>elements.filter(element=>{
      if(element.closest('.table-scroll'))return false
      const box=element.getBoundingClientRect(),parent=element.closest('.nominal-truss').getBoundingClientRect()
      return box.width>0&&(box.left<parent.left-1||box.right>parent.right+1)
    }).map(element=>element.outerHTML))
    assert.deepEqual(overflowing,[])
    await page.evaluate(()=>{
      const props=window.__trussProbe.props,mesh=props.meshes[0],transform=mesh.transform.slice()
      transform[3]+=1;props.meshes[0]={...mesh,transform}
    })
    await panel.getByLabel('Young modulus, MPa',{exact:true}).waitFor({state:'detached'})
    assert.equal(await page.evaluate(()=>window.__trussProbe.preview),null)
    assert.equal(await results.count(),0)
    await panel.getByRole('button',{name:'Generate graph',exact:true}).click()
    await panel.getByLabel('Young modulus, MPa',{exact:true}).waitFor()
    await page.evaluate(()=>{window.__trussProbe.props.source='cube(11);'})
    await panel.getByLabel('Young modulus, MPa',{exact:true}).waitFor({state:'detached'})
    assert.equal(await results.count(),0)
    await page.evaluate(()=>window.__trussProbe.unmount())
    assert.deepEqual(errors,[])
    reports.push({name,viewport,bounds,screenshot,controlsScreenshot,nominalNodes:14,nominalMembers:36,
      singleReactionN:100,combinedReactionN:95,singularRefusal:true,incompatibleSupportsRefusal:true,
      staleReplyIgnored:true,meshReplacementInvalidated:true,sourceInvalidated:true,viewportEvidence,errors})
    await page.close()
  }
  console.log(JSON.stringify({browser:browser.version(),scope:'Actual CAD panel, scenario API and real native worker on desktop/mobile; intercepted late replies test invalidation, not physical part strength or latency',reports},null,2))
}finally{await browser?.close();await server.close()}
