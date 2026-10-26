import {createRenderer, nextTick} from 'vue'
import {afterEach, expect, it, vi} from 'vitest'
import MeshModeler from '../src/features/MeshModeler.vue'
import {createBoxMesh, transformMesh} from '../src/services/meshEditing'

const downloadBytes=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/downloadArtifact',()=>({downloadBytes}))
const repairControl=vi.hoisted(()=>({defer:false,events:[] as {phase:string;completed:number;total:number}[],resolve:(value?:unknown)=>{},reject:(reason?:unknown)=>{},cancel:vi.fn(),dispose:vi.fn()}))
const manifoldControl=vi.hoisted(()=>({defer:false,events:[] as {phase:string;completed:number;total:number}[],resolve:(value?:unknown)=>{},reject:(reason?:unknown)=>{},cancel:vi.fn(),dispose:vi.fn()}))
vi.mock('../src/services/solidPreviewWorker',async()=>{
 const {booleanMeshObjectsDetailed,repairMeshObject}=await import('../src/services/meshEditing')
 const {importMeshFile}=await import('../src/services/meshImport')
 const {exportPolygonMesh}=await import('../src/services/meshConvert')
 const {inspectMeshManifoldObjects}=await import('../src/services/meshManifoldDefects')
 return {createSolidPreviewWorker:()=>{
  let activeKind=''
  const control=()=>activeKind==='meshManifold'?manifoldControl:repairControl
  return {
   run:(job:any,options:any)=>{
    activeKind=job.kind
    const current=control()
    const emit=(progress:{phase:string;completed:number;total:number})=>{current.events.push(progress);options?.onProgress?.(progress)}
    const execute=()=>job.kind==='meshManifold'?inspectMeshManifoldObjects(job.objects,emit):job.kind==='meshImport'?importMeshFile(job.fileName,job.input,job.options,emit):job.kind==='meshExport'?exportPolygonMesh(job.mesh,job.target,job.options,emit):job.kind==='meshBoolean'?booleanMeshObjectsDetailed(job.a,job.b,job.operation,emit,job.options):repairMeshObject(job.mesh,emit)
    if(current.defer){
     emit(job.kind==='meshManifold'?{phase:'check',completed:0,total:job.objects.length}:job.kind==='meshImport'?{phase:'decode',completed:0,total:5}:job.kind==='meshExport'?{phase:'inspect',completed:0,total:2}:job.kind==='meshBoolean'?{phase:'boolean',completed:0,total:1}:{phase:'check',completed:0,total:3})
     return new Promise((resolve,reject)=>{
      current.resolve=()=>resolve(execute())
      current.reject=reject
     })
    }
    return Promise.resolve().then(execute)
   },
   cancel:()=>{control().cancel();control().reject(Object.assign(new Error('Operation cancelled'),{name:'AbortError'}))},
   dispose:()=>{control().dispose();control().reject(Object.assign(new Error('Worker disposed'),{name:'AbortError'}))},
  }
 }}
})
afterEach(()=>{downloadBytes.mockClear();for(const control of [repairControl,manifoldControl]){control.defer=false;control.events=[];control.resolve=()=>{};control.reject=()=>{};control.cancel.mockClear();control.dispose.mockClear()}})

const gate = vi.hoisted(()=>({resolve:()=>{}, promise:Promise.resolve(),ready:false}))
vi.mock('../src/services/geometry/kernel', async importOriginal=>({
  ...await importOriginal<typeof import('../src/services/geometry/kernel')>(),
  isGeometryKernelReady:()=>false,
  warmGeometryKernel:()=>gate.promise,
}))
vi.mock('../src/components/VrControls.vue',()=>({default:{render:()=>null}}))
vi.mock('../src/components/CommandPalette.vue',()=>({default:{render:()=>null}}))
vi.mock('../src/components/ModelingGridControls.vue',()=>({default:{render:()=>null}}))
vi.mock('../src/components/ModelingFloorGrid.vue',()=>({default:{render:()=>null}}))
vi.mock('../src/services/safeStorage',()=>({storageGet:()=>null,storageSet:()=>{}}))
vi.mock('../src/services/meshModelerProjection',async importOriginal=>{
  const actual=await importOriginal<typeof import('../src/services/meshModelerProjection')>()
  return {...actual,projectMeshModelerObject:(...args:Parameters<typeof actual.projectMeshModelerObject>)=>{
    if(!gate.ready)throw new Error('Projection requested before browser WASM readiness')
    return actual.projectMeshModelerObject(...args)
  },projectMeshModelerFaces:(...args:Parameters<typeof actual.projectMeshModelerFaces>)=>{
    if(!gate.ready)throw new Error('Projection requested before browser WASM readiness')
    return actual.projectMeshModelerFaces(...args)
  }}
})

class Node {
  parent:Node|null=null
  children:Node[]=[]
  props:Record<string,unknown>={}
  text=''
  value:unknown=''
  selected=false
  constructor(public tag:string){}
  focus(){}
  addEventListener(){}
  removeEventListener(){}
  get tagName(){return this.tag.toUpperCase()}
  get options(){return this.children.filter(n=>n.tag==='option')}
  getRootNode(){return {activeElement:null}}
}
const renderer=createRenderer<Node,Node>({
  createElement:tag=>new Node(tag),
  createText:text=>Object.assign(new Node('#text'),{text}),
  createComment:()=>new Node('#comment'),
  setText:(n,text)=>{n.text=text},
  setElementText:(n,text)=>{n.children=[];n.text=text},
  parentNode:n=>n.parent,
  nextSibling:n=>n.parent?.children[n.parent.children.indexOf(n)+1]??null,
  patchProp:(n,key,_old,value)=>{n.props[key]=value;if(key==='value')n.value=value},
  insert:(n,parent,anchor)=>{
    if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n)
    n.parent=parent
    const i=anchor?parent.children.indexOf(anchor):-1
    if(i<0)parent.children.push(n);else parent.children.splice(i,0,n)
  },
  remove:n=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n)},
  setScopeId:()=>{},
  insertStaticContent:()=>{throw new Error('Unexpected static content')},
})
const all=(n:Node):Node[]=>[n,...n.children.flatMap(all)]

it('publishes saved mesh faces after delayed kernel readiness without a user edit',async()=>{
  gate.ready=false
  gate.promise=new Promise<void>(resolve=>{gate.resolve=()=>{gate.ready=true;resolve()}})
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{
    open:true,locale:'en',seedDocument:{version:1,objects:[{
      id:'sheet',name:'Sheet',visible:true,
      mesh:{positions:[0,0,0,10,0,0,0,10,0],indices:[0,1,2]},
    }]},
  })
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    expect(errors).toEqual([])
    expect(all(root).filter(n=>n.tag==='polygon')).toHaveLength(0)
    expect(all(root).some(n=>n.text==='Preparing geometry…')).toBe(true)
    gate.resolve()
    await Promise.resolve()
    await nextTick()
    expect(errors).toEqual([])
    const faces=all(root).filter(n=>n.tag==='polygon')
    expect(faces).toHaveLength(1)
    expect(String(faces[0]!.props.points)).not.toMatch(/NaN|Infinity/)
    const stage=all(root).find(n=>n.props.class==='mesh-stage')!
    expect(stage.children.some(n=>String(n.props.class).includes('side-dock'))).toBe(true)
    expect(all(root).some(n=>n.text==='Preparing geometry…')).toBe(false)
  }finally{app.unmount()}
})

it('runs Full manifold repair during mesh import and displays its report',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en'})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const source=[
      'v 0 0 0','v 1 0 0','v 0 1 0','v 0 0 1',
      'f 1 3 2','f 1 2 4','f 2 3 4',
    ].join('\n')
    const bytes=new TextEncoder().encode(source)
    const input=all(root).find(n=>n.tag==='input'&&n.props.type==='file')!
    const mode=all(root).find(n=>n.tag==='select'&&n.props['aria-label']==='Import repair mode')!
    mode.props['onUpdate:modelValue']('full')
    const target={files:[{name:'open.obj',arrayBuffer:async()=>bytes.buffer}],value:'open.obj'}
    await input.props.onChange({target})
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    const report=all(root).find(n=>n.props.class==='import-report')
    expect(report).toBeDefined()
    expect(text(report!)).toContain('manifold topology confirmed')
    expect(text(report!)).toContain('holes filled: 1')
    expect(text(root)).toContain('4f')
    expect(target.value).toBe('')
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('shows manifold Boolean pre-repair diagnostics and commits the result',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const openTetra={
    positions:[0,0,0,2,0,0,0,2,0,0,0,2],
    indices:[0,2,1,0,1,3,1,2,3],
  }
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'a',name:'Open tetra',visible:true,mesh:openTetra},
    {id:'b',name:'Box',visible:true,mesh:createBoxMesh([4,4,4])},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Properties')!.props.onClick()
    await nextTick()
    const section=all(root).find(n=>n.tag==='section'&&n.children.some(c=>c.tag==='h3'&&text(c)==='Boolean'))!
    const selects=all(section).filter(n=>n.tag==='select')
    selects[1]!.props['onUpdate:modelValue']('b')
    await nextTick()
    all(section).find(n=>n.tag==='button'&&text(n)==='Run')!.props.onClick()
    await vi.waitFor(()=>expect(all(root).some(n=>n.props.class==='boolean-report')).toBe(true))
    const report=all(root).find(n=>n.props.class==='boolean-report')!
    expect(text(report)).toContain('manifold result')
    expect(text(report)).toContain('Full pre-repair: operand 1')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Scene')!.props.onClick()
    await nextTick()
    const scene=all(root).find(n=>n.props.class==='scene-list')!
    expect(all(scene).filter(n=>n.tag==='li')).toHaveLength(1)
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('keeps both mesh objects when the Boolean result is empty',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'a',name:'A',visible:true,mesh:createBoxMesh([2,2,2])},
    {id:'b',name:'B',visible:true,mesh:transformMesh(createBoxMesh([2,2,2]),[20,0,0],0,1)},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Properties')!.props.onClick()
    await nextTick()
    const section=all(root).find(n=>n.tag==='section'&&n.children.some(c=>c.tag==='h3'&&text(c)==='Boolean'))!
    const selects=all(section).filter(n=>n.tag==='select')
    selects[0]!.props['onUpdate:modelValue']('intersection')
    selects[1]!.props['onUpdate:modelValue']('b')
    await nextTick()
    all(section).find(n=>n.tag==='button'&&text(n)==='Run')!.props.onClick()
    await vi.waitFor(()=>expect(all(root).some(n=>n.props.role==='alert')).toBe(true))
    const alert=all(root).find(n=>n.props.role==='alert')!
    expect(text(alert)).toContain('Boolean failed')
    expect(text(alert)).toContain('empty mesh')
    expect(text(alert)).toContain('Document was not changed')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Scene')!.props.onClick()
    await nextTick()
    const scene=all(root).find(n=>n.props.class==='scene-list')!
    expect(all(scene).filter(n=>n.tag==='li')).toHaveLength(2)
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('expands kernel manifold defect categories only for defective objects',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'open',name:'Open',visible:true,mesh:{
      positions:[0,0,0,2,0,0,0,2,0,0,0,2],indices:[0,2,1,0,1,3,1,2,3],
    }},
    {id:'closed',name:'Closed',visible:true,mesh:createBoxMesh([2,2,2])},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Scene')!.props.onClick()
    await nextTick()
    const warnings=all(root).filter(n=>n.props.class==='manifold-warning')
    expect(warnings).toHaveLength(1)
    expect(String(warnings[0]!.props['aria-label'])).toContain('boundary edges: 3')
    expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(3)
    expect(all(root).filter(n=>n.props['data-defect']==='non-manifold')).toHaveLength(0)
    expect(all(root).some(n=>n.props.class==='manifold-legend')).toBe(true)
    expect(all(root).some(n=>n.props.class==='manifold-defects')).toBe(false)
    warnings[0]!.props.onClick({stopPropagation(){}})
    await nextTick()
    const details=all(root).find(n=>n.props.class==='manifold-defects')!
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    expect(text(details)).toContain('boundary edges: 3')
    expect(warnings[0]!.props['aria-expanded']).toBe(true)
    warnings[0]!.props.onClick({stopPropagation(){}})
    await nextTick()
    expect(all(root).some(n=>n.props.class==='manifold-defects')).toBe(false)
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('renders pinched-vertex markers above the mesh',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const pinched={
    positions:[0,0,0,2,0,0,0,2,0,0,0,2,-2,0,0,0,-2,0,0,0,-2],
    indices:[0,2,1,0,1,3,1,2,3,0,3,2,0,5,4,0,4,6,4,5,6,0,6,5],
  }
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'pinched',name:'Pinched',visible:true,mesh:pinched},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const markers=all(root).filter(n=>n.props['data-defect']==='pinched-vertex')
    expect(markers).toHaveLength(1)
    expect(markers[0]!.tag).toBe('circle')
    expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(0)
    const legend=all(root).find(n=>n.props.class==='manifold-legend')!
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    expect(text(legend)).toContain('Pinched vertices')
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('repairs the selected mesh with a named history entry and Undo/Redo',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[{
    id:'open',name:'Open',visible:true,mesh:{
      positions:[0,0,0,2,0,0,0,2,0,0,0,2],indices:[0,2,1,0,1,3,1,2,3],
    },
  }]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Properties')!.props.onClick()
    await nextTick()
    const section=all(root).find(n=>n.tag==='section'&&n.children.some(c=>c.tag==='h3'&&text(c)==='Topology'))!
    all(section).find(n=>n.tag==='button'&&text(n)==='Repair mesh · Full')!.props.onClick()
    await vi.waitFor(()=>expect(all(root).some(n=>n.props.class==='repair-report')).toBe(true))
    const report=all(root).find(n=>n.props.class==='repair-report')!
    expect(repairControl.events.map(event=>event.phase)).toEqual(['check','repair','verify'])
    expect(text(report)).toContain('manifold topology confirmed')
    expect(text(report)).toContain('holes filled: 1')
    expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(0)
    const undo=all(root).find(n=>n.props.class==='icon mesh-undo')!
    expect(String(undo.props.title)).toContain('Repair mesh (Full)')
    undo.props.onClick()
    await nextTick()
    await vi.waitFor(()=>expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(3))
    const redo=all(root).find(n=>n.props.class==='icon mesh-redo')!
    expect(String(redo.props.title)).toContain('Repair mesh (Full)')
    redo.props.onClick()
    await nextTick()
    await vi.waitFor(()=>expect(all(root).some(n=>n.props['aria-label']==='manifold-inspection')).toBe(false))
    expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(0)
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('cancels background import and does not add its late mesh to the document',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  repairControl.defer=true
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'existing',name:'Existing',visible:true,mesh:createBoxMesh([2,2,2])},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    const source=['v 0 0 0','v 1 0 0','v 0 1 0','v 0 0 1','f 1 3 2','f 1 2 4','f 2 3 4'].join('\n')
    const bytes=new TextEncoder().encode(source)
    const input=all(root).find(n=>n.tag==='input'&&n.props.type==='file')!
    const target={files:[{name:'open.obj',arrayBuffer:async()=>bytes.buffer}],value:'open.obj'}
    const pending=input.props.onChange({target})
    await Promise.resolve()
    await nextTick()
    const status=all(root).find(n=>n.props.class==='import-progress')!
    expect(text(status)).toContain('Decoding file · 0/5')
    expect(all(root).find(n=>n.tag==='progress')!.props.max).toBe(5)
    all(status).find(n=>n.tag==='button'&&n.props['aria-label']==='Cancel import')!.props.onClick()
    await nextTick()
    expect(repairControl.cancel).toHaveBeenCalledOnce()
    repairControl.resolve()
    await pending
    await nextTick()
    expect(all(root).some(n=>n.props.class==='import-report')).toBe(false)
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Scene')!.props.onClick()
    await nextTick()
    const scene=all(root).find(n=>n.props.class==='scene-list')!
    expect(all(scene).filter(n=>n.tag==='li')).toHaveLength(1)
    expect(target.value).toBe('')
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('cancels a background Boolean and keeps both objects after its late completion',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  repairControl.defer=true
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[
    {id:'a',name:'A',visible:true,mesh:createBoxMesh([2,2,2])},
    {id:'b',name:'B',visible:true,mesh:transformMesh(createBoxMesh([2,2,2]),[1,0,0],0,1)},
  ]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Properties')!.props.onClick()
    await nextTick()
    const section=all(root).find(n=>n.tag==='section'&&n.children.some(c=>c.tag==='h3'&&text(c)==='Boolean'))!
    all(section).filter(n=>n.tag==='select')[1]!.props['onUpdate:modelValue']('b')
    await nextTick()
    all(section).find(n=>n.tag==='button'&&text(n)==='Run')!.props.onClick()
    await nextTick()
    const status=all(root).find(n=>n.props.class==='boolean-progress')!
    expect(text(status)).toContain('Manifold Boolean operation · 0/1')
    expect(all(root).find(n=>n.tag==='progress')!.props.max).toBe(1)
    all(status).find(n=>n.tag==='button'&&n.props['aria-label']==='Cancel Boolean')!.props.onClick()
    await nextTick()
    expect(repairControl.cancel).toHaveBeenCalledOnce()
    repairControl.resolve()
    await Promise.resolve()
    await Promise.resolve()
    await nextTick()
    expect(all(root).some(n=>n.props.class==='boolean-report')).toBe(false)
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Scene')!.props.onClick()
    await nextTick()
    const scene=all(root).find(n=>n.props.class==='scene-list')!
    expect(all(scene).filter(n=>n.tag==='li')).toHaveLength(2)
    expect(errors).toEqual([])
  }finally{app.unmount()}
})

it('cancels background repair, ignores its late result and disposes the worker',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  repairControl.defer=true
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[{
    id:'open',name:'Open',visible:true,mesh:{
      positions:[0,0,0,2,0,0,0,2,0,0,0,2],indices:[0,2,1,0,1,3,1,2,3],
    },
  }]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    all(root).find(n=>n.tag==='button'&&n.props['aria-label']==='Properties')!.props.onClick()
    await nextTick()
    const section=all(root).find(n=>n.tag==='section'&&n.children.some(c=>c.tag==='h3'&&text(c)==='Topology'))!
    all(section).find(n=>n.tag==='button'&&text(n)==='Repair mesh · Full')!.props.onClick()
    await nextTick()
    const status=all(root).find(n=>n.props.class==='repair-progress')!
    expect(text(status)).toContain('Checking the source mesh · 0/3')
    expect(all(root).find(n=>n.tag==='progress')!.props.max).toBe(3)
    all(status).find(n=>n.tag==='button'&&n.props['aria-label']==='Cancel repair')!.props.onClick()
    await nextTick()
    expect(repairControl.cancel).toHaveBeenCalledOnce()
    expect(all(root).some(n=>n.props.class==='repair-progress')).toBe(false)
    repairControl.resolve()
    await Promise.resolve()
    await Promise.resolve()
    await nextTick()
    expect(all(root).some(n=>n.props.class==='repair-report')).toBe(false)
    expect(all(root).filter(n=>n.props['data-defect']==='boundary')).toHaveLength(3)
    expect(errors).toEqual([])
  }finally{app.unmount()}
  expect(repairControl.dispose).toHaveBeenCalledOnce()
})

it('cancels background mesh export and ignores its late download',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  repairControl.defer=true
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[{
    id:'box',name:'Box',visible:true,mesh:createBoxMesh([2,2,2]),
  }]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    await Promise.resolve()
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    const download=all(root).find(n=>n.tag==='button'&&text(n)==='Download')!
    download.props.onClick()
    await nextTick()
    const status=all(root).find(n=>n.props.class==='export-progress')!
    expect(text(status)).toContain('Inspecting mesh · 0/2')
    expect(all(status).find(n=>n.tag==='progress')!.props.max).toBe(2)
    download.props.onClick()
    expect(repairControl.events).toHaveLength(1)
    all(status).find(n=>n.tag==='button'&&n.props['aria-label']==='Cancel export')!.props.onClick()
    await nextTick()
    expect(repairControl.cancel).toHaveBeenCalledOnce()
    repairControl.resolve()
    await Promise.resolve()
    await Promise.resolve()
    await nextTick()
    expect(downloadBytes).not.toHaveBeenCalled()
    expect(all(root).some(n=>n.props.class==='export-progress')).toBe(false)
    expect(errors).toEqual([])
  }finally{app.unmount()}
  expect(repairControl.dispose).toHaveBeenCalledOnce()
})

it('cancels scene manifold inspection and disposes its independent worker',async()=>{
  gate.ready=true
  gate.promise=Promise.resolve()
  manifoldControl.defer=true
  const root=new Node('root')
  const errors:unknown[]=[]
  const app=renderer.createApp(MeshModeler,{open:true,locale:'en',seedDocument:{version:1,objects:[{
    id:'open',name:'Open',visible:true,mesh:{
      positions:[0,0,0,2,0,0,0,2,0,0,0,2],indices:[0,2,1,0,1,3,1,2,3],
    },
  }]}})
  app.config.errorHandler=error=>errors.push(error)
  try{
    app.mount(root)
    await nextTick()
    const text=(n:Node):string=>n.text+n.children.map(text).join('')
    const status=all(root).find(n=>n.props['aria-label']==='manifold-inspection')!
    expect(text(status)).toContain('Checking manifold topology · 0/1')
    expect(all(status).find(n=>n.tag==='progress')!.props.max).toBe(1)
    all(status).find(n=>n.tag==='button'&&n.props['aria-label']==='Cancel manifold inspection')!.props.onClick()
    await nextTick()
    expect(manifoldControl.cancel).toHaveBeenCalledOnce()
    manifoldControl.resolve()
    await Promise.resolve()
    await nextTick()
    expect(all(root).some(n=>n.props['aria-label']==='manifold-inspection')).toBe(false)
    expect(all(root).some(n=>n.props.class==='manifold-warning')).toBe(false)
    expect(errors).toEqual([])
  }finally{app.unmount()}
  expect(manifoldControl.dispose).toHaveBeenCalledOnce()
})
