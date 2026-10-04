import {setImmediate as realSetImmediate} from 'node:timers'
import {createSolidNurbsCurve} from '../src/services/solidNurbs'
import {createRenderer,h,nextTick,shallowReactive,shallowRef} from 'vue'
import {it,expect,vi,afterEach,beforeAll} from 'vitest'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {IDBFactory} from 'fake-indexeddb'
import * as stepStore from '../src/services/cadStepIndexedDb'
import * as geometryKernel from '../src/services/geometry/kernel'
import { useModelingGrid } from '../src/services/modelingGrid'
import DirectModeler from '../src/features/DirectModeler.vue'
beforeAll(async()=>{await import('../src/components/CurvePointTrimControls.vue');await import('../src/components/ProfileIntersectionPresentation.vue')})
import {extrudeDirectSketch,parseDirectDocument,type DirectDocument} from '../src/services/directModeling'
import {projectDirectPoint,unprojectDirectXY,defaultDirectCamera} from '../src/services/directModelingTools'
import {solidTopology} from '../src/services/directSolidTools'
import {sampleCurve} from '../src/services/directSketchGeometry'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
import {stringifyMeshJson} from '../src/services/meshJson'
import {createBrepBox,createBrepTube,analyzeNurbsBrep,createBrepCylinder,createBrepSphere,transformNurbsBrep,tessellateNurbsBrep} from '../src/services/geometry/brep'
vi.mock('../src/services/solidDraftHeadStore',()=>({readSolidDraftHead:async()=>null,writeSolidDraftHead:async(key:string,_expected:string|null,text:string)=>({key,revision:crypto.randomUUID(),text})}))
const previewWorkerRun=vi.hoisted(()=>vi.fn())
const displayWorkerRun=vi.hoisted(()=>vi.fn())
const primitiveWorkerRun=vi.hoisted(()=>vi.fn())
const measurementWorkerRun=vi.hoisted(()=>vi.fn())
const boundaryWorkerRun=vi.hoisted(()=>vi.fn())
const surfaceDisplayWorkerRun=vi.hoisted(()=>vi.fn())
const topologyWorkerRun=vi.hoisted(()=>vi.fn())
const bodyEdgesWorkerRun=vi.hoisted(()=>vi.fn())
const faceSketchWorkerRun=vi.hoisted(()=>vi.fn())
const bodySnapsWorkerRun=vi.hoisted(()=>vi.fn())
const sketchSnapsWorkerRun=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/solidPreviewWorker',async()=>{
 const {bodySnapGeometry}=await import('../src/services/solidSnapGeometry')
 const {sketchSnapGeometry}=await import('../src/services/modelingSnaps')
 const {prepareSolidFaceSketch}=await import('../src/services/solidFaceSketch')
 const {solidBodyEdges}=await import('../src/services/solidBodyEdges')
 const {solidTopology}=await import('../src/services/directSolidTools')
 const {retainedProfileDisplay}=await import('../src/services/retainedSketchProfile')
 const {tessellateSolidNurbsSurface,sampleSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {measureSurfaceBoundaries}=await import('../src/services/solidSurfaceDiagnostics')
 const {measureFaceDistance,measureShellDistance}=await import('../src/services/solidMeasurements')
 const {measureNurbsSurfaceDistance}=await import('../src/services/nurbsSurface')
 const {measureNurbsCurveDistance}=await import('../src/services/nurbsCurve')
 const {measureSolidVertices,measureSolidEdgeCurvature}=await import('../src/services/solidMeasurements')
 const {addSolidPrimitive}=await import('../src/services/solidPrimitive')
 const {importSolidModelGraph}=await import('../src/services/solidModelGraphImport')
 const {prepareSolidDisplay}=await import('../src/services/solidDisplayPreparation')
 const {parseDirectDocument}=await import('../src/services/directModeling')
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const {applySolidProfileEdit}=await import('../src/services/solidProfileEdit')
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const {refitSolidNurbs}=await import('../src/services/solidCurveReduction')
 const {buildSolidSurface}=await import('../src/services/solidSurfaceConstruction')
 const {matchSolidCurve}=await import('../src/services/solidCurveMatching')
 const {matchSolidSurface,prepareSolidSurfaceSeams}=await import('../src/services/solidSurfaceMatching')
 const {applySolidPointEdit}=await import('../src/services/solidPointEdit')
 const {applySolidSketchEdit}=await import('../src/services/solidSketchEdit')
 const {applySolidBoolean}=await import('../src/services/solidBoolean')
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const {applySolidBrepTool}=await import('../src/services/solidBrepTool')
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 return {createSolidPreviewWorker:()=>({run:(job:any)=>(job.kind==='sketchSnaps'?sketchSnapsWorkerRun(job):job.kind==='bodySnaps'?bodySnapsWorkerRun(job):job.kind==='faceSketch'?faceSketchWorkerRun(job):job.kind==='bodyEdges'?bodyEdgesWorkerRun(job):job.kind==='topology'?topologyWorkerRun(job):job.kind==='curveDisplay'?undefined:job.kind==='profileDisplay'?undefined:job.kind==='surfaceMesh'?surfaceDisplayWorkerRun(job):job.kind==='surfaceBoundary'?boundaryWorkerRun(job):['shellDistance','faceDistance','surfaceDistance','curveDistance','measureVertices','measureEdge'].includes(job.kind)?measurementWorkerRun(job):job.kind==='primitive'?primitiveWorkerRun(job):job.kind==='displayMesh'?displayWorkerRun(job):previewWorkerRun(job))??Promise.resolve().then(()=>(job.kind==='sketchSnaps'?sketchSnapGeometry(structuredClone([job.sketch])):job.kind==='bodySnaps'?bodySnapGeometry(job.body):job.kind==='faceSketch'?prepareSolidFaceSketch(job.body,job.face):job.kind==='bodyEdges'?solidBodyEdges(job.body):job.kind==='topology'?solidTopology(job.mesh):job.kind==='curveDisplay'?sampleSolidNurbsCurve(job.curve):job.kind==='profileDisplay'?retainedProfileDisplay(job.profile):job.kind==='surfaceMesh'?tessellateSolidNurbsSurface(job.item):job.kind==='surfaceBoundary'?measureSurfaceBoundaries(job.a,job.b,job.options):job.kind==='shellDistance'?measureShellDistance(job.options):job.kind==='faceDistance'?measureFaceDistance(job.options):job.kind==='surfaceDistance'?measureNurbsSurfaceDistance(job.a,job.b,job.toleranceMm,job.maxCells):job.kind==='curveDistance'?measureNurbsCurveDistance(job.a,job.b,job.toleranceMm,job.maxCells):job.kind==='measureVertices'?measureSolidVertices(job.a,job.indexA,job.b,job.indexB):job.kind==='measureEdge'?measureSolidEdgeCurvature(job.body,job.edge,job.parameter):job.kind==='primitive'?addSolidPrimitive(job.document,job.options):job.kind==='modelGraphImport'?importSolidModelGraph(job.document,job.text,job.group):job.kind==='displayMesh'?prepareSolidDisplay(job.mesh,job.brep,job.segments):job.kind==='restoreDocument'?parseDirectDocument(job.text):job.kind==='brepTool'?applySolidBrepTool(job.document,job.options):job.kind==='nurbsEdit'?applySolidNurbsEdit(job.document,job.options):job.kind==='pointEdit'?applySolidPointEdit(job.document,job.options):job.kind==='sketchEdit'?applySolidSketchEdit(job.document,job.options):job.kind==='boolean'?applySolidBoolean(job.document,job.options):job.kind==='sceneEdit'?applySolidSceneEdit(typeof job.document==='string'?parseDirectDocument(job.document):job.document,job.options):job.kind==='curveMatch'?matchSolidCurve(...job.args):job.kind==='surfaceMatch'?matchSolidSurface(...job.args):job.kind==='seamPrepare'?prepareSolidSurfaceSeams(...job.args):job.kind==='surfaceBuild'?buildSolidSurface(job.document,job.options):job.kind==='nurbsRefit'?refitSolidNurbs(job.document,job.options):job.kind==='profilePrepare'?prepareSolidProfile(job.document,job.ids,job.tolerance):job.kind==='profileEdit'?applySolidProfileEdit(job.document,job.options):job.kind==='bodyEdit'?applySolidBodyEdit(job.document,job.options):job.kind==='revolve'?applySolidRevolve(job.document,job.options):applyDirectExtrusionProfile(job.document,job.options))),cancel:()=>{},dispose:()=>{}})}
})
afterEach(()=>{previewWorkerRun.mockReset();displayWorkerRun.mockReset();primitiveWorkerRun.mockReset();measurementWorkerRun.mockReset();boundaryWorkerRun.mockReset();surfaceDisplayWorkerRun.mockReset();topologyWorkerRun.mockReset();bodyEdgesWorkerRun.mockReset();faceSketchWorkerRun.mockReset();bodySnapsWorkerRun.mockReset();sketchSnapsWorkerRun.mockReset()})
const svgWorkerRun=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/svgWorkerClient',()=>({createSvgWorkerClient:()=>({run:svgWorkerRun,dispose:()=>{}})}))
afterEach(()=>svgWorkerRun.mockReset())
const contactWorkerRun=vi.hoisted(()=>vi.fn())
const clearanceWorkerRun=vi.hoisted(()=>vi.fn())
vi.mock('../src/services/solidClearanceWorker',async()=>{
 const {inspectCadPairs}=await import('../src/services/cadInspection')
 const {inspectSolidIntersections}=await import('../src/services/solidDiagnostics')
 return {createSolidClearanceWorker:()=>({run:(job:any)=>job.kind==='meshContacts'?contactWorkerRun(job)??Promise.resolve().then(()=>inspectSolidIntersections(job.mesh,{maxWork:job.maxWork,maxContacts:job.maxContacts})):clearanceWorkerRun(job)??Promise.resolve(inspectCadPairs(job.bodies)),cancel:()=>{},dispose:()=>{}})}
})
afterEach(()=>{clearanceWorkerRun.mockReset();contactWorkerRun.mockReset()})
async function flushClearance(){await new Promise<void>(resolve=>realSetImmediate(resolve));for(let i=0;i<6;i++){await Promise.resolve();await nextTick()}}
class Node {
 parent:Node|null=null;children:Node[]=[];props:Record<string,any>={};style:Record<string,any>={};text='';value:any='';selected=false
 constructor(public tag:string){}
 get tagName(){return this.tag.toUpperCase()}
 get options(){return this.children.filter(n=>n.tag==='option')}
 get ownerSVGElement():Node|null{return this.tag==='svg'?this:this.parent?.ownerSVGElement??null}
 get clientWidth(){return 1000}get clientHeight(){return 1000}
 getRootNode(){return {activeElement:null}}
 focus(){}addEventListener(){}removeEventListener(){}setPointerCapture(){}hasPointerCapture(){return false}
 getScreenCTM(){return {inverse:()=>({})}}
 setAttribute(key:string,v:any){this.props[key]=v}removeAttribute(key:string){delete this.props[key]}
}
const renderer=createRenderer<Node,Node>({
 createElement:tag=>new Node(tag),createText:text=>{const n=new Node('#text');n.text=text;return n},createComment:()=>new Node('#comment'),
 setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.children=[];n.text=t},parentNode:n=>n.parent,nextSibling:n=>n.parent?.children[n.parent.children.indexOf(n)+1]??null,
 patchProp:(n,k,_old,v)=>{n.props[k]=v;if(k==='value')n.value=v},
 insert:(n,p,anchor)=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n);n.parent=p;const i=anchor?p.children.indexOf(anchor):-1;i<0?p.children.push(n):p.children.splice(i,0,n)},
 remove:n=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n)},setScopeId:()=>{},insertStaticContent:()=>{throw Error('Unexpected static content')}
})
const mounts:Array<()=>void>=[]
afterEach(()=>{mounts.splice(0).forEach(f=>f());const settings=useModelingGrid();settings.enabled.value=true;settings.grid.value=true;settings.geometry.value=true;settings.guides.value=true;settings.step.value=10;settings.unit.value='mm';vi.unstubAllGlobals()})
async function mount(props: Record<string,unknown> = {}, savedDraft?:string, preferences:Record<string,string>={'scad-solid-dock':'true'}){
 const sketch={id:'s',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]] as [number,number][]}
 let stored=savedDraft??stringifyMeshJson({version:1,sketches:[sketch,{id:'circle',name:'Circle',closed:true,analytic:{kind:'circle',center:[20,5],radius:3,start:0,sweep:360},points:sampleCurve({kind:'circle',center:[20,5],radius:3,start:0,sweep:360})},{id:'line',name:'Line',closed:false,points:[[0,-5],[2,-5]]},{id:'boundary',name:'Boundary',closed:false,points:[[5,-10],[5,0]]}],bodies:[{...extrudeDirectSketch(sketch,10,'b'),name:'Cube'}]})
 vi.stubGlobal('navigator',{locks:{request:(_name:string,_options:unknown,action:()=>unknown)=>Promise.resolve(action())}})
 vi.stubGlobal('localStorage',{getItem:(k:string)=>k.includes('modeler')?stored:preferences[k]??null,setItem:(k:string,v:string)=>{if(k.includes('modeler'))stored=v;else preferences[k]=v}})
 const windowListeners=new Map<string,(e:any)=>void>()
 vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {});vi.stubGlobal('document',{activeElement:null});vi.stubGlobal('window',{document:{activeElement:null},addEventListener:(name:string,fn:(e:any)=>void)=>windowListeners.set(name,fn),removeEventListener:(name:string)=>windowListeners.delete(name)});vi.stubGlobal('SVGSVGElement',Node)
 vi.stubGlobal('DOMPoint',class {constructor(public x:number,public y:number){}matrixTransform(){return this}})
 const currentProps=shallowReactive({open:true,locale:'en',canAppend:true,remainingSource:100000,...props})
 const instance=shallowRef<any>(null)
 const root=new Node('root'),app=renderer.createApp({setup:()=>()=>h(DirectModeler,{...currentProps,ref:instance})});app.mount(root);mounts.push(()=>app.unmount());await nextTick();await flushClearance()
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)]
 const text=(n:Node):string=>n.text+n.children.map(text).join('')
 const findButton=(name:string)=>all().find(n=>n.tag==='button'&&(text(n)===name||n.props['aria-label']===name||n.props['aria-label']===name.replace(/^\+ /,'')))
 const button=(name:string)=>{const n=findButton(name);if(!n)throw Error('Missing button '+name);return n}
 // Context actions live in the command palette; fall back to executing the matching command by its label.
 const command=(name:string)=>{const label=name.replace(/ · .*$/,'');return (instance.value?.solidCommands as Array<{id:string;label:string;aliases?:readonly string[]}>|undefined)?.find(c=>c.label===label||c.aliases?.includes(label))}
 const click=async(name:string,shiftKey=false)=>{const n=findButton(name);if(n){n.props.onClick({shiftKey});await flushClearance();return}const c=command(name);if(!c)throw Error('Missing button '+name);instance.value.executeSolidCommand(c.id);await flushClearance()}
 const svg=()=>all().find(n=>n.tag==='svg'&&n.props['aria-label']==='3D body canvas')!
 const event=(n:Node,x=0,y=0)=>({button:0,target:n,currentTarget:n,clientX:x,clientY:y,pointerId:1,preventDefault(){},stopPropagation(){}})
 const pointer=async(n:Node,x=0,y=0)=>{n.props.onPointerdown(event(n,x,y));await nextTick()}
 return {commands:()=>instance.value.solidCommands as Array<{id:string;label:string;enabled?:boolean;disabledReason?:string}>,storageChanged:()=>windowListeners.get('storage')?.({key:'scad-solid-modeler-v1'}),all,text,button,click,svg,pointer,event,setProps:async(next:Record<string,unknown>)=>{Object.assign(currentProps,next);await nextTick()},preferences,serialized:()=>stored,doc:()=>{const parsed=JSON.parse(stored) as DirectDocument;return parsed.bodies.some(body=>body.instance&&!body.mesh)?JSON.parse(stringifyMeshJson(parseDirectDocument(stored))) as DirectDocument:parsed},field:async(value:number)=>{const input=all().find(n=>n.tag==='input'&&n.props['onUpdate:modelValue']&&n.props.step===.5)??all().find(n=>n.tag==='input'&&n.props.type==='number'&&n.props['onUpdate:modelValue']);if(!input)throw Error('Missing field');input.props['onUpdate:modelValue'](value);await flushClearance()}}
}

function cylinderSeed(): DirectDocument {
 const brep=createBrepCylinder(3,5),built=tessellateNurbsBrep(brep,2)
 return {version:1,sketches:[],bodies:[{id:'imported-cylinder',name:'Imported cylinder',brep,
  mesh:{positions:built.positions,indices:built.indices}}]}
}

it('edits physical material parameters without losing color or history, then restores the saved document',async()=>{
 const ui=await mount()
 await ui.click('Cube');await ui.click('Properties')
 const change=async(label:string,value:string)=>{
  const input=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']===label)!
  expect(input).toBeDefined()
  input.props.onChange({target:{value}});await nextTick()
 }
 await change('Metallic','1');await change('Roughness','0.2');await change('Material color','#b87333')
 expect(ui.doc().bodies[0].material).toEqual({name:'Custom',color:'#b87333',metallic:1,roughness:.2})
 await ui.click('↶')
 expect(ui.doc().bodies[0].material).toMatchObject({color:'#7094ba',metallic:1,roughness:.2})
 await ui.click('↷')
 const saved=ui.serialized(),restored=await mount({},saved)
 expect(restored.doc().bodies[0].material).toEqual({name:'Custom',color:'#b87333',metallic:1,roughness:.2})
})

it('imports STEP through the Solid file input and keeps Undo independent of the saved original',async()=>{
 const ui=await mount(),before=ui.doc()
 vi.stubGlobal('indexedDB',new IDBFactory())
 const text=readFileSync(resolve('tests/fixtures/step-v6/self-authored-ap242-assembly.step'),'utf8')
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.step,.stp')!
 const target={files:[{size:Buffer.byteLength(text),text:async()=>text}],value:'assembly.step'}
 await input.props.onChange({target});await nextTick()
 expect(target.value).toBe('')
 expect(ui.doc().bodies).toHaveLength(before.bodies.length+1)
 expect(ui.doc().bodies[0]).toEqual(before.bodies[0])
 expect(ui.all().some(n=>n.props.role==='alert')).toBe(false)
 expect((await stepStore.loadProjectStepModel())?.document?.occurrenceIdentities).toHaveLength(3)
 await ui.click('↶')
 expect(ui.doc()).toEqual(before)
 expect((await stepStore.loadProjectStepModel())?.document?.occurrenceIdentities).toHaveLength(3)
})

it('does not overwrite scene edits made while a STEP original is being saved',async()=>{
 const ui=await mount()
 let started!:()=>void,finish!:()=>void
 const saving=new Promise<void>(resolve=>{started=resolve})
 const pending=new Promise<void>(resolve=>{finish=resolve})
 const save=vi.spyOn(stepStore,'saveProjectStepModel').mockImplementation(()=>{started();return pending})
 try{
  const text=readFileSync(resolve('tests/fixtures/step-v6/self-authored-ap242-assembly.step'),'utf8')
  const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.step,.stp')!
  const importing=input.props.onChange({target:{files:[{size:Buffer.byteLength(text),text:async()=>text}],value:''}})
  await saving
  await ui.click('Box');const edited=ui.doc()
  finish();await importing;await nextTick()
  expect(ui.doc()).toEqual(edited)
  expect(ui.all().some(n=>n.props.role==='alert'&&ui.text(n).includes('scene changed'))).toBe(true)
 }finally{finish();save.mockRestore()}
})

it('refuses oversized and malformed STEP files without changing the scene or leaving import busy',async()=>{
 const ui=await mount(),before=ui.doc()
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.step,.stp')!
 const read=vi.fn(async()=> 'not STEP')
 const target={files:[{size:16*1024*1024+1,text:read}],value:'large.step'}
 await input.props.onChange({target});await nextTick()
 expect(read).not.toHaveBeenCalled()
 expect(target.value).toBe('')
 expect(ui.doc()).toEqual(before)
 target.files[0]!.size=8
 await input.props.onChange({target});await nextTick()
 expect(read).toHaveBeenCalledOnce()
 expect(ui.doc()).toEqual(before)
 expect(ui.button('Import STEP').props.disabled).toBe(false)
})

it('changes retained B-rep display detail and restores the previous mesh with Undo',async()=>{
 const seed=cylinderSeed(),before=JSON.parse(stringifyMeshJson(seed.bodies[0]))
 const ui=await mount({seedDocument:seed})
 await ui.click('Imported cylinder');await ui.click('B-rep detail')
 const field=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('B-rep detail'))!
 field.props['onUpdate:modelValue'](8);await nextTick();await ui.click('Retessellate')
 expect(ui.doc().bodies[0].mesh.indices.length).toBeGreaterThan(before.mesh.indices.length)
 expect(ui.doc().bodies[0].brep).toEqual(before.brep)
 expect(ui.button('↶').props.disabled).toBe(false)
 await ui.click('↶')
 expect(ui.doc().bodies[0]).toEqual(before)
 expect(JSON.parse(stringifyMeshJson(seed.bodies[0]))).toEqual(before)
})

it('applies a seed already present at mount and preserves Undo across reopening',async()=>{
 const seed=cylinderSeed(),ui=await mount({seedDocument:seed})
 expect(ui.doc().bodies.map(body=>body.id)).toEqual(['imported-cylinder'])
 expect(ui.doc().bodies[0].brep).toEqual(seed.bodies[0].brep)
 expect(ui.button('↶').props.disabled).toBe(false)
 await ui.click('↶')
 expect(ui.doc().bodies.map(body=>body.id)).toEqual(['b'])
 expect(ui.doc().sketches.map(sketch=>sketch.id)).toEqual(['s','circle','line','boundary'])
 const undone=ui.doc()
 await ui.setProps({open:false});await ui.setProps({open:true})
 expect(ui.doc()).toEqual(undone)
})

it('defers a closed workspace seed until opening and does not replay it over later edits',async()=>{
 const ui=await mount({open:false,seedDocument:cylinderSeed()})
 expect(ui.doc().bodies.map(body=>body.id)).toEqual(['b'])
 await ui.setProps({open:true})
 expect(ui.doc().bodies.map(body=>body.id)).toEqual(['imported-cylinder'])
 await ui.click('Box')
 const edited=ui.doc();expect(edited.bodies).toHaveLength(2)
 await ui.setProps({open:false});await ui.setProps({open:true})
 expect(ui.doc()).toEqual(edited)
 await ui.setProps({open:false,seedDocument:{version:1,sketches:[],bodies:[]}})
 expect(ui.doc()).toEqual(edited)
 await ui.setProps({open:true});expect(ui.doc().bodies).toEqual([])
 await ui.click('↶');expect(ui.doc()).toEqual(edited)
})
it('binds face selection, Push/Pull preview, confirm and undo to the document',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces');const polygon=ui.all(ui.svg()).find(n=>n.tag==='polygon')!;await ui.pointer(polygon)
 await ui.click('Push / Pull');expect(ui.doc().bodies[0].mesh).toBeDefined();await ui.click('Apply · Enter')
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1200)
 await ui.click('↶');expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
})

it('removes a fully subtracted B-rep body and restores it with Undo',async()=>{
 const brep=createBrepCylinder(3,5),built=tessellateNurbsBrep(brep,4)
 const body=(id:string)=>({id,name:id,brep:structuredClone(brep),mesh:{positions:built.positions,indices:built.indices}})
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[body('Stock'),body('Cutter')]}})
 // The command opens the guided panel with A and B prefilled from the selection; OK runs it.
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep A − B');await ui.click('OK')
 expect(ui.doc().bodies).toEqual([])
 await ui.click('↶');expect(ui.doc().bodies.map(b=>b.id)).toEqual(['Stock','Cutter'])
})
it('reports the stated tolerance when a subtraction falls back to the numerical trace',async()=>{
 // An off-axis sphere in a cylinder wall has no exact family: the kernel traces the
 // intersection numerically, the body carries its tolerance and the notice says so.
 const stock=createBrepCylinder(3,5),cutter=transformNurbsBrep(createBrepSphere(2),[[1,0,0,2.5],[0,1,0,0.4],[0,0,1,2.5],[0,0,0,1]])
 const body=(id:string,brep:ReturnType<typeof createBrepCylinder>)=>{const built=tessellateNurbsBrep(brep,4);return {id,name:id,brep,mesh:{positions:built.positions,indices:built.indices}}}
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[body('Stock',stock),body('Cutter',cutter)]}})
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep A − B');await ui.click('OK')
 expect(ui.doc().bodies).toHaveLength(1)
 expect(ui.doc().bodies[0].brep!.toleranceMm).toBeGreaterThan(1e-6)
 expect(ui.text(ui.all()[0])).toContain('traced numerically within')
 // An exact pair (a sphere on the axis, fully inside: a cavity) keeps the kernel
 // tolerance and shows no such notice.
 await ui.click('↶')
 const axial=transformNurbsBrep(createBrepSphere(1.5),[[1,0,0,0],[0,1,0,0],[0,0,1,2.5],[0,0,0,1]])
 const exact=await mount({initialDocument:{version:1,sketches:[],bodies:[body('Stock',stock),body('Cutter',axial)]}})
 await exact.click('Stock');await exact.click('Cutter',true);await exact.click('B-rep A − B');await exact.click('OK')
 expect(exact.doc().bodies[0].brep!.toleranceMm).toBeLessThan(1e-6)
 expect(exact.text(exact.all()[0])).not.toContain('traced numerically')
})
it('binds an edge selection to chamfer and creates a shell with the selected opening',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Edges');const edge=ui.all(ui.svg()).find(n=>n.tag==='polyline'&&n.props.onPointerdown)!;await ui.pointer(edge)
 await ui.click('Chamfer 3D');await ui.click('Apply · Enter');expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(980)
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Shell');await ui.click('Apply · Enter')
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000-6*6*8)
})
it('runs authored B-rep boolean and refuses unsupported fillet edge combinations without mutation',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Box')
 const objectBoxes=ui.all().filter(n=>n.tag==='button'&&ui.text(n).startsWith('Box · 3D'))
 objectBoxes[0].props.onClick({shiftKey:false});await nextTick();objectBoxes[1].props.onClick({shiftKey:true});await nextTick()
 await ui.click('B-rep Union')
 expect(ui.doc().bodies.filter(b=>b.brep)).toHaveLength(1)
 const before=ui.doc()
 await ui.click('Edges');const edges=ui.all(ui.svg()).filter(n=>n.tag==='polyline'&&n.props.onPointerdown),edge=edges[0],ends=new Set(String(edge.props.points).split(' '))
 await ui.pointer(edge);const connected=edges.slice(1).find(item=>String(item.props.points).split(' ').some(point=>ends.has(point)))!
 connected.props.onPointerdown({...ui.event(connected),shiftKey:true});await nextTick();await ui.click('Fillet 3D')
 await ui.click('Apply · Enter')
 expect(ui.text(ui.all()[0])).toContain('The feature is unproven for this geometry')
 expect(ui.text(ui.all()[0])).toContain('select suitable edges')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.doc()).toEqual(before)
})
it('previews, applies and undoes a qualified exact fillet from an edge pick',async()=>{
 const ui=await mount();await ui.click('Box');const before=ui.doc()
 await ui.click('Edges')
 const edge=ui.all(ui.svg()).find(n=>n.tag==='polyline'&&n.props.onPointerdown)!
 await ui.pointer(edge);await ui.click('Fillet 3D')
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 expect(ui.doc()).toEqual(before)
 await ui.click('Apply · Enter')
 const body=ui.doc().bodies.at(-1)!
 expect(body.brep).toBeDefined()
 expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(8000-(1-Math.PI/4)*80,5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc().bodies.at(-1)!.brep).toEqual(body.brep)
})

it('locates an oversized fillet and recovers after reducing its radius',async()=>{
 const ui=await mount();await ui.click('Box');const before=ui.doc(),body=before.bodies.at(-1)!,brep=body.brep!
 const index=brep.edges.findIndex(edge=>{const [a,b]=edge.vertices.map(v=>brep.vertices[v].point);return a[0]===b[0]&&a[1]===b[1]})
 await ui.click('Edges');await ui.pointer(ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!)
 await ui.click('Fillet 3D')
 quantityField(ui,'Radius / size, mm').props['onUpdate:modelValue']('30');await flushClearance()
 const text=ui.text(ui.all()[0]);expect(text).toContain(body.name+' · edges ');expect(text).toContain('Reduce the radius or chamfer size')
 const keys=()=>ui.all().find(n=>String(n.props.class??'').includes('command-keys'))!
 expect(ui.text(keys())).not.toContain('Enter')
 expect(ui.text(keys())).toContain('Esc')
 expect(ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!.props.stroke).toBe('#f87171')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 quantityField(ui,'Radius / size, mm').props['onUpdate:modelValue']('1');await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 expect(ui.text(ui.all()[0])).not.toContain('The feature does not fit')
 expect(ui.text(keys())).toContain('Enter')
 expect(ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!.props.stroke).toBe('#ffc977')
 await ui.click('Apply · Enter');expect(ui.doc().bodies.at(-1)!.brep).not.toEqual(brep)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('authors a full sketch revolve as an explicitly faceted B-rep',async()=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Revolve')
 await vi.waitFor(()=>expect(ui.button('Apply · Enter').props.disabled).toBe(false),{timeout:3000});await ui.click('Apply · Enter')
 const body=ui.doc().bodies.at(-1)!
 expect(body.name).toMatch(/faceted B-rep/)
 expect(body.brep).toBeDefined()
 expect(body.brep?.faces.length).toBeGreaterThan(6)
 expect(inspectPolygonMesh(body.mesh).closed).toBe(true)
})
it.each(['x','y'])('keeps exact partial revolve outward oriented around %s and refuses implicit mesh combination',async axis=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Revolve')
 const set=async(label:string,value:unknown)=>{const field=ui.all().find(n=>['input','select'].includes(n.tag)&&n.parent&&ui.text(n.parent.tag==='span'?n.parent.parent!:n.parent).startsWith(label)&&n.props['onUpdate:modelValue'])!;field.props['onUpdate:modelValue'](value);await nextTick()}
 await set('Sketch axis',axis);await set('Axis offset, mm',-5);await set('Revolve surfaces','exact');await set('Angle, °',180);await ui.click('Add')
 await vi.waitFor(()=>expect(ui.text(ui.all()[0])).toContain('Exact B-rep revolve combination requires an authored B-rep target.'),{timeout:3000})
 await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(1)
 await ui.click('New');await vi.waitFor(()=>expect(ui.button('Apply · Enter').props.disabled).toBe(false),{timeout:3000});await ui.click('Apply · Enter')
 const body=ui.doc().bodies.at(-1)!
 expect(body.name).toMatch(/exact B-rep/);expect(body.brep?.faces).toHaveLength(10)
 const report=inspectPolygonMesh(body.mesh);expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(3000)
})
it('binds splitting and Shift selection to shared transforms',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Split');await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(2)
 const names=ui.doc().bodies.map(b=>b.name);await ui.click(names[0]);await ui.click(names[1],true);await ui.click('Transform selection');await ui.click('Esc');expect(ui.doc().bodies).toHaveLength(2)
 await ui.click('Duplicate · ⌘/Ctrl D');expect(ui.doc().bodies).toHaveLength(4)
})
it('binds analytic radius editing and offset without baking the curve',async()=>{
 const ui=await mount();await ui.click('Circle');await ui.click('Curve parameters')
 const inputs=ui.all().filter(n=>n.tag==='input'&&n.props['onUpdate:modelValue']);const radius=inputs.find(n=>n.parent&&ui.text(n.parent.tag==='span'?n.parent.parent!:n.parent).startsWith('Radius / size, mm'))!
 radius.props['onUpdate:modelValue'](5);await flushClearance();await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(5)
 await ui.click('Offset');await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(7)
 await ui.click('↶');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(5)
})
it('binds extend and trim to the selected 2D edge',async()=>{
 const ui=await mount();await ui.click('Line');await ui.click('Extend');await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='line')?.points.at(-1)).toEqual([5,-5])
 await ui.click('Trim');const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!,path=ui.all(svg).find(n=>n.tag==='path'&&n.props.d==='M 0,5 L 5,5')!
 await ui.pointer(path,2,5);expect(ui.doc().sketches.some(s=>s.id==='line')).toBe(false)
 await ui.click('↶');expect(ui.doc().sketches.find(s=>s.id==='line')?.points.at(-1)).toEqual([5,-5])
})
it('creates a sketch on the selected face and preserves its plane through extrusion',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Sketch on face')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 svg.props.onPointerdown({...ui.event(svg,1,-1),altKey:true});svg.props.onPointermove({...ui.event(svg,3,-3),altKey:true});svg.props.onPointerup({...ui.event(svg,3,-3),altKey:true});await flushClearance()
 const sketch=ui.doc().sketches.at(-1)!;expect(sketch.plane).toBeDefined();expect(sketch.points).toHaveLength(4)
 await ui.click('Extrude · E');await ui.click('New');await new Promise(resolve=>setTimeout(resolve,80));await nextTick();await ui.click('Apply · Enter')
 expect(ui.doc().bodies).toHaveLength(2);expect(inspectPolygonMesh(ui.doc().bodies[1].mesh).signedVolumeMm3).toBeCloseTo(40)
})
it('commits a gizmo drag once and undoes it',async()=>{
 const ui=await mount();await ui.click('Cube');const svg=ui.svg(),gizmo=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 const before=ui.doc();await ui.pointer(gizmo,0,0);svg.props.onPointermove({...ui.event(svg,20,0),altKey:true});svg.props.onPointerup({...ui.event(svg,20,0),altKey:true});await flushClearance()
 expect(ui.doc().bodies[0].mesh.positions).not.toEqual(before.bodies[0].mesh.positions);await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('drags a body itself, previewing in place and committing the exact move once',async()=>{
 const ui=await mount();await ui.click('Cube');const svg=ui.svg()
 // In 3D a plain drag orbits the camera; moving a body is its own mode.
 await ui.click('Move · G')
 const before=ui.doc(),face=ui.all(svg).find(n=>n.tag==='polygon'&&n.props.onPointerdown)!
 await ui.pointer(face,0,0)
 svg.props.onPointermove(ui.event(svg,30,0));await flushClearance()
 // The preview must not have been committed yet: the kernel runs once, on release.
 expect(ui.doc().bodies[0].mesh.positions).toEqual(before.bodies[0].mesh.positions)
 svg.props.onPointerup(ui.event(svg,30,0));await flushClearance()
 const moved=ui.doc().bodies[0]
 expect(moved.mesh.positions).not.toEqual(before.bodies[0].mesh.positions)
 // The exact translation keeps the body's B-rep rather than dropping it to a mesh.
 expect(Boolean(moved.brep)).toBe(Boolean(before.bodies[0].brep))
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('keeps gizmo labels and the canvas out of native text drag',async()=>{
 // A press on an axis label used to start a native text drag: macOS showed the copy badge
 // and a ghost of the label while the gizmo gesture underneath was cancelled.
 const ui=await mount();await ui.click('Cube');const svg=ui.svg()
 expect(svg.props.draggable).toBe('false')
 expect(typeof svg.props.onDragstart).toBe('function')
 expect(typeof svg.props.onSelectstart).toBe('function')
 // Axis labels live in the gizmo groups next to the axis line; every one must be inert.
 const gizmoGroups=ui.all(svg).filter(n=>n.tag==='g'&&n.children?.some((c:any)=>c.tag==='line'))
 expect(gizmoGroups.length).toBeGreaterThan(0)
 const labels=gizmoGroups.flatMap(g=>ui.all(g).filter(n=>n.tag==='text'))
 expect(labels.length).toBe(gizmoGroups.length)
 for(const label of labels)expect(label.props['pointer-events']).toBe('none')
})
it('box-selects multiple sketches and deletes the selection atomically',async()=>{
 const ui=await mount(),svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 await ui.click('Box select')
 await ui.pointer(svg,-1,-11);svg.props.onPointermove(ui.event(svg,11,11));svg.props.onPointerup(ui.event(svg,11,11));await flushClearance()
 await ui.click('Delete');expect(ui.doc().sketches.map(s=>s.name)).toEqual(['Circle']);await ui.click('↶');expect(ui.doc().sketches).toHaveLength(4)
})

it('creates closed primitives with undo in the embedded main scene editor',async()=>{
 const emitted:string[]=[]
 const ui=await mount({embedded:true,initialDocument:{version:1,sketches:[],bodies:[]},onAppend:(source:string)=>emitted.push(source)})
 for(const name of ['Box','Wedge','Cylinder','Cone','Sphere']) {
  await ui.click(name)
  const body=ui.doc().bodies.at(-1)!,report=inspectPolygonMesh(body.mesh)
  expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(0)
 }
 expect(ui.doc().bodies).toHaveLength(5)
 expect(ui.doc().bodies.find(body=>body.name.startsWith('Wedge'))?.brep?.faces).toHaveLength(5)
 expect(ui.doc().bodies.find(body=>body.name.startsWith('Cylinder'))?.brep?.faces).toHaveLength(6)
 expect(ui.doc().bodies.find(body=>body.name.startsWith('Sphere'))?.brep?.faces).toHaveLength(8)
 await ui.click('↶');expect(ui.doc().bodies).toHaveLength(4)
 await ui.click('Apply to code');expect(emitted).toHaveLength(1);expect(emitted[0].match(/polyhedron\(/g)).toHaveLength(4)
})
it('opens native NURBS CV tools without baking a body',async()=>{
 const ui=await mount()
 await ui.click('+ NURBS surface')
 expect(ui.doc().surfaces).toHaveLength(1)
 expect(ui.doc().bodies).toHaveLength(1)
 expect(ui.button('Apply CV')).toBeDefined()
 expect(ui.button('Bake surface to mesh body')).toBeDefined()
})
it('drags a native NURBS CV in the 3D viewport and commits one undo step',async()=>{
 const ui=await mount()
 await ui.click('+ NURBS surface')
 const before=ui.doc().surfaces![0].surface.controlPoints[0][0].slice()
 const svg=ui.svg(),cv=ui.all(svg).find(n=>n.tag==='circle'&&n.props.onPointerdown)!
 await ui.pointer(cv,0,0)
 svg.props.onPointermove(ui.event(svg,10,5))
 svg.props.onPointerup(ui.event(svg,10,5))
 await flushClearance()
 expect(ui.doc().surfaces![0].surface.controlPoints[0][0]).not.toEqual(before)
 await ui.click('↶')
 expect(ui.doc().surfaces![0].surface.controlPoints[0][0]).toEqual(before)
})

it('creates rational tube/frustum solids and retains the faceted cylinder option',async()=>{
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[]}})
 for(const name of ['Tube','Frustum']){
  await ui.click(name)
  const body=ui.doc().bodies.at(-1)!
  expect(body.brep?.faces.some(face=>face.surface.degreeU===2)).toBe(true)
  expect(inspectPolygonMesh(body.mesh).closed).toBe(true)
 }
 expect(ui.doc().bodies[0].brep!.faces.filter(face=>face.holes.length)).toHaveLength(2)
 const selector=ui.all().find(n=>n.tag==='select'&&n.options.some(o=>o.props.value==='faceted')&&n.options.some(o=>ui.text(o)==='Exact surfaces'))!
 selector.props['onUpdate:modelValue']('faceted');await nextTick();await ui.click('Cylinder')
 expect(ui.doc().bodies.at(-1)?.brep?.faces).toHaveLength(50)
})

it('selects a B-rep vertex without translating the body on click or pointer jitter',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Vertices')
 const svg=ui.svg(),before=ui.doc()
 const vertex=ui.all(svg).find(n=>n.tag==='circle'&&n.props.style?.cursor==='grab'&&Math.abs(n.props.cy)>1)!
 expect(vertex).toBeDefined()
 const x=Number(vertex.props.cx),y=Number(vertex.props.cy)
 await ui.pointer(vertex,x,y)
 svg.props.onPointerup(ui.event(svg,x,y));await flushClearance()
 expect(ui.doc()).toEqual(before)
 await ui.pointer(vertex,x,y)
 svg.props.onPointermove(ui.event(svg,x+1,y+1))
 svg.props.onPointerup(ui.event(svg,x+1,y+1));await flushClearance()
 expect(ui.doc()).toEqual(before)
 await ui.click('↶')
 expect(ui.doc().bodies.some(b=>b.id===before.bodies.at(-1)!.id)).toBe(false)
})

it('drags a B-rep vertex by the pointer delta without an initial coordinate jump',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Vertices')
 const svg=ui.svg(),before=ui.doc(),body=before.bodies.at(-1)!
 const vertex=ui.all(svg).find(n=>n.tag==='circle'&&n.props.style?.cursor==='grab'&&Math.abs(n.props.cy)>1)!
 const x=Number(vertex.props.cx),y=Number(vertex.props.cy)
 const a=unprojectDirectXY([x,y],defaultDirectCamera()),b=unprojectDirectXY([x+8,y+5],defaultDirectCamera())
 await ui.pointer(vertex,x,y)
 const end={...ui.event(svg,x+8,y+5),altKey:true}
 svg.props.onPointermove(end);svg.props.onPointerup(end);await flushClearance()
 const moved=ui.doc().bodies.find(b=>b.id===body.id)!
 expect(moved.brep!.topologyIds).toEqual(body.brep!.topologyIds)
 for(let i=0;i<body.mesh.positions.length;i++)expect(moved.mesh.positions[i]).toBeCloseTo(body.mesh.positions[i]+(i%3===2?0:b[i%3]-a[i%3]),5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('preserves authored B-rep for movement and push while refusing retained shell',async()=>{
 const ui=await mount();await ui.click('Box');const original=ui.doc().bodies.at(-1)!
 await ui.click('Move · G');const svg=ui.svg();await ui.pointer(ui.all(svg).find(n=>n.tag==='polygon'&&n.props['data-body']===original.id&&n.props.onPointerdown)!,0,0)
 svg.props.onPointermove(ui.event(svg,4,3));svg.props.onPointerup(ui.event(svg,4,3));await flushClearance()
 const moved=ui.doc().bodies.find(b=>b.id===original.id)!
 expect(moved.brep).toBeDefined();expect(moved.brep!.topologyIds).toEqual(original.brep!.topologyIds);expect(moved.brep!.vertices).not.toEqual(original.brep!.vertices)
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon'&&n.props['data-body']===original.id)!);await ui.click('Push / Pull');await ui.click('Apply · Enter')
 expect(ui.doc().bodies.find(b=>b.id===original.id)!.brep).toBeDefined()
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon'&&n.props['data-body']===original.id)!);await ui.click('Shell');await ui.click('Apply · Enter')
 expect(ui.text(ui.all()[0])).toContain('refuse faceted fallback')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.doc().bodies.find(b=>b.id===original.id)).toEqual(original)
})

it('previews and commits retained splits with positive-side identity and reversible history',async()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),mesh=tessellateNurbsBrep(brep,1)
 const seed:DirectDocument={version:1,sketches:[],bodies:[{id:'retained',name:'Retained stock',brep,mesh}]}
 const before=JSON.parse(stringifyMeshJson(seed)),ui=await mount({seedDocument:seed})
 const initialized=ui.doc()
 await ui.click('Retained stock');await ui.click('Split')
 expect(ui.doc()).toEqual(initialized)
 await ui.click('Apply · Enter')
 const result=ui.doc();expect(result.bodies).toHaveLength(2)
 expect(result.bodies[0].id).toBe('retained');expect(result.bodies[1].id).not.toBe('preview-split')
 expect(result.bodies[1].id).not.toBe('retained')
 expect(analyzeNurbsBrep(result.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(800,7)
 expect(analyzeNurbsBrep(result.bodies[1].brep!).signedVolumeMm3).toBeCloseTo(200,7)
 for(const body of result.bodies)expect(inspectPolygonMesh(body.mesh).signedVolumeMm3).toBeCloseTo(analyzeNurbsBrep(body.brep!).signedVolumeMm3,7)
 await ui.click('↶');expect(ui.doc().bodies).toEqual(before.bodies)
 expect(JSON.parse(stringifyMeshJson(seed))).toEqual(before)
})
it('previews, applies and undoes a retained ruled loft from ordered sketch selection',async()=>{
 const c=Math.SQRT1_2,points:[number,number][]=[[-1,-1],[1,-1],[1,1],[-1,1]]
 const seed:DirectDocument={version:1,sketches:[{id:'lower',name:'Lower',closed:true,points},{id:'upper',name:'Upper',closed:true,points:points.map(([x,y])=>[c*(x-y),c*(x+y)]),plane:{origin:[0,0,3],u:[1,0,0],v:[0,1,0]}}],bodies:[]}
 const ui=await mount({seedDocument:seed}),before=ui.doc()
 await ui.click('Lower');await ui.click('Upper',true);await ui.click('B-rep loft')
 expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await ui.click('Apply · Enter')
 const result=ui.doc();expect(result.bodies).toHaveLength(1)
 expect(result.bodies[0].brep!.faces).toHaveLength(6)
 expect(analyzeNurbsBrep(result.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(4*(2+c),6)
 expect(result.sketches).toEqual(before.sketches)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Upper');await ui.click('Lower',true);await ui.click('B-rep loft')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.doc()).toEqual(before)
})

it('shares cell units between panes and snaps drawn corners to the configured grid',async()=>{
 const ui=await mount(),settings=useModelingGrid()
 settings.geometry.value=false;settings.guides.value=false
 const units=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Cell units')!
 units.props.onChange({target:{value:'in'}});await nextTick()
 const size=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Cell size')!
 size.props.onChange({target:{valueAsNumber:1,value:'1'}});await nextTick()
 expect(settings.step.value).toBe(25.4)
 expect(ui.all().filter(n=>n.tag==='select'&&n.props['aria-label']==='Cell units').every(n=>n.props.value==='in')).toBe(true)
 await ui.click('Rectangle · R')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 await ui.pointer(svg,2,-2);svg.props.onPointermove(ui.event(svg,28,-29));await nextTick()
 expect(ui.all(svg).some(n=>n.tag==='text'&&ui.text(n)==='Grid')).toBe(true)
 svg.props.onPointerup(ui.event(svg,28,-29));await flushClearance()
 expect(ui.doc().sketches.at(-1)!.points).toEqual([[0,0],[25.4,0],[25.4,25.4],[0,25.4]])
 await ui.click('↶');expect(ui.doc().sketches).toHaveLength(4)
})
it('snaps a moved sketch by its grabbed corner, and Alt bypasses the same target',async()=>{
 const ui=await mount(),settings=useModelingGrid();settings.grid.value=false;settings.guides.value=false
 await ui.click('Profile')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 const path=()=>ui.all(svg).find(n=>n.tag==='path'&&n.props.d==='M 0,0 L 10,0 L 10,-10 L 0,-10 Z')!
 await ui.pointer(path(),.1,-.1);svg.props.onPointermove(ui.event(svg,20.2,-5.2));svg.props.onPointerup(ui.event(svg,20.2,-5.2));await flushClearance()
 expect(ui.doc().sketches[0].points[0]).toEqual([20,5])
 await ui.click('↶')
 await ui.pointer(path(),.1,-.1);svg.props.onPointermove({...ui.event(svg,20.2,-5.2),altKey:true});svg.props.onPointerup({...ui.event(svg,20.2,-5.2),altKey:true});await flushClearance()
 expect(ui.doc().sketches[0].points[0][0]).toBeCloseTo(20.1)
 expect(ui.doc().sketches[0].points[0][1]).toBeCloseTo(5.1)
})

it('snaps a 3D translation to the grid without unlocking the gizmo axis',async()=>{
 const ui=await mount();await ui.click('Cube');const svg=ui.svg(),gizmo=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 const before=ui.doc().bodies[0].mesh.positions
 await ui.pointer(gizmo,0,0);svg.props.onPointermove(ui.event(svg,20,0));svg.props.onPointerup(ui.event(svg,20,0));await flushClearance()
 const after=ui.doc().bodies[0].mesh.positions
 const delta=after[0]-before[0]
 expect(delta).toBe(5)
 for(let i=0;i<after.length;i++)expect(after[i]-before[i]).toBeCloseTo(i%3===0?delta:0)
 await ui.click('↶');expect(ui.doc().bodies[0].mesh.positions).toEqual(before)
})

it('starts with a flat shape from the main toolbar and extrudes it into a separate solid',async()=>{
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[]}})
 expect(ui.button('Extrude · E').props.disabled).toBe(true)
 await ui.click('Rectangle · R')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 await ui.pointer(svg,0,0);svg.props.onPointermove(ui.event(svg,20,-20));svg.props.onPointerup(ui.event(svg,20,-20));await flushClearance()
 expect(ui.doc().sketches).toHaveLength(1)
 expect(ui.doc().bodies).toHaveLength(0)
 expect(ui.button('Extrude · E').props.disabled).toBe(false)
 await ui.click('Extrude · E')
 const heightLabel=ui.all().find(n=>n.tag==='label'&&ui.text(n)==='Height, mm')!
 ui.all(heightLabel).find(n=>n.tag==='input')!.props['onUpdate:modelValue'](15)
 await nextTick();await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await ui.click('Apply · Enter')
 expect(ui.doc().bodies).toHaveLength(1)
 expect(ui.doc().sketches).toHaveLength(1)
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(6000)
 await ui.click('↶');expect(ui.doc().bodies).toHaveLength(0);expect(ui.doc().sketches).toHaveLength(1)
})

it('switches touch controls live and cancels a sketch edit when a pinch takes over', async()=>{
 const ui=await mount()
 await ui.click('Mouse')
 expect(ui.button('Touch').props['aria-pressed']).toBe(true)
 expect(ui.button('Navigate')).toBeTruthy()
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 ;(svg as any).getBoundingClientRect=()=>({left:0,top:0,width:1000,height:1000})
 const touch=(id:number,x:number,y:number)=>({...ui.event(svg,x,y),pointerId:id,pointerType:'touch'})
 const before=ui.doc(), initial=svg.props.viewBox
 svg.props.onPointerdownCapture(touch(1,400,500))
 svg.props.onPointerdown(touch(1,400,500))
 svg.props.onPointerdownCapture(touch(2,600,500))
 svg.props.onPointermoveCapture(touch(2,800,500))
 await nextTick()
 expect(svg.props.viewBox).not.toBe(initial)
 expect(Number(svg.props.viewBox.split(' ')[2])).toBeLessThan(Number(initial.split(' ')[2]))
 expect(ui.doc()).toEqual(before)
 svg.props.onPointerupCapture(touch(2,800,500))
 const pinched=svg.props.viewBox
 svg.props.onPointermoveCapture(touch(1,300,500))
 await nextTick()
 expect(svg.props.viewBox).toBe(pinched)
 await ui.click('Touch')
 expect(ui.button('Mouse').props['aria-pressed']).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('draws on a picked 3D face, then adds and cuts material from its supporting body',async()=>{
 await geometryKernel.warmGeometryKernel()
 const brep=createBrepBox([0,0,0],[10,10,10]),built=tessellateNurbsBrep(brep,2)
 const body={id:'base',name:'Base',brep,mesh:{positions:built.positions,indices:built.indices}}
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[body]}}),svg=ui.svg(),camera=defaultDirectCamera()
 const face=solidTopology(body.mesh).faces.find(f=>f.normal[2]>.99)!,triangle=face.triangles[0]
 const expected=Array.from(body.mesh.indices.slice(triangle*3,triangle*3+3),i=>projectDirectPoint(Array.from(body.mesh.positions.slice(i*3,i*3+3)),camera).slice(0,2).join(',')).join(' ')
 await ui.click('On face')
 await ui.pointer(ui.all(svg).find(n=>n.tag==='polygon'&&n.props.points===expected)!)
 await flushClearance();await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('Sketch snaps unavailable')
 expect(ui.all().some(n=>n.props['aria-label']==='sketch-snap-preparation')).toBe(false)
 const a=projectDirectPoint([2,2,10],camera),b=projectDirectPoint([4,4,10],camera)
 svg.props.onPointerdown({...ui.event(svg,a[0],a[1]),altKey:true});svg.props.onPointermove({...ui.event(svg,b[0],b[1]),altKey:true});svg.props.onPointerup({...ui.event(svg,b[0],b[1]),altKey:true});await flushClearance()
 const sketch=ui.doc().sketches[0]
 expect(sketch.supportBodyId).toBe('base');expect(sketch.plane!.origin[2]).toBeCloseTo(10)
 expect(ui.doc().bodies).toHaveLength(1)
 const setHeight=async(value:number)=>{const input=ui.all(ui.all().find(n=>n.tag==='label'&&ui.text(n)==='Height, mm')!).find(n=>n.tag==='input')!;input.props['onUpdate:modelValue'](value);await nextTick();await new Promise(resolve=>setTimeout(resolve,100));await nextTick()}
 await ui.click('Extrude · E');expect(ui.button('Add').props['aria-pressed']).toBe(true);await setHeight(3)
 expect(analyzeNurbsBrep(ui.doc().bodies[0].brep!).signedVolumeMm3).toBeCloseTo(1000)
 await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(1);expect(analyzeNurbsBrep(ui.doc().bodies[0].brep!).signedVolumeMm3).toBeCloseTo(1012)
 await ui.click('↶');await ui.click(sketch.name);await ui.click('Extrude · E');await setHeight(2);await ui.click('Cut');await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 expect(Number(ui.all(ui.all().find(n=>n.tag==='label'&&ui.text(n)==='Height, mm')!).find(n=>n.tag==='input')!.value)).toBe(-2)
 await ui.click('Apply · Enter');expect(analyzeNurbsBrep(ui.doc().bodies[0].brep!).signedVolumeMm3).toBeCloseTo(992)
 await ui.click('↶');expect(analyzeNurbsBrep(ui.doc().bodies[0].brep!).signedVolumeMm3).toBeCloseTo(1000)
})
it('selects an inner contour as a hole in the extrusion profile',async()=>{
 const sketch=(id:string,name:string,a:number,b:number)=>({id,name,closed:true,points:[[a,a],[b,a],[b,b],[a,b]]})
 const ui=await mount({initialDocument:{version:1,bodies:[],sketches:[sketch('outer','Outer',0,10),sketch('hole','Hole',3,5)]},initialSelection:'outer'})
 await ui.click('Extrude · E')
 const check=ui.all().find(n=>n.tag==='label'&&ui.text(n)==='Hole')!.children.find(n=>n.tag==='input')!
 check.props['onUpdate:modelValue'](['outer','hole']);await nextTick();await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(1)
 expect(analyzeNurbsBrep(ui.doc().bodies[0].brep!).signedVolumeMm3).toBeCloseTo(960)
 expect(ui.doc().bodies[0].brep!.faces.filter(f=>f.holes.length)).toHaveLength(2)
})

it('restores a saved body after asynchronous kernel warm-up without recording an empty undo state',async()=>{
 let release!:()=>void
 const ready=vi.spyOn(geometryKernel,'isGeometryKernelReady').mockReturnValue(false)
 const warm=vi.spyOn(geometryKernel,'warmGeometryKernel').mockImplementation(()=>new Promise<void>(resolve=>{release=resolve}))
 try{
  const ui=await mount()
  expect(ui.all().some(n=>n.props.class==='restore-loading')).toBe(true)
  release();await new Promise(resolve=>setTimeout(resolve,0));await nextTick()
  expect(ui.button('Cube')).toBeTruthy()
  expect(ui.doc().bodies).toHaveLength(1)
  expect(ui.button('↶').props.disabled).toBe(true)
  expect(ui.all().some(n=>n.props.role==='alert')).toBe(false)
 }finally{ready.mockRestore();warm.mockRestore()}
})

it('isolates selected bodies without changing the saved document and restores scene visibility',async()=>{
 const ui=await mount();const before=ui.doc()
 await ui.click('Cube')
 await ui.click('Isolate bodies')
 expect(ui.button('Exit isolation').props['aria-pressed']).toBe(true)
 expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.tag==='path'&&String(n.props.d??'').includes('NaN'))).toBe(false)
 await ui.click('Exit isolation')
 expect(ui.button('Isolate bodies').props['aria-pressed']).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('edits extrusion from the manipulator field and cancels the preview without persisting it',async()=>{
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 await ui.click('Extrude')
 const inline=ui.all().find(n=>n.props['aria-label']==='Dimension at manipulator, mm')!
 expect(inline).toBeDefined()
 inline.props['onUpdate:modelValue'](17);await nextTick()
 const height=ui.all().find(n=>n.tag==='label'&&ui.text(n).startsWith('Height, mm'))!
 expect(Number(ui.all(height).find(n=>n.tag==='input')?.value)).toBe(17)
 const cancel=ui.all().find(n=>n.tag==='button'&&ui.text(n).includes('cancel')&&ui.text(n).includes('Esc'))!
 cancel.props.onClick();await nextTick()
 expect(ui.all().some(n=>n.props['aria-label']==='Dimension at manipulator, mm')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

async function commandKey(ui: Awaited<ReturnType<typeof mount>>, key: string, target?: Node, extra = {}) {
 const workspace=ui.all().find(n=>n.tag==='section'&&n.props.onKeydown)!
 const prevented=vi.fn()
 workspace.props.onKeydown({key,target:target??{matches:()=>false,closest:()=>null},preventDefault:prevented,stopPropagation(){},...extra})
 await nextTick()
 return prevented
}

it('cancels subtraction with Escape even when focus is inside its operand panel',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc()
 await ui.click('B-rep A − B')
 const panel=ui.all().find(n=>n.props.class==='operation-card subtract-card')!
 expect(panel).toBeDefined()
 await commandKey(ui,'Escape',{closest:()=>panel} as unknown as Node)
 expect(ui.all().some(n=>n.props.class==='operation-card subtract-card')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('leaves native Enter on a Cancel button alone and ignores composing Enter',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 const cancel=ui.button('Esc')
 expect(await commandKey(ui,'Enter',{closest:()=>cancel} as unknown as Node)).not.toHaveBeenCalled()
 expect(await commandKey(ui,'Enter',undefined,{isComposing:true})).not.toHaveBeenCalled()
 expect(ui.doc()).toEqual(before)
 await ui.click('Esc');expect(ui.doc()).toEqual(before)
})

it('does not apply incomplete subtraction and closes its command before Undo',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('B-rep A − B');const before=ui.doc()
 expect(ui.button('OK').props.disabled).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await ui.click('↶')
 expect(ui.all().some(n=>n.props.class==='operation-card subtract-card')).toBe(false)
 expect(ui.doc().bodies).toHaveLength(before.bodies.length-1)
})

it('applies a command from numeric input and reverses exactly one history entry',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 const input=ui.all().find(n=>n.props['aria-label']==='Dimension at manipulator, mm')!
 input.props['onUpdate:modelValue'](5);await flushClearance()
 await commandKey(ui,'Enter',{closest:()=>null} as unknown as Node)
 const applied=ui.doc();expect(applied.bodies).toHaveLength(2)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(applied)
})
it('repeats a face operation only after a new face is selected and preserves its distance',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces')
 await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon'&&n.props.onPointerdown)!)
 await ui.click('Push / Pull');quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('0.2 cm');await nextTick()
 await ui.click('Apply · Enter');const after=ui.doc()
 expect(ui.button('Repeat · Shift R').props.disabled).toBe(true)
 await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon'&&n.props.onPointerdown)!)
 expect(ui.button('Repeat · Shift R').props.disabled).toBe(false)
 await ui.click('Repeat · Shift R');expect(Number(quantityField(ui,'Distance, mm').value)).toBe(2)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after)
})

function quantityField(ui:Awaited<ReturnType<typeof mount>>,label:string) {
 const parent=ui.all().find(n=>n.tag==='label'&&ui.text(n).startsWith(label))!
 return ui.all(parent).find(n=>n.tag==='input')!
}
it('previews a variable fillet with endpoint labels and refuses equal endpoint radii',async()=>{
 const ui=await mount();await ui.click('Box');const before=ui.doc(),body=before.bodies.at(-1)!,brep=body.brep!
 const index=brep.edges.findIndex(edge=>{const [a,b]=edge.vertices.map(v=>brep.vertices[v].point);return a[0]===b[0]&&a[1]===b[1]})
 await ui.click('Edges')
 await ui.pointer(ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!)
 await ui.click('Fillet 3D')
 const mode=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Fillet type')!
 mode.props['onUpdate:modelValue']('variable');await nextTick()
 expect(ui.text(ui.svg())).toContain('A');expect(ui.text(ui.svg())).toContain('B')
 quantityField(ui,'Radius B, mm').props['onUpdate:modelValue']('2');await nextTick()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 quantityField(ui,'Radius B, mm').props['onUpdate:modelValue']('3');await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await ui.click('Apply · Enter')
 expect(analyzeNurbsBrep(ui.doc().bodies.at(-1)!.brep!).signedVolumeMm3).toBeCloseTo(8000-(1-Math.PI/4)*20*19/3,4)
})
it('shares unit-aware values between panel and manipulator and blocks invalid input',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Split');const before=ui.doc(),field=quantityField(ui,'Distance, mm')
 field.props['onUpdate:modelValue']('0.5 cm');await nextTick()
 const inline=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension at manipulator, mm')!
 expect(Number(inline.value)).toBe(5)
 field.props['onUpdate:modelValue']('5 deg');await nextTick()
 expect(field.props['aria-invalid']).toBe(true);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 inline.props['onUpdate:modelValue']('3 mm');await flushClearance()
 expect(Number(field.value)).toBe(3);expect(field.props['aria-invalid']).toBe(false)
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(2)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('repeats extrusion on a new compatible profile as a cancellable preview',async()=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Extrude')
 quantityField(ui,'Height, mm').props['onUpdate:modelValue']('1.7 cm');await nextTick()
 await new Promise(resolve=>setTimeout(resolve,100));await nextTick();await ui.click('Apply · Enter')
 expect(ui.button('Repeat · Shift R').props.disabled).toBe(true)
 await ui.click('Circle');const before=ui.doc()
 expect(ui.button('Repeat · Shift R').props.disabled).toBe(false)
 await commandKey(ui,'R',undefined,{shiftKey:true})
 expect(Number(quantityField(ui,'Height, mm').value)).toBe(17);expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['aria-label']==='Dimension at manipulator, mm')).toBe(false)
 await ui.click('Repeat · Shift R');await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 await commandKey(ui,'Enter');expect(ui.doc().bodies).toHaveLength(before.bodies.length+1)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('shows a drag axis and cancels a constrained move without changing the document',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc(),svg=ui.svg()
 const gizmo=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 await ui.pointer(gizmo,0,0);svg.props.onPointermove({...ui.event(svg,20,0),altKey:true});await nextTick()
 expect(ui.text(ui.all()[0])).toContain('Constraint: X')
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).not.toContain('Constraint: X')
})
it('updates both quantity fields from the split handle and cancels without committing',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Split');const before=ui.doc(),svg=ui.svg()
 const handle=ui.all(svg).find(n=>n.tag==='circle'&&n.props.fill==='#ffc977'&&n.props.onPointerdown)!
 await ui.pointer(handle,0,0);svg.props.onPointermove({...ui.event(svg,0,1),altKey:true});await nextTick()
 const panel=quantityField(ui,'Distance, mm'),inline=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension at manipulator, mm')!
 expect(Number(panel.value)).not.toBe(2);expect(Number(panel.value)).toBe(Number(inline.value))
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})
it('hides and locks objects without changing saved geometry, and prevents tree selection',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc()
 await ui.click('Hide: Cube');expect(ui.button('Cube').props.disabled).toBe(true)
 expect(ui.all(ui.svg()).filter(n=>n.props['data-body']==='b')).toHaveLength(0)
 await ui.click('Cube');expect(ui.button('Cube').props['aria-pressed']).toBe(false)
 await ui.click('Show: Cube');await ui.click('Lock: Cube')
 expect(ui.button('Cube').props.disabled).toBe(true)
 const polygons=ui.all(ui.svg()).filter(n=>n.props['data-body']==='b');expect(polygons.length).toBeGreaterThan(0)
 expect(polygons.every(n=>n.props['pointer-events']==='none')).toBe(true)
 await ui.pointer(polygons[0]);expect(ui.button('Cube').props['aria-pressed']).toBe(false)
 await ui.click('Unlock: Cube');await ui.click('Cube');expect(ui.button('Cube').props['aria-pressed']).toBe(true)
 expect(ui.doc()).toEqual(before)
})
it('creates bodies in the active group and keeps new bodies visible during isolation with Undo',async()=>{
 const ui=await mount();await ui.click('New empty group')
 const name=ui.doc().groups![0].name
 await ui.click('Cube');await ui.click('Isolate bodies');const before=ui.doc()
 await ui.click('Box');const body=ui.doc().bodies.at(-1)!
 expect(body.group).toBe(name)
 expect(ui.all(ui.svg()).some(n=>n.props['data-body']===body.id)).toBe(true)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc().bodies.at(-1)?.group).toBe(name)
})
it('keeps hidden sketches out of the canvas and restores them from the tree',async()=>{
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 await ui.click('Hide: Profile');expect(ui.button('Profile').props.disabled).toBe(true)
 await ui.click('Show: Profile');await ui.click('Profile');expect(ui.button('Profile').props['aria-pressed']).toBe(true)
 expect(ui.doc()).toEqual(before)
})
it('moves sketches into the active group and isolates all group members with reversible assignment',async()=>{
 const ui=await mount();await ui.click('New empty group');const name=ui.doc().groups![0].name
 await ui.click('Profile');const before=ui.doc();await ui.click('Move selection to group')
 expect(ui.doc().sketches.find(s=>s.id==='s')?.group).toBe(name)
 await ui.click('Isolate group: '+name)
 expect(ui.all(ui.svg()).filter(n=>n.props['data-body']==='b')).toHaveLength(0)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('assigns new NURBS objects to the active group and prevents deleting locked members',async()=>{
 const ui=await mount();await ui.click('New empty group');const name=ui.doc().groups![0].name
 await ui.click('NURBS curve');const curve=ui.doc().curves!.at(-1)!
 expect(curve.group).toBe(name)
 await ui.click('Lock: '+curve.name);const before=ui.doc()
 await ui.click('Delete group '+name);expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).toContain('Unlock the group objects first')
 await ui.click('Unlock: '+curve.name);await ui.click('Delete group '+name)
 expect((ui.doc().curves??[]).some(c=>c.id===curve.id)).toBe(false)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('edits a face sketch dimension with units before extrusion and restores both edits with Undo',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Sketch on face')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 svg.props.onPointerdown({...ui.event(svg,1,-1),altKey:true});svg.props.onPointermove({...ui.event(svg,3,-3),altKey:true});svg.props.onPointerup({...ui.event(svg,3,-3),altKey:true});await flushClearance()
 await ui.click('Sketch dimensions');await ui.click('Add dimension');const before=ui.doc()
 const input=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension 0')!
 input().props['onUpdate:modelValue']('bad');await nextTick();expect(ui.button('Apply dimension 0').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 input().props['onUpdate:modelValue']('0.3 cm');await nextTick();expect(ui.doc()).toEqual(before)
 await ui.click('Apply dimension 0');const resized=ui.doc();expect(resized.sketches.at(-1)!.points[1][0]-resized.sketches.at(-1)!.points[0][0]).toBeCloseTo(3)
 await ui.click('Extrude · E');await ui.click('New');await new Promise(resolve=>setTimeout(resolve,80));await nextTick();await ui.click('Apply · Enter')
 expect(ui.doc().bodies).toHaveLength(2)
 await ui.click('↶');expect(ui.doc()).toEqual(resized)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(resized)
})
it('undoes polyline points locally and commits the remaining line only on Enter',async()=>{
 const ui=await mount();const before=ui.doc();await ui.click('Polyline · L')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 for(const [x,y] of [[40,-40],[60,-40],[60,-60]]){svg.props.onPointerdown({...ui.event(svg,x,y),altKey:true});await nextTick()}
 await commandKey(ui,'z',undefined,{ctrlKey:true});expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Enter');expect(ui.doc().sketches.at(-1)!.points).toEqual([[40,40],[60,40]])
 expect(ui.doc().sketches.at(-1)!.closed).toBe(false)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('draws a slot with a validated width and undoes it as one operation',async()=>{
 const ui=await mount();const before=ui.doc();await ui.click('Slot')
 const width=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Slot width, mm')!
 width.props['onUpdate:modelValue']('0.4 cm');await nextTick()
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 svg.props.onPointerdown({...ui.event(svg,40,-40),altKey:true});svg.props.onPointermove({...ui.event(svg,50,-40),altKey:true});svg.props.onPointerup({...ui.event(svg,50,-40),altKey:true});await flushClearance()
 const slot=ui.doc().sketches.at(-1)!;expect(slot.closed).toBe(true);expect(slot.retainedProfile!.loops[0]).toHaveLength(6)
 expect(Math.max(...slot.points.map(p=>p[1]))-Math.min(...slot.points.map(p=>p[1]))).toBeCloseTo(4)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('keeps the document intact and explains conflicting length dimensions',async()=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Sketch dimensions')
 await ui.click('Add dimension');await ui.click('Add dimension');const before=ui.doc()
 const input=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension 0')!
 input.props['onUpdate:modelValue']('20 mm');await nextTick();await ui.click('Apply dimension 0')
 expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props.role==='alert'&&ui.text(n).includes('Dimensions conflict'))).toBe(true)
})

it('retains geometry and identities through 20 edits, command cancellation, full Undo/Redo and draft reload',async()=>{
 const ui=await mount(),snapshots=[ui.serialized()]
 for(let cycle=0;cycle<5;cycle++)for(const command of ['Box','New empty group','Move selection to group','Duplicate']){
  await ui.click(command);snapshots.push(ui.serialized())
 }
 expect(new Set(snapshots).size).toBe(21)
 const final=ui.doc();expect(new Set(final.bodies.map(b=>b.id)).size).toBe(final.bodies.length)
 await ui.click('Split');await commandKey(ui,'Escape');expect(ui.doc()).toEqual(final)
 for(let i=19;i>=0;i--){await ui.click('↶');expect(ui.doc()).toEqual(JSON.parse(snapshots[i]))}
 for(let i=1;i<=20;i++){await ui.click('↷');expect(ui.doc()).toEqual(JSON.parse(snapshots[i]))}
 const restored=await mount({},ui.serialized());expect(restored.doc()).toEqual(final)
 expect(restored.all().some(n=>n.props.role==='alert')).toBe(false)
},20000)
it('rejects a delayed JSON file instead of replacing newer edits',async()=>{
 const ui=await mount(),before=ui.serialized();let release!:(text:string)=>void
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 const importing=input.props.onChange({target:{files:[{size:before.length,text:()=>new Promise<string>(resolve=>release=resolve)}],value:'test.json'}})
 await ui.click('Box');const edited=ui.doc();release(before);await importing;await nextTick()
 expect(ui.doc()).toEqual(edited)
 expect(ui.all().some(n=>n.props.role==='alert'&&ui.text(n).includes('scene changed'))).toBe(false)
})

it('restores hidden, locked and isolated workspace context without changing saved geometry',async()=>{
 const ui=await mount();await ui.click('New empty group');const name=ui.doc().groups![0].name
 await ui.click('Cube');await ui.click('Move selection to group');await ui.click('Isolate group: '+name)
 await ui.click('Lock: Cube');await ui.click('Hide: Profile');const before=ui.doc()
 const restored=await mount({},ui.serialized(),{...ui.preferences})
 expect(restored.button('Cube').props.disabled).toBe(true)
 expect(restored.button('Profile').props.disabled).toBe(true)
 expect(restored.button('Exit isolation').props['aria-pressed']).toBe(true)
 expect(restored.button('Active group: '+name).props['aria-pressed']).toBe(true)
 expect(restored.doc()).toEqual(before)
})
it('renders a read-only section and localized open boundaries for a defective mesh',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Body diagnostics')
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
 expect(ui.text(ui.all()[0])).toContain('Open edges: 0')
 expect(ui.doc()).toEqual(before)
 const sectionBefore=ui.all().find(n=>n.props['data-diagnostic']==='section')!.props.points
 const normal=(axis:string)=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Normal '+axis)!
 normal('X').props['onUpdate:modelValue'](1);normal('Z').props['onUpdate:modelValue'](0);await flushClearance()
 expect(ui.all().find(n=>n.props['data-diagnostic']==='section')!.props.points).not.toEqual(sectionBefore)
 normal('X').props['onUpdate:modelValue'](0);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Section normal must be finite and nonzero')
 expect(ui.text(ui.all()[0])).toContain('Open edges: 0')
 expect(ui.doc()).toEqual(before)
 const broken=structuredClone(before);delete broken.bodies[0].brep;broken.bodies[0].mesh.indices=Array.from(broken.bodies[0].mesh.indices).slice(3) as never
 const other=await mount({initialDocument:broken});await other.click('Cube');await other.click('Body diagnostics')
 expect(other.all().some(n=>n.props['data-diagnostic']==='boundary')).toBe(true)
 expect(other.text(other.all()[0])).toContain('Close the highlighted boundaries')
})
it('measures explicit vertices without creating history or mutating the model',async()=>{
 const ui=await mount();await ui.click('Box');const before=ui.doc()
 await ui.click('Properties');await ui.click('Measurements')
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(true)
 expect(ui.all().filter(n=>n.tag==='output').map(ui.text).join(' ')).toContain('20.000000 mm')
 const input=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Vertex B')!
 input.props['onUpdate:modelValue'](9999);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Choose an existing vertex')
 expect(ui.doc()).toEqual(before)
})

it('discards cancelled queued previews and commits only the latest rapid parameter edit',async()=>{
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 vi.useFakeTimers()
 try{
  await ui.click('Extrude · E');await commandKey(ui,'Escape');await vi.advanceTimersByTimeAsync(100);await nextTick()
  expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
  await ui.click('Extrude · E')
  const input=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension at manipulator, mm')!
  input().props['onUpdate:modelValue']('12 mm');await nextTick()
  input().props['onUpdate:modelValue']('23 mm');await nextTick()
  await vi.advanceTimersByTimeAsync(100);await nextTick();await ui.click('Apply · Enter')
  expect(inspectPolygonMesh(ui.doc().bodies.at(-1)!.mesh).signedVolumeMm3).toBeCloseTo(2300)
 }finally{vi.useRealTimers()}
})

it('shows localized non-manifold edges and repair guidance without changing geometry',async()=>{
 const source=await mount(),document=source.doc();delete document.bodies[0].brep
 document.bodies[0].mesh={positions:[0,0,0,10,0,0,0,10,0,0,-10,0,0,0,10],indices:[0,1,2,0,1,3,1,0,4]} as never
 const ui=await mount({},stringifyMeshJson(document));await ui.click('Cube');await ui.click('Body diagnostics')
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='non-manifold')).toHaveLength(1)
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='boundary')).toHaveLength(6)
 expect(ui.text(ui.all()[0])).toContain('Remove extra faces or separate shells')
 expect(ui.doc()).toEqual(document)
})
it('marks a fully collapsed triangle even when its outline has zero length',async()=>{
 const source=await mount(),document=source.doc();delete document.bodies[0].brep
 document.bodies[0].mesh={positions:[0,0,0,0,0,0,0,0,0],indices:[0,1,2]} as never
 const ui=await mount({},stringifyMeshJson(document));await ui.click('Cube');await ui.click('Body diagnostics')
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='degenerate-marker')).toHaveLength(1)
 expect(ui.text(ui.all()[0])).toContain('Remove or rebuild them')
})

it.each(['Sweep','NURBS loft'])('previews %s, cancels and restores its native surface through undo/redo',async(command)=>{
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='Profile curve';b.name='Path curve'
 b.curve.controlPoints=b.curve.controlPoints.map(([x,y,z])=>[z,x,y+10])
 const seed={version:1,sketches:[],bodies:[],curves:[a,b]}
 const ui=await mount({},JSON.stringify(seed));await ui.click('Profile curve');await ui.click('Path curve',true)
 const before=ui.doc();await ui.click(command)
 expect(ui.all().some(n=>n.tag==='polygon'&&Number(n.props['fill-opacity'])===.45)).toBe(true)
 expect(ui.all().some(n=>String(n.props.class).includes('nurbs-card'))).toBe(false)
 expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click(command);await commandKey(ui,'Enter')
 const after=ui.doc();expect(after.surfaces).toHaveLength(1);expect(after.curves).toEqual(before.curves)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(after)
 const restored=await mount({},ui.serialized());expect(restored.doc()).toEqual(after)
})

it('shows sampled boundary gaps and keeps the document unchanged',async()=>{
 await geometryKernel.warmGeometryKernel()
 const make=(id:string,x:number)=>({id,name:id,surface:{degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[x,0,0],[x,10,0]],[[x+10,0,0],[x+10,10,0]]],weights:[[1,1],[1,1]]},segmentsU:4,segmentsV:4})
 const seed={version:1,sketches:[],bodies:[],surfaces:[make('Surface A',0),make('Surface B',10.25)]}
 const ui=await mount({},JSON.stringify(seed));await ui.click('Surface A');await ui.click('Surface B',true)
 const before=ui.doc();await ui.click('Inspect surface boundary')
 expect(ui.text(ui.all()[0])).toContain('Maximum gap: 0.250000')
 expect(ui.text(ui.all()[0])).toContain('Tolerance not confirmed')
 expect(ui.all().filter(n=>n.props['data-boundary-inspection'])).toHaveLength(2)
 expect(ui.doc()).toEqual(before)
})

it('requires the reduction error bound before committing and preserves the curve identity',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curve=createSolidNurbsCurve('line');curve.name='Reducible curve';curve.curve.controlPoints=[[0,0,0],[1,0,0],[2,0,0],[3,0,0]]
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}));await ui.click('Reducible curve')
 const before=ui.doc();await ui.click('Reduce curve degree')
 expect(ui.text(ui.all()[0])).toContain('Deviation upper bound: 0 mm')
 expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(true)
 expect(ui.doc()).toEqual(before);await commandKey(ui,'Enter')
 expect(ui.doc().curves![0].id).toBe('line');expect(ui.doc().curves![0].curve.degree).toBe(1)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('refuses a curve reduction outside tolerance without adding an undo entry',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curve=createSolidNurbsCurve('curved');curve.name='Curved input'
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}));await ui.click('Curved input')
 const before=ui.doc();await ui.click('Reduce curve degree')
 expect(ui.text(ui.all()[0])).toContain('Deviation exceeds the tolerance')
 expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 expect(ui.button('↶').props.disabled).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})
it('previews surface degree reduction on both axes and supports cancel and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const surface={degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,0])),weights:Array.from({length:3},()=>[1,1,1])}
 const seed={version:1,sketches:[],bodies:[],surfaces:[{id:'plane',name:'Plane',surface,segmentsU:4,segmentsV:4}]}
 const ui=await mount({},JSON.stringify(seed));await ui.click('Plane');const before=ui.doc()
 await ui.click('Reduce surface degree');expect(ui.text(ui.all()[0])).toContain('Deviation upper bound: 0 mm')
 expect(ui.doc()).toEqual(before);await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Reduce surface degree')
 const direction=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Direction')!;direction.props['onUpdate:modelValue']('v');await flushClearance()
 await commandKey(ui,'Enter');expect(ui.doc().surfaces![0].surface.degreeV).toBe(1);expect(ui.doc().surfaces![0].surface.degreeU).toBe(2)
 expect(ui.doc().surfaces![0].id).toBe('plane');await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it.each([false,true])('previews and commits a four-boundary patch (rational=%s) with undo and immutable inputs',async rational=>{
 await geometryKernel.warmGeometryKernel()
 const points=rational?[[[1,0,0],[1,1,0],[0,1,0]],[[1,0,2],[1,1,2],[0,1,2]],[[1,0,0],[1,0,2]],[[0,1,0],[0,1,2]]]:[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=points.map((controlPoints,i)=>({id:'edge'+i,name:['Bottom','Top','Left','Right'][i],curve:{degree:controlPoints.length-1,knots:[...Array(controlPoints.length).fill(0),...Array(controlPoints.length).fill(1)],controlPoints,weights:controlPoints.length===3?[1,Math.SQRT1_2,1]:[1,1]}}))
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves}));for(let i=0;i<4;i++)await ui.click(curves[i].name,i>0)
 const before=ui.doc();await ui.click('Coons patch');expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).toContain('Bottom · vMin');expect(ui.all().some(n=>n.tag==='polygon'&&Number(n.props['fill-opacity'])===.45)).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Coons patch');await commandKey(ui,'Enter');expect(ui.doc().surfaces).toHaveLength(1);expect(ui.doc().curves).toEqual(before.curves)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
// Two complete rational Coons previews include construction and tessellation.
},60_000)
it('repeats the chosen surface input roles after swapping profile and path',async()=>{
 await geometryKernel.warmGeometryKernel()
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='Profile input';b.name='Path input';b.curve.controlPoints=b.curve.controlPoints.map(([x,y,z])=>[z,x,y+10])
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[a,b]}));await ui.click('Profile input');await ui.click('Path input',true)
 await ui.click('Sweep');await ui.click('Reverse input order');await commandKey(ui,'Enter')
 const surface=ui.doc().surfaces![0].surface
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter')
 expect(ui.doc().surfaces).toHaveLength(2);expect(ui.doc().surfaces![1].surface).toEqual(surface)
})

it('rebuilds a straight curve with preview, cancel, apply and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curve=createSolidNurbsCurve('rebuild');curve.name='Rebuild input'
 curve.curve.controlPoints=[[0,0,0],[1,0,0],[2,0,0],[3,0,0]]
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}));await ui.click('Rebuild input')
 const before=ui.doc();await ui.click('Rebuild curve')
 expect(ui.text(ui.all()[0])).toContain('Control points')
 expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Rebuild curve');await commandKey(ui,'Enter')
 expect(ui.doc().curves![0].id).toBe('rebuild')
 expect(ui.doc().curves![0].curve.controlPoints).toHaveLength(6)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('refuses a rebuild outside tolerance and reuses accepted degree and control count',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curve=createSolidNurbsCurve('rebuild-parameters');curve.name='Rebuild parameters'
 curve.curve={degree:2,knots:[2,2,2,5,5,5],controlPoints:[[0,0,0],[1,2,0],[2,0,0]],weights:[1,1,1]}
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}));await ui.click(curve.name)
 const before=ui.doc();await ui.click('Rebuild curve')
 const set=async(label:string,value:number)=>{const input=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent.tag==='span'?n.parent.parent!:n.parent).startsWith(label)&&n.props['onUpdate:modelValue'])!;expect(input).toBeTruthy();input.props['onUpdate:modelValue'](value);await flushClearance()}
 await set('Control points',2)
 expect(ui.text(ui.all()[0])).toContain('Deviation exceeds the tolerance')
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before);expect(ui.button('↶').props.disabled).toBe(true)
 await set('Target degree',3);await set('Control points',6)
 expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc().curves![0].curve.degree).toBe(3)
 expect(ui.doc().curves![0].curve.controlPoints).toHaveLength(6)
 const accepted=ui.doc()
 await ui.click('Repeat · Shift R');expect(ui.text(ui.all()[0])).toContain('Control points')
 await commandKey(ui,'Enter');expect(ui.doc().curves![0].curve.controlPoints).toHaveLength(6)
 await commandKey(ui,'Escape')
 const restored=await mount({},JSON.stringify(accepted));expect(restored.doc()).toEqual(accepted)
})

it('previews a surface rebuild and preserves the original through cancel and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const surface={degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,0])),weights:Array.from({length:3},()=>[1,1,1])}
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],surfaces:[{id:'surface-rebuild',name:'Surface rebuild input',surface,segmentsU:4,segmentsV:4}]}))
 await ui.click('Surface rebuild input');const before=ui.doc();await ui.click('Rebuild surface')
 expect(ui.all().some(n=>n.tag==='polygon'&&Number(n.props['fill-opacity'])===.45)).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Rebuild surface');const direction=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Direction')!
 direction.props['onUpdate:modelValue']('v');await flushClearance();await commandKey(ui,'Enter')
 const out=ui.doc().surfaces![0];expect(out.id).toBe('surface-rebuild');expect(out.surface.controlPoints).toHaveLength(3);expect(out.surface.controlPoints[0]).toHaveLength(6)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('keeps a surface rebuild outside tolerance out of the document and undo history',async()=>{
 await geometryKernel.warmGeometryKernel()
 const surface={degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,i===1&&j===1?1:0])),weights:Array.from({length:3},()=>[1,1,1])}
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],surfaces:[{id:'bump-rebuild',name:'Bump rebuild',surface,segmentsU:4,segmentsV:4}]}))
 await ui.click('Bump rebuild');const before=ui.doc();await ui.click('Rebuild surface')
 const controls=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('Control points')&&n.props['onUpdate:modelValue'])!
 controls.props['onUpdate:modelValue'](2);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Deviation exceeds the tolerance')
 expect(ui.all().some(n=>n.tag==='polygon'&&Number(n.props['fill-opacity'])===.45)).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before);expect(ui.button('↶').props.disabled).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})

it('fits small NURBS geometry at its actual scale and focuses only the selected curve',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curves=[0,100].map((offset,i)=>({id:'fit'+i,name:'Fit curve '+i,curve:{degree:1,knots:[0,0,1,1],controlPoints:[[offset,0,0],[offset+2,0,0]],weights:[1,1]}}))
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves}))
 await ui.click('Fit');const entire=Number(ui.svg().props.viewBox.split(' ')[2]);expect(entire).toBeGreaterThan(100)
 await ui.click('Fit curve 0');const before=ui.doc();await ui.click('Focus selection')
 const focused=Number(ui.svg().props.viewBox.split(' ')[2]);expect(focused).toBeLessThan(4);expect(focused).toBeGreaterThan(.1)
 expect(ui.doc()).toEqual(before)
})

it('shows display-mesh clearance witnesses without modifying the document',async()=>{
 await geometryKernel.warmGeometryKernel()
 const body=(id:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)}}
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[body('Clearance A',[0,0,0],[10,10,10]),body('Clearance B',[13,0,0],[23,10,10])]}))
 await ui.click('Clearance A');const before=ui.doc();await ui.click('Measure vertices / edge');await ui.click('Body mesh clearance')
 expect(ui.text(ui.all()[0])).toContain('Choose a different body B.')
 const target=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Body B')!
 target.props['onUpdate:modelValue']('Clearance B');await nextTick();await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Clearance: 3.000000 mm')
 expect(ui.text(ui.all()[0])).toContain('B-rep accuracy is limited by tessellation')
 expect(ui.all().some(n=>n.props['data-measurement']==='clearance')).toBe(true)
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(false)
 expect(ui.doc()).toEqual(before)
 await ui.click('Scene');await ui.click('Clearance B',true);await ui.click('Two-body mesh clearance');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Clearance: 3.000000 mm')
 expect(ui.doc()).toEqual(before)
})

it('shows a tiny contained overlap without a false clearance or a rounded zero volume',async()=>{
 await geometryKernel.warmGeometryKernel()
 const body=(id:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)}}
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[body('Tiny outer',[0,0,0],[.01,.01,.01]),body('Tiny inner',[.001,.001,.001],[.0011,.0011,.0011])]}))
 await ui.click('Tiny outer');await ui.click('Tiny inner',true);const before=ui.doc()
 await ui.click('Two-body mesh clearance');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Clearance: 0.000000 mm')
 expect(ui.text(ui.all()[0])).toContain('Overlap: 1.000e-12 mm³')
 expect(ui.all().some(n=>n.props['data-measurement']==='clearance')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('ignores late clearance replies after the target changes',async()=>{
 await geometryKernel.warmGeometryKernel()
 let finish!:(value:unknown)=>void
 clearanceWorkerRun.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
 const body=(id:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)}}
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[body('Async A',[0,0,0],[1,1,1]),body('Async B',[4,0,0],[5,1,1])]}))
 await ui.click('Async A');await ui.click('Async B',true);const before=ui.doc();await ui.click('Two-body mesh clearance')
 expect(ui.text(ui.all()[0])).toContain('Computing clearance…')
 expect(clearanceWorkerRun.mock.calls[0][0].bodies[0]).not.toHaveProperty('brep')
 const target=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Body B')!
 target.props['onUpdate:modelValue']('Async A');await nextTick()
 finish([{a:'Async A',b:'Async B',gapMm:999,overlapMm3:0,closestPoints:[[0,0,0],[999,0,0]],displayMeshOnly:true}])
 await Promise.resolve();await nextTick();await Promise.resolve();await nextTick()
 expect(ui.text(ui.all()[0])).toContain('Choose a different body B.')
 expect(ui.text(ui.all()[0])).not.toContain('999.000000')
 expect(ui.all().some(n=>n.props['data-measurement']==='clearance')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('creates, places, updates and detaches a linked instance with undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');const before=ui.doc()
 await ui.click('Create linked instance');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const instance=ui.doc().bodies.find(b=>b.instance)!
 expect(instance.instance!.sourceId).toBe('b')
 await ui.click('Place instance');quantityField(ui,'X').props['onUpdate:modelValue']('20');await flushClearance();await commandKey(ui,'Enter')
 expect(ui.doc().bodies.find(b=>b.id===instance.id)!.instance!.matrix[0][3]).toBe(20)
 await ui.click('Select instance source');await ui.click('Transform selection');quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance();
 expect(new Set(ui.all().filter(n=>n.props['data-preview-body']).map(n=>n.props['data-preview-body']))).toEqual(new Set(['b',instance.id]));await commandKey(ui,'Enter')
 const linked=ui.doc().bodies.find(b=>b.id===instance.id)!
 expect(Math.max(...Array.from(linked.mesh.positions).filter((_,i)=>i%3===0))).toBe(35)
 await ui.click(instance.name);await ui.click('Make independent')
 expect(ui.doc().bodies.find(b=>b.id===instance.id)!.instance).toBeUndefined()
 await ui.click('↶');expect(ui.doc().bodies.find(b=>b.id===instance.id)!.instance!.sourceId).toBe('b')
})

it('refuses direct instance geometry edits and source edits affecting a locked instance',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const instance=ui.doc().bodies.find(b=>b.instance)!,before=ui.doc()
 await ui.click('Split')
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).toContain('Edit the source or detach the instance')
 await commandKey(ui,'Escape');await ui.click('Lock: '+instance.name);await ui.click('Cube')
 await ui.click('Transform selection');quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance();await commandKey(ui,'Enter')
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).toContain('Object is locked')
})

it('previews and applies instance rotation and scale without detaching the source',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const before=ui.doc(),instance=before.bodies.find(b=>b.instance)!
 await ui.click('Transform instance')
 quantityField(ui,'Rotation, °').props['onUpdate:modelValue']('90')
 quantityField(ui,'Scale').props['onUpdate:modelValue']('2');await flushClearance()
 expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Transform instance')
 quantityField(ui,'Rotation, °').props['onUpdate:modelValue']('90')
 quantityField(ui,'Scale').props['onUpdate:modelValue']('2');await flushClearance();await commandKey(ui,'Enter')
 const body=ui.doc().bodies.find(b=>b.id===instance.id)!
 expect(body.instance!.sourceId).toBe('b')
 expect(body.instance!.matrix[0][1]).toBeCloseTo(-2,10)
 expect(body.instance!.matrix[1][0]).toBeCloseTo(2,10)
 expect(ui.doc().bodies.find(b=>b.id==='b')).toEqual(before.bodies.find(b=>b.id==='b'))
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc().bodies.find(b=>b.id===instance.id)!.instance).toEqual(body.instance)
})

it('persists compact linked instances and restores their displayed geometry on reload',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const document=ui.doc(),text=ui.serialized(),wire=JSON.parse(text),linked=wire.bodies.find((body:any)=>body.instance)
 expect(linked.mesh).toBeUndefined();expect(linked.brep).toBeUndefined()
 const restored=await mount({},text)
 expect(restored.doc()).toEqual(document)
 await restored.click(linked.name);await restored.click('Make independent')
 const independent=JSON.parse(restored.serialized()).bodies.find((body:any)=>body.id===linked.id)
 expect(independent.instance).toBeUndefined();expect(independent.mesh.positions.length).toBeGreaterThan(0)
})

it.each(['move','rotate','scale'])('keeps source linkage when the instance gizmo performs %s',async(kind)=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 await ui.click('Gizmo: '+kind)
 const before=ui.doc(),instance=before.bodies.find(b=>b.instance)!,svg=ui.svg()
 const handle=()=>ui.all(svg).find(n=>kind==='rotate'?n.tag==='polyline'&&n.props.onPointerdown&&n.props.stroke==='#ff7777':n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 await ui.pointer(handle(),0,0);svg.props.onPointermove({...ui.event(svg,20,30),altKey:true});await flushClearance()
 expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.pointer(handle(),0,0);svg.props.onPointermove({...ui.event(svg,20,30),altKey:true});svg.props.onPointerup({...ui.event(svg,20,30),altKey:true});await flushClearance()
 const edited=ui.doc().bodies.find(b=>b.id===instance.id)!
 expect(edited.instance!.sourceId).toBe('b');expect(edited.instance!.matrix).not.toEqual(instance.instance!.matrix)
 expect(ui.doc().bodies.find(b=>b.id==='b')).toEqual(before.bodies.find(b=>b.id==='b'))
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})


it('moves a source and linked instance together once through the shared transform command',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const before=ui.doc(),instance=before.bodies.find(body=>body.instance)!
 await ui.click('Cube',true);await ui.click('Transform selection')
 quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance();await commandKey(ui,'Enter')
 const after=ui.doc(),maxX=(body:any)=>Math.max(...Array.from(body.mesh.positions as number[]).filter((_,i)=>i%3===0))
 expect(maxX(after.bodies.find(body=>body.id==='b'))).toBe(15)
 expect(maxX(after.bodies.find(body=>body.id===instance.id))).toBe(25)
 expect(after.bodies.find(body=>body.id===instance.id)!.instance!.sourceId).toBe('b')
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(after)
})

it('duplicates a source-instance pair with the copied instance linked to the copied source',async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const before=ui.doc();await ui.click('Cube',true);await ui.click('Duplicate')
 const after=ui.doc(),copiedSource=after.bodies.find(body=>body.name==='Cube · copy')!,copiedInstance=after.bodies.find(body=>body.name==='Cube · instance · copy')!
 expect(after.bodies).toHaveLength(before.bodies.length+2)
 expect(copiedInstance.instance!.sourceId).toBe(copiedSource.id)
 const xs=Array.from(copiedInstance.mesh.positions).filter((_,i)=>i%3===0)
 expect(Math.min(...xs)).toBe(20);expect(Math.max(...xs)).toBe(30)
 for(const original of before.bodies)expect(after.bodies.find(body=>body.id===original.id)).toEqual(original)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('saves and reloads a document larger than the localStorage draft quota',async()=>{
 await geometryKernel.warmGeometryKernel();vi.stubGlobal('indexedDB',new IDBFactory())
 const {solidDraftSnapshotId,readSolidDraftSnapshot}=await import('../src/services/solidDraftStore')
 const seed:DirectDocument={version:1,sketches:[],bodies:[{...extrudeDirectSketch({id:'p',name:'Big draft body',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'large-body')}],groups:Array.from({length:60},(_,i)=>({name:`Group ${i}`,source:' '.repeat(70000)}))}
 const ui=await mount({seedDocument:seed})
 await vi.waitFor(()=>expect(solidDraftSnapshotId(ui.serialized())).not.toBeNull())
 const marker=ui.serialized(),text=await readSolidDraftSnapshot(marker)
 expect(JSON.parse(text).bodies[0].id).toBe('large-body')
 const restored=await mount({},marker)
 await vi.waitFor(()=>expect(restored.text(restored.all()[0])).not.toContain('Restoring geometry…'))
 await restored.click(seed.bodies[0].name);await restored.click('Transform selection')
 quantityField(restored,'X').props['onUpdate:modelValue']('5');await flushClearance();await commandKey(restored,'Enter')
 await vi.waitFor(()=>expect(restored.serialized()).not.toBe(marker))
 const edited=JSON.parse(await readSolidDraftSnapshot(restored.serialized()))
 expect(Math.min(...edited.bodies[0].mesh.positions.filter((_:number,i:number)=>i%3===0))).toBe(5)
 await expect(readSolidDraftSnapshot(marker)).rejects.toThrow('missing')
},30000)

it('does not publish a delayed large draft after undo saves a newer small state',async()=>{
 await geometryKernel.warmGeometryKernel();vi.stubGlobal('indexedDB',new IDBFactory())
 const store=await import('../src/services/solidDraftStore')
 const seed:DirectDocument={version:1,sketches:[],bodies:[],groups:Array.from({length:60},(_,i)=>({name:`Group ${i}`,source:' '.repeat(70000)}))}
 const marker=await store.writeSolidDraftSnapshot(stringifyMeshJson(seed))
 let finish!:(value:string)=>void
 const spy=vi.spyOn(store,'writeSolidDraftSnapshot').mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
 try{
  const ui=await mount({seedDocument:seed});const prior=ui.serialized()
  expect(spy).toHaveBeenCalledOnce();await ui.click('↶');const restored=ui.serialized()
  expect(restored).toBe(prior)
  finish(marker)
  await vi.waitFor(async()=>{await expect(store.readSolidDraftSnapshot(marker)).rejects.toThrow('missing')})
  expect(ui.serialized()).toBe(restored)
 }finally{spy.mockRestore()}
})

it('retains the last durable draft when the next IndexedDB write fails',async()=>{
 await geometryKernel.warmGeometryKernel();vi.stubGlobal('indexedDB',new IDBFactory())
 const store=await import('../src/services/solidDraftStore')
 const seed:DirectDocument={version:1,sketches:[],bodies:[],groups:Array.from({length:60},(_,i)=>({name:`Group ${i}`,source:' '.repeat(70000)}))}
 const ui=await mount({seedDocument:seed})
 await vi.waitFor(()=>expect(store.solidDraftSnapshotId(ui.serialized())).not.toBeNull())
 const marker=ui.serialized(),saved=await store.readSolidDraftSnapshot(marker)
 const spy=vi.spyOn(store,'writeSolidDraftSnapshot').mockRejectedValueOnce(Error('quota exceeded'))
 try{
  await ui.click('Box');await vi.waitFor(()=>expect(ui.text(ui.all()[0])).toContain('quota exceeded'))
  expect(ui.serialized()).toBe(marker);expect(await store.readSolidDraftSnapshot(marker)).toBe(saved)
 }finally{spy.mockRestore()}
})

it('refuses a delayed draft restore after a new seed and undo even when history is empty again',async()=>{
 await geometryKernel.warmGeometryKernel();vi.stubGlobal('indexedDB',new IDBFactory())
 const store=await import('../src/services/solidDraftStore')
 const body=extrudeDirectSketch({id:'p',name:'Old restored body',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'old-body')
 const oldText=stringifyMeshJson({version:1,sketches:[],bodies:[body]}),marker=await store.writeSolidDraftSnapshot(oldText)
 let finish!:(text:string)=>void
 const spy=vi.spyOn(store,'readSolidDraftSnapshot').mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
 try{
  const ui=await mount({},marker);await vi.waitFor(()=>expect(spy).toHaveBeenCalledOnce())
  await ui.setProps({seedDocument:{version:1,sketches:[],bodies:[{...body,id:'new-body',name:'New seed'}]}})
  await ui.click('↶');const current=ui.serialized();expect(JSON.parse(current).bodies).toHaveLength(0)
  finish(oldText)
  await vi.waitFor(()=>expect(ui.text(ui.all()[0])).not.toContain('Restoring geometry…'))
  expect(ui.serialized()).toBe(current)
  expect(ui.text(ui.all()[0])).not.toContain('Old restored body')
  expect(ui.text(ui.all()[0])).not.toContain('Draft changed while restoring')
 }finally{spy.mockRestore()}
})

it('exports Blender from the file menu and persists the same identity on repeat export',async()=>{
 const brep=createBrepBox([0,0,0],[2,3,4])
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[{id:'box',name:'Box',brep,mesh:tessellateNurbsBrep(brep,1)}]}})
 const blobs:Blob[]=[],click=vi.fn(),anchor={href:'',download:'',click}
 vi.stubGlobal('window',{document:{createElement:()=>anchor}})
 vi.spyOn(URL,'createObjectURL').mockImplementation(blob=>{blobs.push(blob as Blob);return 'blob:qualification'})
 vi.spyOn(URL,'revokeObjectURL').mockImplementation(()=>{})
 try {
  await ui.click('Export for Blender')
  await vi.waitFor(()=>expect(click).toHaveBeenCalledTimes(1))
  const first=JSON.parse(await blobs[0].text())
  expect(first.bodies[0].id).toBe('box')
  expect(ui.doc().blenderProjectId).toBe(first.projectId)
  expect(anchor.download).toBe('solid-model.osv-blender.json')
  await ui.click('Export for Blender')
  await vi.waitFor(()=>expect(click).toHaveBeenCalledTimes(2))
  expect(JSON.parse(await blobs[1].text()).projectId).toBe(first.projectId)
 } finally {vi.restoreAllMocks()}
})

it('highlights the first disallowed triangle pair without changing the document',async()=>{
 const initial={version:1,sketches:[],bodies:[{id:'overlap',name:'Overlap',mesh:{positions:new Float64Array([0,0,0,2,0,0,0,2,0]),indices:new Uint32Array([0,1,2,0,1,2])}}]}
 const ui=await mount({initialDocument:initial})
 await ui.click('Overlap')
 const before=ui.serialized()
 await ui.click('Body diagnostics')
 await flushClearance()
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='intersection')).toHaveLength(2)
 const nodes=ui.all();expect(nodes.findIndex(n=>n.props['data-diagnostic']==='intersection')).toBeGreaterThan(nodes.findLastIndex(n=>n.props['data-diagnostic']==='boundary'))
 expect(ui.all().some(n=>ui.text(n).includes('Disallowed triangle contact: 1 / 2'))).toBe(true)
 expect(ui.serialized()).toBe(before)
})


it('discards a late contact report after diagnostics closes',async()=>{
 let finish!:(value:any)=>void
 contactWorkerRun.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
 const ui=await mount()
 await ui.click('Cube');await ui.click('Body diagnostics')
 expect(ui.all().some(n=>ui.text(n).includes('Inspecting mesh contacts…'))).toBe(true)
 await ui.click('Body diagnostics')
 finish({contact:{triangles:[0,1],point:[0,0,0],sharedVertices:0,toleranceMm:1e-9},lines:[[[0,0,0],[1,0,0],[0,1,0],[0,0,0]]],relativeTolerance:1e-9,scope:'display-mesh-first-contact'})
 await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='intersection')).toBe(false)
 await ui.click('Body diagnostics');await flushClearance()
 expect(ui.all().some(n=>ui.text(n).includes('No disallowed mesh contacts found'))).toBe(true)
})

it('separates an imported legacy project from the current Blender identity and restores both through history',async()=>{
 const ui=await mount({initialDocument:{version:1,blenderProjectId:'project-a',sketches:[],bodies:[]}})
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 async function load(document:object){
  const text=JSON.stringify(document)
  await input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'project.json'}})
  await nextTick()
 }
 await load({version:1,sketches:[],bodies:[],groups:[{name:'Imported',source:''}]})
 const importedId=ui.doc().blenderProjectId
 expect(importedId).toBeTruthy();expect(importedId).not.toBe('project-a')
 await ui.click('↶');expect(ui.doc().blenderProjectId).toBe('project-a')
 await ui.click('↷');expect(ui.doc().blenderProjectId).toBe(importedId)
 await load({version:1,blenderProjectId:'project-c',sketches:[],bodies:[]})
 expect(ui.doc().blenderProjectId).toBe('project-c')
})

it('gives a replacement seed its own exchange identity without replaying or mutating it',async()=>{
 const ui=await mount({initialDocument:{version:1,blenderProjectId:'old-project',sketches:[],bodies:[]}})
 const seed={version:1,sketches:[],bodies:[],groups:[{name:'New seed',source:''}]}
 await ui.setProps({seedDocument:seed})
 const identity=ui.doc().blenderProjectId
 expect(identity).toBeTruthy();expect(identity).not.toBe('old-project')
 expect(seed).not.toHaveProperty('blenderProjectId')
 await ui.setProps({open:false});await ui.setProps({open:true})
 expect(ui.doc().blenderProjectId).toBe(identity)
 await ui.click('↶');expect(ui.doc().blenderProjectId).toBe('old-project')
 await ui.click('↷');expect(ui.doc().blenderProjectId).toBe(identity)
 await ui.setProps({seedDocument:{...seed,blenderProjectId:'explicit-project'}})
 expect(ui.doc().blenderProjectId).toBe('explicit-project')
})

it('refuses SVG profiles when the document changes during worker conversion',async()=>{
 const ui=await mount()
 let finish!:(value:any)=>void
 svgWorkerRun.mockReturnValue(new Promise(resolve=>{finish=resolve}))
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.svg,image/svg+xml')!
 const target={files:[{name:'late.svg',size:100,text:async()=>'<svg/>'}],value:'late.svg'}
 const pending=input.props.onChange({target})
 for(let i=0;i<10&&!svgWorkerRun.mock.calls.length;i++){await Promise.resolve();await nextTick()}
 await ui.click('Cube');await ui.click('Delete')
 const afterEdit=ui.doc()
 finish({contours:[[[0,0],[1,0],[0,1]]],warnings:[]})
 await pending;await nextTick()
 expect(ui.doc()).toEqual(afterEdit)
 expect(ui.all().some(n=>ui.text(n).includes('The scene changed. Import SVG again.'))).toBe(true)
 expect(target.value).toBe('')
})

it('aborts SVG work on panel close and ignores a late worker result after reopening',async()=>{
 const ui=await mount(),before=ui.doc()
 let finish!:(value:any)=>void
 svgWorkerRun.mockReturnValue(new Promise(resolve=>{finish=resolve}))
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.svg,image/svg+xml')!
 const target={files:[{name:'late.svg',size:100,text:async()=>'<svg/>'}],value:'late.svg'}
 const pending=input.props.onChange({target})
 for(let i=0;i<10&&!svgWorkerRun.mock.calls.length;i++){await Promise.resolve();await nextTick()}
 const signal=svgWorkerRun.mock.calls[0][1].signal as AbortSignal
 expect(signal.aborted).toBe(false)
 await ui.setProps({open:false})
 expect(signal.aborted).toBe(true)
 await ui.setProps({open:true})
 finish({contours:[[[0,0],[1,0],[0,1]]],warnings:[]})
 await pending;await nextTick()
 expect(ui.doc()).toEqual(before)
 expect(target.value).toBe('')
 expect(ui.all().some(n=>n.props.role==='alert')).toBe(false)
})

it('supersedes an SVG export and cancels its replacement when the panel closes',async()=>{
 const ui=await mount()
 const finish:Array<(value:any)=>void>=[]
 svgWorkerRun.mockImplementation(()=>new Promise(resolve=>finish.push(resolve)))
 await ui.click('Cube')
 await ui.click('Selected body SVG · XY projection')
 for(let i=0;i<10&&finish.length<1;i++){await Promise.resolve();await nextTick()}
 const first=svgWorkerRun.mock.calls[0][1].signal as AbortSignal
 await ui.click('Selected body SVG · XY projection')
 for(let i=0;i<10&&finish.length<2;i++){await Promise.resolve();await nextTick()}
 expect(first.aborted).toBe(true)
 const second=svgWorkerRun.mock.calls[1][1].signal as AbortSignal
 expect(second.aborted).toBe(false)
 await ui.setProps({open:false})
 expect(second.aborted).toBe(true)
 for(const resolve of finish)resolve({svg:'<svg/>',warnings:[]})
 await Promise.resolve();await nextTick()
 expect(ui.all().some(n=>n.props.role==='alert')).toBe(false)
})

it.each(['en','ru'])('explains incompatible patch weights and keeps the document unchanged (%s)',async locale=>{
 await geometryKernel.warmGeometryKernel()
 const controls=[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=controls.map((controlPoints,i)=>({id:`edge-${i}`,name:`Edge ${i}`,curve:{degree:1,knots:[0,0,1,1],controlPoints,weights:i===0?[1,.5]:[1,1]}}))
 const ui=await mount({locale},JSON.stringify({version:1,sketches:[],bodies:[],curves}))
 for(let i=0;i<4;i++)await ui.click(`Edge ${i}`,i>0)
 const before=ui.doc();await ui.click('Coons patch')
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Несовместимые угловые веса':'Incompatible corner weights')
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})

it('marks both disconnected patch endpoints and clears the markers on cancellation',async()=>{
 await geometryKernel.warmGeometryKernel()
 const controls=[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[.5,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=controls.map((controlPoints,i)=>({id:`gap-${i}`,name:`Gap ${i}`,curve:{degree:1,knots:[0,0,1,1],controlPoints,weights:[1,1]}}))
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves}))
 for(let i=0;i<4;i++)await ui.click(`Gap ${i}`,i>0)
 const before=ui.doc();await ui.click('Coons patch')
 const markers=ui.all().find(n=>n.props['data-diagnostic']==='patch-gap')!
 expect(markers).toBeDefined()
 expect(ui.all(markers).filter(n=>n.tag==='circle')).toHaveLength(2)
 expect(ui.text(ui.all()[0])).toContain('bottom and left boundaries do not meet')
 expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape')
 expect(ui.all().some(n=>n.props['data-diagnostic']==='patch-gap')).toBe(false)
})

it('frames small NURBS geometry immediately after importing a Solid document',async()=>{
 const ui=await mount()
 const seed={version:1,sketches:[],bodies:[],curves:[{id:'small',name:'Small',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[100,200,0],[101,201,2]],weights:[1,1]}}]}
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 const text=JSON.stringify(seed),before=ui.svg().props.viewBox
 await input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'small.json'}});await nextTick()
 const box=ui.svg().props.viewBox.split(' ').map(Number)
 expect(ui.svg().props.viewBox).not.toBe(before)
 expect(box[2]).toBeLessThan(10)
 expect(box.every(Number.isFinite)).toBe(true)
 expect(ui.doc().curves![0].curve.controlPoints).toEqual(seed.curves[0].curve.controlPoints)
})

it('blocks a coarse framed sweep and preserves mode and sections when repeating',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curves=[
  {id:'p',name:'Sweep profile',curve:{degree:1,controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1],knots:[0,0,1,1]}},
  {id:'q',name:'Sweep path',curve:{degree:2,controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1],knots:[0,0,0,1,1,1]}}
 ]
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves}))
 await ui.click('Sweep profile');await ui.click('Sweep path',true);const before=ui.doc()
 await ui.click('Sweep')
 const set=async(label:string,value:unknown)=>{const n=ui.all().find(n=>n.props['aria-label']===label)!;expect(n).toBeTruthy();n.props['onUpdate:modelValue'](value);await flushClearance()}
 await set('Sweep orientation','framed');await set('Sweep sections',3)
 expect(ui.text(ui.all()[0])).toContain('Sampled deviation exceeds the budget')
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await set('Sweep sections',32);await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Sweep');await commandKey(ui,'Enter')
 expect(ui.doc().surfaces![0].name).toBe('Framed sweep');expect(ui.doc().curves).toEqual(before.curves)
 const result=ui.doc().surfaces![0].surface
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter')
 expect(ui.doc().surfaces![1].surface).toEqual(result)
 await ui.click('↶');await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('rebuilds a periodic curve while preserving its seam, identity, history and saved parameters',async()=>{
 await geometryKernel.warmGeometryKernel()
 const curve={id:'periodic-ui',name:'Periodic rebuild input',curve:{degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:[1,.8,1.2,1,1,.8],periodic:true}}
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}))
 await ui.click(curve.name);const before=ui.doc();await ui.click('Rebuild curve')
 expect(ui.text(ui.all()[0])).toContain('The periodic seam is preserved')
 const set=async(label:string,value:number)=>{const n=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent.tag==='span'?n.parent.parent!:n.parent).startsWith(label)&&n.props['onUpdate:modelValue'])!;expect(n).toBeTruthy();n.props['onUpdate:modelValue'](value);await flushClearance()}
 await set('Target degree',3);await set('Control points',15);await set('Deviation tolerance, mm',0)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await set('Deviation tolerance, mm',.2)
 expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Rebuild curve');await commandKey(ui,'Enter')
 const result=ui.doc().curves![0]
 expect(result.id).toBe(curve.id);expect(result.curve.periodic).toBe(true)
 expect(result.curve.controlPoints).toHaveLength(15)
 expect(result.curve.controlPoints.slice(-3)).toEqual(result.curve.controlPoints.slice(0,3))
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('gates G2 surface matching and preserves preview, cancel, repeat and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const surface={degreeU:3,degreeV:3,knotsU:[0,0,0,0,1,1,1,1],knotsV:[0,0,0,0,1,1,1,1],controlPoints:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>[i,j,.15*i*i+.1*i*j+.2*j*j])),weights:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>1+.03*i+.02*j+.01*i*j))}
 const a={id:'match-a',name:'Match reference',surface,segmentsU:4,segmentsV:4},b=structuredClone(a);b.id='match-b';b.name='Match edited';b.surface.controlPoints.forEach(row=>row.forEach(p=>p[0]+=5))
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],surfaces:[a,b]}))
 await ui.click(a.name);await ui.click(b.name,true);const before=ui.doc()
 await ui.click('Match surfaces G1/G2')
 const set=async(label:string,value:unknown)=>{const n=ui.all().find(n=>['input','select'].includes(n.tag)&&n.parent&&ui.text(n.parent).startsWith(label)&&n.props['onUpdate:modelValue'])!;expect(n).toBeTruthy();n.props['onUpdate:modelValue'](value);await flushClearance()}
 await set('Order',2);await set('Derivative tolerance, mm',0)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await set('Derivative tolerance, mm',1e-6)
 expect(ui.text(ui.all()[0])).toContain('Tolerance confirmed')
 expect(ui.all().some(n=>n.props['data-diagnostic']==='surface-match-boundaries')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Match surfaces G1/G2');await commandKey(ui,'Enter')
 const committed=ui.doc();expect(committed.surfaces![0]).toEqual(a);expect(committed.surfaces![1].id).toBe(b.id)
 expect(committed.surfaces![1].surface).not.toEqual(b.surface)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(ui.doc()).toEqual(committed)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('prepares both surface bases with cancellation, undo and reversal reset',async()=>{
 await geometryKernel.warmGeometryKernel()
 const a={id:'prep-a',name:'Prepare A',segmentsU:4,segmentsV:4,surface:{degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,.1*i*j])),weights:Array.from({length:3},()=>[1,1,1])}}
 const b=structuredClone(a);b.id='prep-b';b.name='Prepare B';b.surface.degreeV=3;b.surface.knotsV=[0,0,0,0,1,1,1,1];b.surface.controlPoints=b.surface.controlPoints.map(row=>[row[0],row[1],row[1],row[2]]);b.surface.weights=b.surface.weights.map(()=>[1,1,1,1])
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],surfaces:[a,b]}))
 await ui.click(a.name);await ui.click(b.name,true);const before=ui.doc()
 await ui.click('Prepare surface boundaries')
 const checkbox=ui.all().find(n=>n.tag==='input'&&n.props.type==='checkbox'&&n.parent&&ui.text(n.parent).includes('Reverse B direction'))!
 checkbox.props['onUpdate:modelValue'](true);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Tolerance confirmed')
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Prepare surface boundaries');await commandKey(ui,'Enter')
 expect(ui.doc().surfaces![0].surface.degreeV).toBe(3)
 expect(ui.doc().surfaces![0].surface.knotsV).toEqual(ui.doc().surfaces![1].surface.knotsV)
 await ui.click('Match surfaces G1/G2')
 const reverse=ui.all().find(n=>n.tag==='input'&&n.props.type==='checkbox'&&n.parent&&ui.text(n.parent).includes('Reverse B direction'))!
 expect(reverse.props.checked).not.toBe(true)
 await commandKey(ui,'Escape');await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('previews curve G1 with endpoint roles, refusal, cancellation, repeat and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const a=createSolidNurbsCurve('curve-a'),b=createSolidNurbsCurve('curve-b');a.name='Reference curve';b.name='Edited curve'
 b.curve.controlPoints.forEach(p=>p[0]+=50)
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[a,b]}))
 await ui.click(a.name);await ui.click(b.name,true);const before=ui.doc()
 await ui.click('Match curves G1')
 const set=async(name:string,value:unknown)=>{const n=ui.all().find(n=>n.props['aria-label']===name&&n.props['onUpdate:modelValue'])!;expect(n).toBeTruthy();n.props['onUpdate:modelValue'](value);await flushClearance()}
 await set('Endpoint A','start');await set('Endpoint B','end')
 await set('Angular tolerance, °','0 deg');await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await set('Angular tolerance, °','0.000001 deg')
 expect(ui.text(ui.all()[0])).toContain('Tolerance confirmed')
 expect(ui.all().some(n=>n.props['data-preview']==='curve-match')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Match curves G1');await commandKey(ui,'Enter')
 const committed=ui.doc();expect(committed.curves![0]).toEqual(a)
 expect(committed.curves![1].curve.controlPoints[3]).toEqual(a.curve.controlPoints[0])
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(ui.doc()).toEqual(committed)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('prepares an open profile with gap diagnostics, cancel, apply and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const a={id:'profile-a',name:'Open profile',closed:false,points:[[0,0],[10,0],[10,10],[0,10],[0,.1]]}
 const ui=await mount({},JSON.stringify({version:1,sketches:[a],bodies:[]}));await ui.click(a.name);const before=ui.doc()
 await ui.click('Prepare profile')
 expect(ui.all().some(n=>n.props['data-diagnostic']==='profile-preparation')).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 const input=ui.all().find(n=>n.props['aria-label']==='Gap tolerance, mm'&&n.props['onUpdate:modelValue'])!
 input.props['onUpdate:modelValue']('0.11 mm');await flushClearance()
 expect(ui.all().some(n=>n.props['data-preview']==='prepared-profile')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Prepare profile');await commandKey(ui,'Enter')
 expect(ui.doc().sketches[0].closed).toBe(true);expect(ui.doc().sketches[0].points).toEqual(a.points)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(ui.doc().sketches[0].closed).toBe(true)
})

it('marks the actual intersecting profile segments and refuses application',async()=>{
 await geometryKernel.warmGeometryKernel()
 const sketch={id:'crossing',name:'Crossing',closed:false,points:[[0,0],[2,2],[0,2],[2,0],[0,0]]}
 const ui=await mount({},JSON.stringify({version:1,sketches:[sketch],bodies:[]}));await ui.click(sketch.name);const before=ui.doc()
 await ui.click('Prepare profile')
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='profile-segment').map(n=>n.props['data-segment-index'])).toEqual([0,2])
 expect(ui.text(ui.all()[0])).toContain('The marked segments intersect')
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})

it('unions exact profiles with retained arcs, cancel, undo and a recoverable document',async()=>{
 await geometryKernel.warmGeometryKernel()
 const rectangle={id:'rect',name:'Rectangle profile',closed:true,points:[[-4,-3],[3,-3],[3,3],[-4,3]]}
 const circle={id:'circle',name:'Round profile',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}
 const ui=await mount({},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[rectangle,circle],bodies:[]}))))
 await ui.click(rectangle.name);await ui.click(circle.name,true);const before=ui.doc()
 await ui.click('Union exact profiles');expect(ui.all().some(n=>n.props['data-preview']==='retained-profile')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Union exact profiles');await commandKey(ui,'Enter')
 const joined=ui.doc();expect(joined.sketches).toHaveLength(1);expect(joined.sketches[0].id).toBe(rectangle.id)
 expect(joined.sketches[0].retainedProfile!.loops.flat().some(c=>c.degree===2)).toBe(true)
 expect(parseDirectDocument(JSON.stringify(joined)).sketches).toEqual(joined.sketches)
 await ui.click('↶');expect(parseDirectDocument(JSON.stringify(ui.doc()))).toEqual(parseDirectDocument(JSON.stringify(before)))
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(parseDirectDocument(JSON.stringify(ui.doc()))).toEqual(parseDirectDocument(JSON.stringify(joined)))
})

it('prepares a retained arc profile with preview, cancel, undo and repeat',async()=>{
 await geometryKernel.warmGeometryKernel()
 const arc={id:'arc',name:'Assembly semicircle',closed:false,points:[],analytic:{kind:'arc',center:[0,0],radius:2,start:0,sweep:180}}
 const line={id:'line',name:'Assembly diameter',closed:false,points:[[-2,0],[2,0]]}
 const ui=await mount({},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[arc,line],bodies:[]}))))
 await ui.click(arc.name);await ui.click(line.name,true);const before=ui.doc()
 await ui.click('Prepare profile');expect(ui.all().some(n=>n.props['data-preview']==='prepared-profile')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Prepare profile');await commandKey(ui,'Enter')
 const committed=ui.doc();expect(committed.sketches).toHaveLength(1)
 expect(committed.sketches[0].retainedProfile!.areaMm2).toBeCloseTo(2*Math.PI,10)
 expect(committed.sketches[0].analytic).toBeUndefined()
 await ui.click('↶');expect(parseDirectDocument(JSON.stringify(ui.doc()))).toEqual(parseDirectDocument(JSON.stringify(before)))
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(parseDirectDocument(JSON.stringify(ui.doc()))).toEqual(parseDirectDocument(JSON.stringify(committed)))
})

it('subtracts profile regions with target roles, empty-result refusal, cancel, undo and repeat',async()=>{
 await geometryKernel.warmGeometryKernel()
 const plate={id:'plate',name:'Plate profile',closed:true,points:[[-4,-3],[4,-3],[4,3],[-4,3]]}
 const circle={id:'cut',name:'Round cutter',closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}}
 const ui=await mount({},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[plate,circle],bodies:[]}))))
 const normalized=()=>parseDirectDocument(JSON.stringify(ui.doc()))
 await ui.click(plate.name);await ui.click(circle.name,true);const before=normalized()
 await ui.click('Subtract profile regions');expect(ui.all().some(n=>n.props['data-preview']==='retained-profile')).toBe(true)
 const target=ui.all().find(n=>n.props['aria-label']==='Target profile')!
 target.props.onChange({target:{value:'cut'}});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Empty result')
 await commandKey(ui,'Enter');expect(normalized()).toEqual(before)
 target.props.onChange({target:{value:'plate'}});await flushClearance()
 await commandKey(ui,'Escape');expect(normalized()).toEqual(before)
 await ui.click('Subtract profile regions');await commandKey(ui,'Enter')
 const committed=normalized();expect(committed.sketches).toHaveLength(1);expect(committed.sketches[0].id).toBe('plate')
 expect(committed.sketches[0].retainedProfile!.loops).toHaveLength(2)
 expect(committed.sketches[0].retainedProfile!.areaMm2).toBeCloseTo(48-4*Math.PI,8)
 await ui.click('↶');expect(normalized()).toEqual(before)
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(normalized()).toEqual(committed)
})
it('keeps the common profile region with retained circular boundaries',async()=>{
 await geometryKernel.warmGeometryKernel()
 const plate={id:'plate',name:'Intersection plate',closed:true,points:[[-4,-3],[4,-3],[4,3],[-4,3]]}
 const circle={id:'cut',name:'Intersection disk',closed:true,points:[],analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}}
 const ui=await mount({},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[plate,circle],bodies:[]}))))
 await ui.click(plate.name);await ui.click(circle.name,true)
 await ui.click('Intersect exact profiles');await commandKey(ui,'Enter')
 expect(ui.doc().sketches).toHaveLength(1);expect(ui.doc().sketches[0].retainedProfile!.areaMm2).toBeCloseTo(4*Math.PI,8)
})

it('keeps the explicitly chosen subtraction target and restores its operand order on repeat',async()=>{
 await geometryKernel.warmGeometryKernel()
 const plate={id:'plate',name:'Partial plate',closed:true,points:[[-4,-3],[3,-3],[3,3],[-4,3]]}
 const circle={id:'cut',name:'Partial disk',closed:true,points:[],analytic:{kind:'circle',center:[3,0],radius:2,start:0,sweep:360}}
 const ui=await mount({},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[plate,circle],bodies:[]}))))
 const normalized=()=>parseDirectDocument(JSON.stringify(ui.doc()))
 await ui.click(plate.name);await ui.click(circle.name,true);const before=normalized()
 await ui.click('Subtract profile regions')
 ui.all().find(n=>n.props['aria-label']==='Target profile')!.props.onChange({target:{value:'cut'}});await flushClearance()
 await commandKey(ui,'Enter')
 const committed=normalized();expect(committed.sketches).toHaveLength(1);expect(committed.sketches[0].id).toBe('cut')
 expect(committed.sketches[0].retainedProfile!.areaMm2).toBeCloseTo(2*Math.PI,8)
 await ui.click('↶');expect(normalized()).toEqual(before)
 await ui.click('Repeat · Shift R');await commandKey(ui,'Enter');expect(normalized()).toEqual(committed)
})

it('previews every retained offset loop and supports units, refusal, cancel and undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {combineSketchProfiles}=await import('../src/services/retainedSketchProfile')
 const profile=combineSketchProfiles([{id:'plate',name:'Offset plate',closed:true,points:[[0,0],[8,0],[8,6],[0,6]]},{id:'hole',name:'Offset hole',closed:true,points:[],analytic:{kind:'circle',center:[4,3],radius:1,start:0,sweep:360}}],'difference')
 const ui=await mount({},JSON.stringify({version:1,sketches:[profile],bodies:[]}))
 const normalized=()=>parseDirectDocument(JSON.stringify(ui.doc()))
 await ui.click(profile.name);const before=normalized();await ui.click('Offset')
 const input=ui.all().find(n=>n.props['onUpdate:modelValue']&&n.props.step==='.5')!
 input.props['onUpdate:modelValue']('0.05 cm');await flushClearance()
 const preview=ui.all().find(n=>n.props['data-preview']==='offset-profile')!
 expect(preview.props.d.match(/M /g)).toHaveLength(2)
 input.props['onUpdate:modelValue']('-10 mm');await flushClearance();await commandKey(ui,'Enter');expect(normalized()).toEqual(before)
 expect(ui.text(ui.all()[0])).toContain('Offset removes the profile')
 input.props['onUpdate:modelValue']('0.5 mm');await flushClearance();await commandKey(ui,'Escape');expect(normalized()).toEqual(before)
 await ui.click('Offset');await commandKey(ui,'Enter')
 expect(ui.doc().sketches[0].retainedProfile!.areaMm2).toBeCloseTo(62,7)
 await ui.click('↶');expect(normalized()).toEqual(before)
})

it('ignores late extrusion success and failure after edits, cancellation and panel close',async()=>{
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 vi.useFakeTimers()
 try {
  await ui.click('Extrude · E');await vi.advanceTimersByTimeAsync(100)
  const input=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Dimension at manipulator, mm')!
  input.props['onUpdate:modelValue']('23 mm');await nextTick();await vi.advanceTimersByTimeAsync(100)
  expect(requests).toHaveLength(2)
  requests[1].resolve(applyDirectExtrusionProfile(requests[1].job.document,requests[1].job.options));await flushClearance()
  requests[0].reject(Error('obsolete geometry failure'));await flushClearance()
  expect(ui.text(ui.all()[0])).not.toContain('obsolete geometry failure')
  await ui.click('Apply · Enter')
  expect(inspectPolygonMesh(ui.doc().bodies.at(-1)!.mesh).signedVolumeMm3).toBeCloseTo(2300)
  expect(requests).toHaveLength(2) // Apply consumes the finished worker result.
  await ui.click('Profile');await ui.click('Extrude · E');await vi.advanceTimersByTimeAsync(100)
  await commandKey(ui,'Escape')
  requests[2].resolve(applyDirectExtrusionProfile(requests[2].job.document,requests[2].job.options));await flushClearance()
  expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
  const applied=ui.doc()
  await ui.click('Extrude · E');await vi.advanceTimersByTimeAsync(100)
  await ui.setProps({open:false})
  requests[3].reject(Error('closed panel failure'));await flushClearance()
  await ui.setProps({open:true});await nextTick()
  expect(ui.text(ui.all()[0])).not.toContain('closed panel failure')
  expect(ui.doc()).toEqual(applied);expect(ui.doc().bodies.length).toBe(before.bodies.length+1)
 } finally {vi.useRealTimers()}
})

it('rejects an old revolve preview after undo and reuses the latest completed result on Apply',async()=>{
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 vi.useFakeTimers()
 try {
  await ui.click('Revolve');await vi.advanceTimersByTimeAsync(100)
  await ui.click('↶')
  requests[0].resolve(applySolidRevolve(requests[0].job.document,requests[0].job.options));await flushClearance()
  expect(ui.doc()).toEqual(before)
  expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
  await ui.click('Revolve');await vi.advanceTimersByTimeAsync(100)
  requests[1].resolve(applySolidRevolve(requests[1].job.document,requests[1].job.options));await flushClearance()
  await ui.click('Apply · Enter')
  expect(requests).toHaveLength(2)
  expect(ui.doc().bodies.at(-1)!.id).toBe(requests[1].job.options.id)
  expect(ui.doc().bodies.at(-1)!.brep).toBeDefined()
 }finally{vi.useRealTimers()}
})

it('keeps only the newest body edit and ignores a result after Escape',async()=>{
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('5 mm');await flushClearance()
 expect(requests).toHaveLength(2)
 requests[1].resolve(applySolidBodyEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 requests[0].reject(Error('obsolete split failure'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('obsolete split failure')
 await ui.click('Apply · Enter');expect(requests).toHaveLength(2)
 const result=ui.doc();expect(result.bodies).toHaveLength(2)
 expect(result.bodies.map(b=>inspectPolygonMesh(b.mesh).signedVolumeMm3)).toEqual([500,500])
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Cube');await ui.click('Split');await commandKey(ui,'Escape')
 requests[2].resolve(applySolidBodyEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
})

it.each(['success','failure'] as const)('isolates a reopened body command from late %s of the closed panel',async outcome=>{
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('5 mm');await flushClearance()
 const obsolete=[...requests]
 await ui.setProps({open:false});await ui.setProps({open:true});await flushClearance()
 expect(requests).toHaveLength(obsolete.length)
 expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
 expect(ui.doc()).toEqual(before)
 await ui.click('Split');await flushClearance()
 const current=requests.at(-1)!
 expect(obsolete).not.toContain(current)
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 for(const request of obsolete){
  if(outcome==='success')request.resolve(applySolidBodyEdit(request.job.document,request.job.options))
  else request.reject(Error('closed panel split failure'))
 }
 await flushClearance()
 expect(ui.doc()).toEqual(before)
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.text(ui.all()[0])).not.toContain('closed panel split failure')
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 current.resolve(applySolidBodyEdit(current.job.document,current.job.options));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await commandKey(ui,'Enter');const after=ui.doc();expect(after.bodies).toHaveLength(2)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(after)
})

it.each([false,true])('waits for the released Push/Pull handle result; cancel=%s',async cancel=>{
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Cube');await ui.click('Faces')
 const svg=ui.svg();await ui.pointer(ui.all(svg).find(n=>n.tag==='polygon')!)
 const before=ui.doc(),handle=ui.all(svg).find(n=>n.tag==='g'&&n.props.style?.cursor==='ns-resize'&&n.props.onPointerdown)!
 expect(handle).toBeDefined();await ui.pointer(handle,0,0)
 svg.props.onPointermove({...ui.event(svg,0,1),altKey:true});await flushClearance()
 svg.props.onPointerup({...ui.event(svg,0,1),altKey:true});await flushClearance()
 expect(ui.doc()).toEqual(before)
 if(cancel)await commandKey(ui,'Escape')
 const request=requests.at(-1)!
 expect(request.job.options.distance).not.toBe(0)
 request.resolve(applySolidBodyEdit(request.job.document,request.job.options));await flushClearance()
 if(cancel)expect(ui.doc()).toEqual(before)
 else {expect(ui.doc()).not.toEqual(before);await ui.click('↶');expect(ui.doc()).toEqual(before)}
})

it('invalidates a pending profile offset on parameter changes and panel close',async()=>{
 const {applySolidProfileEdit}=await import('../src/services/solidProfileEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click('Offset')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('1 mm');await flushClearance()
 expect(requests).toHaveLength(2)
 requests[1].resolve(applySolidProfileEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 requests[0].reject(Error('old profile error'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('old profile error')
 await ui.click('Apply · Enter');expect(requests).toHaveLength(2);expect(ui.doc().sketches[0].id).toBe('s')
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Profile');await ui.click('Offset');await ui.setProps({open:false})
 requests[2].resolve(applySolidProfileEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
})

it('does not show or commit the old Boolean preview after swapping target roles',async()=>{
 const {applySolidProfileEdit}=await import('../src/services/solidProfileEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const document={version:1,sketches:[{id:'plate',name:'Plate',closed:true,points:[[0,0],[8,0],[8,6],[0,6]]},{id:'hole',name:'Hole',closed:true,points:[[2,2],[3,2],[3,3],[2,3]]}],bodies:[]}
 const ui=await mount({},JSON.stringify(document));await ui.click('Plate');await ui.click('Hole',true);const before=ui.doc()
 await ui.click('Subtract profile regions')
 ui.all().find(n=>n.props['aria-label']==='Target profile')!.props.onChange({target:{value:'hole'}});await flushClearance()
 expect(requests).toHaveLength(2);expect(requests[1].job.options.inputs).toEqual(['hole','plate'])
 requests[1].reject(Error('Profile contains no material.'));await flushClearance()
 requests[0].resolve(applySolidProfileEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.text(ui.all()[0])).toContain('Empty result')
 expect(ui.all().some(n=>n.props['data-preview']==='retained-profile')).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
})

it('discards old profile preparation diagnostics after changing tolerance',async()=>{
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const document={version:1,sketches:[{id:'a',name:'A',closed:false,points:[[0,0],[10,0],[10,10]]},{id:'b',name:'B',closed:false,points:[[10,10],[0,10],[0,.005]]}],bodies:[]}
 const ui=await mount({},JSON.stringify(document));await ui.click('A');await ui.click('B',true);const before=ui.doc()
 await ui.click('Prepare profile')
 const field=quantityField(ui,'Gap tolerance, mm')
 field.props['onUpdate:modelValue']('.001 mm');await flushClearance()
 field.props['onUpdate:modelValue']('.01 mm');await flushClearance()
 const current=requests.at(-1)!,old=requests.at(-2)!
 current.resolve(prepareSolidProfile(current.job.document,current.job.ids,current.job.tolerance));await flushClearance()
 old.resolve(prepareSolidProfile(old.job.document,old.job.ids,old.job.tolerance));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 expect(ui.all().some(n=>n.props['data-preview']==='prepared-profile')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 requests[0].resolve(prepareSolidProfile(requests[0].job.document,requests[0].job.ids,requests[0].job.tolerance));await flushClearance()
 expect(ui.all().some(n=>n.props['data-preview']==='prepared-profile')).toBe(false)
})

it('ignores an old refit certificate after parameter changes and Escape',async()=>{
 const {refitSolidNurbs}=await import('../src/services/solidCurveReduction')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const curve=createSolidNurbsCurve('refit');curve.name='Refit curve';curve.curve.controlPoints=[[0,0,0],[1,0,0],[2,0,0],[3,0,0]]
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[curve]}));await ui.click(curve.name);const before=ui.doc()
 await ui.click('Rebuild curve');expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 quantityField(ui,'Control points').props['onUpdate:modelValue'](8);await flushClearance()
 requests[1].resolve(refitSolidNurbs(requests[1].job.document,requests[1].job.options));await flushClearance()
 requests[0].resolve(refitSolidNurbs(requests[0].job.document,requests[0].job.options));await flushClearance()
 await commandKey(ui,'Enter');expect(ui.doc().curves![0].curve.controlPoints).toHaveLength(8)
 expect(requests).toHaveLength(2);await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click(curve.name);await ui.click('Rebuild curve');await commandKey(ui,'Escape')
 requests[2].resolve(refitSolidNurbs(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='curve-reduction')).toBe(false)
})

it('discards a surface construction after input reversal and cancellation',async()=>{
 const {buildSolidSurface}=await import('../src/services/solidSurfaceConstruction')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='Section A';b.name='Section B';b.curve.controlPoints=b.curve.controlPoints.map(([x,y,z])=>[x,y,z+10])
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[a,b]}));await ui.click(a.name);await ui.click(b.name,true);const before=ui.doc()
 await ui.click('NURBS loft');expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await ui.click('Reverse input order');expect(requests).toHaveLength(2)
 const latest=buildSolidSurface(requests[1].job.document,requests[1].job.options)
 requests[1].resolve(latest);await flushClearance()
 requests[0].resolve(buildSolidSurface(requests[0].job.document,requests[0].job.options));await flushClearance()
 await commandKey(ui,'Enter');expect(ui.doc().surfaces![0].surface).toEqual(latest.document!.surfaces![0].surface)
 expect(requests).toHaveLength(2);await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('NURBS loft');await commandKey(ui,'Escape')
 requests[2].resolve(buildSolidSurface(requests[2].job.document,requests[2].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
},60_000)

it.each(['summary','select'])('keeps native Enter on %s while a command is active',async tag=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Split');const before=ui.doc()
 const summary={closest:(selector:string)=>selector.split(',').map(s=>s.trim()).includes(tag)?{}:null}
 expect(await commandKey(ui,'Enter',summary as unknown as Node)).not.toHaveBeenCalled()
 expect(ui.doc()).toEqual(before)
})

it('ignores stale curve matching after changing endpoint roles and cancelling',async()=>{
 const {matchSolidCurve}=await import('../src/services/solidCurveMatching')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='Match A';b.name='Match B';b.curve.controlPoints.forEach(p=>p[0]+=50)
 const ui=await mount({},JSON.stringify({version:1,sketches:[],bodies:[],curves:[a,b]}));await ui.click(a.name);await ui.click(b.name,true);const before=ui.doc()
 await ui.click('Match curves G1')
 ui.all().find(n=>n.props['aria-label']==='Endpoint A')!.props['onUpdate:modelValue']('start');await flushClearance()
 const latest=matchSolidCurve(...requests[1].job.args)
 requests[1].resolve(latest);await flushClearance()
 requests[0].resolve(matchSolidCurve(...requests[0].job.args));await flushClearance()
 await commandKey(ui,'Enter');expect(ui.doc().curves![1].curve).toEqual(latest.document.curves![1].curve)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Match curves G1');await commandKey(ui,'Escape')
 requests[2].resolve(matchSolidCurve(...requests[2].job.args));await flushClearance();expect(ui.doc()).toEqual(before)
})

it('keeps the latest transform preview and drops an instance response after Escape',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Transform selection')
 quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance()
 requests[1].resolve(applySolidSceneEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 await commandKey(ui,'Enter');expect(requests).toHaveLength(2)
 expect(Math.min(...ui.doc().bodies[0].mesh.positions.filter((_,i)=>i%3===0))).toBeCloseTo(5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Create linked instance');await commandKey(ui,'Escape')
 requests[2].resolve(applySolidSceneEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
})

it('discards Boolean replies after Escape, operand changes, document edits and panel close',async()=>{
 const {applySolidBoolean}=await import('../src/services/solidBoolean')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='boolean'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const brep=createBrepCylinder(3,5),built=tessellateNurbsBrep(brep,4)
 const body=(id:string)=>({id,name:id,brep:structuredClone(brep),mesh:{positions:built.positions,indices:built.indices}})
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[body('Stock'),body('Cutter')]}})
 const before=ui.doc()
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep Union')
 expect(ui.text(ui.all()[0])).toContain('Computing Boolean')
 await commandKey(ui,'Escape');requests[0].resolve(applySolidBoolean(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
 await ui.click('B-rep A − B');await ui.click('OK');expect(ui.button('OK').props.disabled).toBe(true)
 ui.all().find(n=>n.props.class==='subtract-field')!.props.onClick();await flushClearance();await ui.click('Cutter')
 requests[1].reject(Error('obsolete kernel refusal'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('obsolete kernel refusal');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep Union')
 await ui.click('Box');const changed=ui.doc()
 requests[2].resolve(applySolidBoolean(requests[2].job.document,requests[2].job.options));await flushClearance();expect(ui.doc()).toEqual(changed)
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep Union')
 await ui.setProps({open:false});await flushClearance();requests[3].resolve(applySolidBoolean(requests[3].job.document,requests[3].job.options));await flushClearance()
 expect(ui.doc()).toEqual(changed)
})

it('uses the latest corner candidate and ignores circular copies after Escape',async()=>{
 const {applySolidSketchEdit}=await import('../src/services/solidSketchEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sketchEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click('Fillet')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 quantityField(ui,'Radius, mm').props['onUpdate:modelValue']('3');await flushClearance()
 requests[1].resolve(applySolidSketchEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 const candidate=applySolidSketchEdit(requests[1].job.document,requests[1].job.options)
 requests[0].resolve(applySolidSketchEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 await ui.click('Apply · Enter');expect(requests).toHaveLength(2);expect(ui.doc().sketches).toEqual(candidate.sketches)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Circular copies');await commandKey(ui,'Escape')
 requests[2].resolve(applySolidSketchEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
})

it('applies one circular-copy candidate and exposes invalid array parameters',async()=>{
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click('Circular copies')
 const calls=previewWorkerRun.mock.calls.length
 await ui.click('Apply · Enter');expect(previewWorkerRun.mock.calls.length).toBe(calls)
 expect(ui.doc().sketches).toHaveLength(before.sketches.length+7)
 expect(new Set(ui.doc().sketches.map(s=>s.id)).size).toBe(ui.doc().sketches.length)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Circular copies')
 const count=ui.all().find(n=>n.tag==='input'&&String(n.props.min)==='2'&&String(n.props.max)==='64')!
 count.props['onUpdate:modelValue'](65);await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.text(ui.all()[0])).toContain('Use 2–64 instances')
})

it.each(['move','rotate','scale'])('does not publish a stationary %s gizmo click',async(kind)=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount(),baseline=ui.doc();await ui.click('Box');await ui.click('Gizmo: '+kind)
 const before=ui.doc(),svg=ui.svg(),commit=vi.spyOn(DirectHistory.prototype,'commitAsync')
 try{
  const handle=ui.all(svg).find(n=>kind==='rotate'?n.tag==='polyline'&&n.props.onPointerdown&&n.props.stroke==='#ff7777':n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
  await ui.pointer(handle,0,0);svg.props.onPointermove({...ui.event(svg,0,0),altKey:true});await flushClearance();svg.props.onPointerup({...ui.event(svg,0,0),altKey:true});await flushClearance()
  expect(commit).not.toHaveBeenCalled();expect(ui.doc()).toEqual(before)
  await ui.click('↶');expect(ui.doc()).toEqual(baseline)
 }finally{commit.mockRestore()}
})

it('supersedes a pending gizmo history publication with the next gesture',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount();await ui.click('Cube');const before=ui.doc(),svg=ui.svg()
 const original=DirectHistory.prototype.commitAsync
 let release!:()=>void,calls=0
 const gate=new Promise<void>(resolve=>{release=resolve})
 const commit=vi.spyOn(DirectHistory.prototype,'commitAsync').mockImplementation(function(load,validate){
  const hold=++calls===1
  return original.call(this,async()=>{const result=await load();if(hold)await gate;return result},validate)
 })
 async function drag(x:number){
  const handle=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
  await ui.pointer(handle,0,0);svg.props.onPointermove({...ui.event(svg,x,0),altKey:true})
  svg.props.onPointerup({...ui.event(svg,x,0),altKey:true});await flushClearance()
 }
 try{
  await drag(20);expect(calls).toBe(1)
  await drag(40);release();await flushClearance()
  expect(calls).toBe(2);expect(ui.doc()).not.toEqual(before)
  await ui.click('↶');expect(ui.doc()).toEqual(before)
 }finally{release();commit.mockRestore()}
})

it('cancels gizmo history publication after the final worker response',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount(),baseline=ui.doc();await ui.click('Box');const before=ui.doc(),svg=ui.svg()
 const original=DirectHistory.prototype.commitAsync
 let release!:()=>void,loaded=false
 const gate=new Promise<void>(resolve=>{release=resolve})
 const commit=vi.spyOn(DirectHistory.prototype,'commitAsync').mockImplementation(function(load,validate){
  return original.call(this,async()=>{const result=await load();loaded=true;await gate;return result},validate)
 })
 try{
  const handle=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
  await ui.pointer(handle,0,0)
  svg.props.onPointermove({...ui.event(svg,20,0),altKey:true})
  svg.props.onPointerup({...ui.event(svg,20,0),altKey:true});await flushClearance()
  expect(loaded).toBe(true);await commandKey(ui,'Escape');release();await flushClearance()
  expect(ui.doc()).toEqual(before)
  await ui.click('↶');expect(ui.doc()).toEqual(baseline)
 }finally{release();commit.mockRestore()}
})

it('waits for the released translation, discards a canceled result and commits the latest drag once',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Cube');const before=ui.doc(),svg=ui.svg()
 const handle=()=>ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 async function release(x:number){await ui.pointer(handle(),0,0);svg.props.onPointermove({...ui.event(svg,x,0),altKey:true});svg.props.onPointerup({...ui.event(svg,x,0),altKey:true});await flushClearance()}
 await release(20);expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).toContain('Computing transform')
 await commandKey(ui,'Escape');await release(30)
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 const latest=applySolidSceneEdit(requests[1].job.document,requests[1].job.options)
 requests[1].resolve(latest);await flushClearance();expect(ui.doc().bodies).toEqual(JSON.parse(stringifyMeshJson(latest)).bodies)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each(['Escape','invalid-input','selection-change'])('cancels numeric history publication after worker completion: %s',async(action)=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount(),baseline=ui.doc();await ui.click('Box');await ui.click('Profile');const before=ui.doc();await ui.click('Properties')
 const input=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='ΔX')!
 input().props['onUpdate:modelValue']('5 mm');await flushClearance()
 const original=DirectHistory.prototype.commitAsync
 let release!:()=>void,loaded=false
 const gate=new Promise<void>(resolve=>{release=resolve})
 const commit=vi.spyOn(DirectHistory.prototype,'commitAsync').mockImplementation(function(load,validate){
  return original.call(this,async()=>{const result=await load();loaded=true;await gate;return result},validate)
 })
 try{
  await ui.click('Apply');expect(loaded).toBe(true)
  if(action==='Escape')await commandKey(ui,'Escape')
  else if(action==='invalid-input'){input().props['onUpdate:modelValue']('bad');await flushClearance()}
  else{await ui.click('Scene');await ui.click('Cube')}
  release();await flushClearance();expect(ui.doc()).toEqual(before)
  await ui.click('↶');expect(ui.doc()).toEqual(baseline)
 }finally{release();commit.mockRestore()}
})

it('publishes the validated transform response without another history geometry clone',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 let request!:{job:any;resolve:(value:any)=>void}
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>{request={job,resolve}}):undefined)
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Properties')
 ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='ΔX')!.props['onUpdate:modelValue']('5 mm')
 await flushClearance();await ui.click('Apply')
 const result=applySolidSceneEdit(request.job.document,request.job.options)
 const read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  request.resolve(result);await flushClearance()
  expect(read).not.toHaveBeenCalled()
  expect(ui.doc().bodies[0].mesh.positions).not.toEqual(before.bodies[0].mesh.positions)
 }finally{read.mockRestore()}
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('accepts units in numeric transforms and cancels pending work on malformed input',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click('Properties')
 const field=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='ΔX')!
 field().props['onUpdate:modelValue']('2 cm');await flushClearance();await ui.click('Apply')
 expect(requests[0].job.options.x).toBe(20)
 field().props['onUpdate:modelValue']('bad');await flushClearance()
 expect(ui.button('Apply').props.disabled).toBe(true);expect(field().props['aria-invalid']).toBe(true)
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
 field().props['onUpdate:modelValue']('1 in');await flushClearance();await ui.click('Apply')
 expect(requests[1].job.options.x).toBe(25.4)
 requests[1].resolve(applySolidSceneEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().sketches[0].points[0]).toEqual([25.4,0]);await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each([['ΔX','1 rad'],['ΔY','NaN'],['ΔZ','Infinity'],['Z rotation','2 cm'],['Scale','0'],['Scale','-1']])('refuses invalid numeric transform %s = %s without a worker request',async(label,value)=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Properties')
 const field=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']===label)!
 field().props['onUpdate:modelValue'](value);await flushClearance()
 expect(field().props['aria-invalid']).toBe(true)
 expect(field().props['aria-errormessage']).toBeTruthy()
 expect(ui.button('Apply').props.disabled).toBe(true)
 const count=previewWorkerRun.mock.calls.filter(([job])=>job.kind==='sceneEdit').length
 ui.button('Apply').props.onClick();await flushClearance()
 expect(previewWorkerRun.mock.calls.filter(([job])=>job.kind==='sceneEdit')).toHaveLength(count)
 expect(ui.doc()).toEqual(before)
 field().props['onUpdate:modelValue'](label==='Scale'?'1':'0');await flushClearance()
 expect(ui.button('Apply').props.disabled).toBe(false)
})

it('invalidates a pending numeric transform when its input changes',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click('Properties')
 const input=ui.all().find(n=>n.props['aria-label']==='ΔX')!
 input.props['onUpdate:modelValue'](5);await flushClearance();await ui.click('Apply')
 input.props['onUpdate:modelValue'](7);await flushClearance()
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Apply');requests[1].resolve(applySolidSceneEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().sketches[0].points[0]).toEqual([7,0]);await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each(['rotate','scale'])('coalesces %s drag requests and commits only the last released result',async(kind)=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Cube');await ui.click('Gizmo: '+kind);const before=ui.doc(),svg=ui.svg()
 const handle=()=>ui.all(svg).find(n=>kind==='rotate'?n.tag==='polyline'&&n.props.onPointerdown&&n.props.stroke==='#ff7777':n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 await ui.pointer(handle(),0,0)
 for(const x of [10,20,30])svg.props.onPointermove({...ui.event(svg,x,20),altKey:true})
 svg.props.onPointerup({...ui.event(svg,40,25),altKey:true});svg.props.onLostpointercapture(ui.event(svg,40,25));await flushClearance()
 expect(requests).toHaveLength(1);expect(ui.doc()).toEqual(before)
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(requests).toHaveLength(2);expect(ui.doc()).toEqual(before)
 const expected=applySolidSceneEdit(requests[1].job.document,requests[1].job.options)
 requests[1].resolve(expected);await flushClearance()
 expect(ui.doc().bodies).toEqual(JSON.parse(stringifyMeshJson(expected)).bodies)
 expect(ui.doc().bodies).not.toEqual(before.bodies)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 const rendered=()=>ui.all(svg).filter(n=>n.tag==='polygon'&&n.props['data-body']==='b').map(n=>n.props.points)
 const originalRendered=rendered();expect(originalRendered.length).toBeGreaterThan(0)
 // A displayed preview and an in-flight replacement both disappear on Escape.
 await ui.pointer(handle(),0,0);svg.props.onPointermove({...ui.event(svg,15,20),altKey:true});await flushClearance()
 requests[2].resolve(applySolidSceneEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(rendered()).not.toEqual(originalRendered);expect(ui.doc()).toEqual(before)
 svg.props.onPointermove({...ui.event(svg,25,20),altKey:true});await flushClearance();await commandKey(ui,'Escape')
 requests[3].resolve(applySolidSceneEdit(requests[3].job.document,requests[3].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
 expect(rendered()).toEqual(originalRendered)
 expect(ui.text(ui.all()[0])).not.toContain('Computing transform')
})

it('queues CV drags, keeps the latest release, and discards a late CV after Escape',async()=>{
 const {applySolidPointEdit}=await import('../src/services/solidPointEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='pointEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('+ NURBS surface');const before=ui.doc(),svg=ui.svg()
 const cv=()=>ui.all(svg).find(n=>n.tag==='circle'&&n.props.onPointerdown)!
 await ui.pointer(cv(),0,0)
 svg.props.onPointermove({...ui.event(svg,10,5),altKey:true});svg.props.onPointermove({...ui.event(svg,15,8),altKey:true});svg.props.onPointerup({...ui.event(svg,20,10),altKey:true});svg.props.onLostpointercapture(ui.event(svg,20,10));await flushClearance()
 expect(requests).toHaveLength(1);expect(ui.doc()).toEqual(before)
 requests[0].resolve(applySolidPointEdit(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(requests).toHaveLength(2);expect(ui.doc()).toEqual(before)
 const expected=applySolidPointEdit(requests[1].job.document,requests[1].job.options)
 requests[1].resolve(expected);await flushClearance();expect(ui.doc().surfaces).toEqual(expected.surfaces)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.pointer(cv(),0,0);svg.props.onPointermove({...ui.event(svg,10,5),altKey:true});await commandKey(ui,'Escape')
 requests[2].resolve(applySolidPointEdit(requests[2].job.document,requests[2].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.pointer(cv(),0,0);svg.props.onPointermove({...ui.event(svg,12,6),altKey:true});await flushClearance()
 const panel=ui.all().find(n=>String(n.props.class).includes('nurbs-card'))!
 ui.all(panel).find(n=>n.tag==='select')!.props['onUpdate:modelValue'](1);await flushClearance()
 requests[3].resolve(applySolidPointEdit(requests[3].job.document,requests[3].job.options));await flushClearance();expect(ui.doc()).toEqual(before)

})

it('keeps the CV panel available and invalidates a numeric edit when coordinates change',async()=>{
 const {applySolidPointEdit}=await import('../src/services/solidPointEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='pointEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('+ NURBS surface');const before=ui.doc()
 const card=()=>ui.all().find(n=>String(n.props.class).includes('nurbs-card'))!
 const input=ui.all(card()).find(n=>n.tag==='label'&&ui.text(n).trim()==='X')!.children.find(n=>n.tag==='input')!
 input.props['onUpdate:modelValue'](5);await flushClearance();await ui.click('Apply CV')
 expect(card()).toBeDefined();expect(ui.button('Apply CV').props.disabled).toBe(true)
 input.props['onUpdate:modelValue'](7);await flushClearance();await ui.click('Apply CV')
 requests[0].resolve(applySolidPointEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 requests[1].resolve(applySolidPointEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().surfaces![0].surface.controlPoints[0][0][0]).toBe(7)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('commits a mixed sketch/body drag once and ignores a canceled analytic handle reply',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const {applySolidPointEdit}=await import('../src/services/solidPointEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Profile');await ui.click('Cube',true);const before=ui.doc()
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 const path=ui.all(svg).find(n=>n.tag==='path'&&n.props.d==='M 0,0 L 10,0 L 10,-10 L 0,-10 Z')!
 await ui.pointer(path,0,0);svg.props.onPointermove({...ui.event(svg,10,5),altKey:true});svg.props.onPointerup({...ui.event(svg,20,10),altKey:true});svg.props.onLostpointercapture(ui.event(svg,20,10));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(requests).toHaveLength(1)
 requests[0].resolve(applySolidSceneEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(requests).toHaveLength(2)
 requests[1].resolve(applySolidSceneEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().sketches[0].points[0]).toEqual([20,-10])
 expect(ui.doc().bodies[0].mesh.positions[0]).toBeCloseTo(before.bodies[0].mesh.positions[0]+20)
 expect(ui.doc().bodies[0].mesh.positions[1]).toBeCloseTo(before.bodies[0].mesh.positions[1]-10)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Circle');const radius=ui.all(svg).find(n=>n.tag==='circle'&&n.props.style?.cursor==='ew-resize')!
 await ui.pointer(radius,23,-5);svg.props.onPointermove({...ui.event(svg,28,-5),altKey:true});await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(applySolidPointEdit(requests[2].job.document,requests[2].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
})

it('cancels native NURBS commands on parameter changes and Esc, then commits one undoable result',async()=>{
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('+ NURBS curve');const before=ui.doc()
 await ui.click('Insert knot');expect(ui.button('Insert knot').props['aria-disabled']).toBe(true)
 expect(ui.button('Insert knot').props.disabled).not.toBe(true)
 await ui.click('Insert knot');expect(requests).toHaveLength(1)
 const parameter=ui.all().find(n=>n.tag==='label'&&ui.text(n).includes('Knot / iso parameter'))!.children.find(n=>n.tag==='input')!
 parameter.props['onUpdate:modelValue'](.3);await flushClearance()
 requests[0].resolve(applySolidNurbsEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Degree +1');await commandKey(ui,'Escape')
 requests[1].resolve(applySolidNurbsEdit(requests[1].job.document,requests[1].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Insert knot');requests[2].resolve(applySolidNurbsEdit(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc().curves![0].curve.knots).toContain(.3)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('ignores native NURBS replies after selection, document replacement and closure',async()=>{
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount();await ui.click('+ NURBS curve');const before=ui.doc()
 await ui.click('Insert knot');await ui.click('Cube');requests[0].resolve(applySolidNurbsEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 ui.all().find(n=>n.tag==='button'&&ui.text(n)==='NURBS curve')!.props.onClick({shiftKey:false});await flushClearance();await ui.click('Insert knot');await ui.click('Box');const changed=ui.doc()
 requests[1].reject(Error('obsolete NURBS error'));await flushClearance();expect(ui.doc()).toEqual(changed);expect(ui.text(ui.all()[0])).not.toContain('obsolete NURBS error')
 ui.all().find(n=>n.tag==='button'&&ui.text(n)==='NURBS curve')!.props.onClick({shiftKey:false});await flushClearance();await ui.click('Degree +1');await ui.setProps({open:false})
 requests[2].resolve(applySolidNurbsEdit(requests[2].job.document,requests[2].job.options));await flushClearance();expect(ui.doc()).toEqual(changed)
})

it.each(['success','failure'] as const)('discards native bridge %s after changing the destination group',async outcome=>{
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='First';b.name='Second';b.curve.controlPoints.forEach(p=>p[0]+=60)
 const ui=await mount({seedDocument:{version:1,sketches:[],bodies:[],curves:[a,b],groups:[{name:'Destination',source:''}]}})
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 await ui.click('First');await ui.click('Second',true);const before=ui.doc();await ui.click('Create G2 bridge')
 expect(requests).toHaveLength(1)
 await ui.click('Active group: Destination')
 if(outcome==='success')requests[0].resolve(applySolidNurbsEdit(requests[0].job.document,requests[0].job.options))
 else requests[0].reject(Error('obsolete group bridge error'))
 await flushClearance();expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).not.toContain('obsolete group bridge error')
 await ui.click('Create G2 bridge');expect(requests).toHaveLength(2)
 requests[1].resolve(applySolidNurbsEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().curves).toHaveLength(3);expect(ui.doc().curves![2].group).toBe('Destination')
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('invalidates bridge creation when tension changes and saves one undoable dependent curve',async()=>{
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');a.name='First';b.name='Second';b.curve.controlPoints.forEach(p=>p[0]+=60)
 const ui=await mount({seedDocument:{version:1,sketches:[],bodies:[],curves:[a,b]}})
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 await ui.click('First');await ui.click('Second',true);const before=ui.doc();await ui.click('Create G2 bridge')
 const tension=ui.all().find(n=>n.tag==='label'&&ui.text(n)==='Tension')!.children.find(n=>n.tag==='input')!
 for(const value of ['',0,-1,NaN,Infinity,10.1]){
  tension.props['onUpdate:modelValue'](value);await flushClearance()
  await ui.click('Create G2 bridge');expect(requests).toHaveLength(1)
  expect(ui.text(ui.all()[0])).toContain('Enter tension from 0.01 to 10.')
  expect(ui.doc()).toEqual(before)
 }
 tension.props['onUpdate:modelValue'](.5);await flushClearance()
 requests[0].resolve(applySolidNurbsEdit(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Create G2 bridge');requests[1].resolve(applySolidNurbsEdit(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().curves).toHaveLength(3);expect(ui.doc().curves![2].bridge!.tension).toBe(.5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('discards stale B-rep measurements and canceled detail changes',async()=>{
 const {applySolidBrepTool}=await import('../src/services/solidBrepTool')
 const ui=await mount({seedDocument:cylinderSeed()});await ui.click('Imported cylinder');await ui.click('B-rep detail');const before=ui.doc()
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='brepTool'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 await ui.click('B-rep properties');await commandKey(ui,'Escape')
 requests[0].resolve(applySolidBrepTool(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('mm³ · A =')
 await ui.click('B-rep detail');await ui.click('Retessellate');const field=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('B-rep detail'))!
 field.props['onUpdate:modelValue'](8);await flushClearance()
 requests[1].resolve(applySolidBrepTool(requests[1].job.document,requests[1].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('B-rep properties');requests[2].resolve(applySolidBrepTool(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('mm³ · A =');expect(ui.doc()).toEqual(before)
})

it('ignores a section from an older plane and a diagnostic reply after Escape',async()=>{
 const {applySolidBrepTool}=await import('../src/services/solidBrepTool')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='brepTool'&&job.options.kind==='display'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Cube');await ui.click('Body diagnostics')
 expect(ui.text(ui.all()[0])).toContain('Computing section')
 const normal=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Normal X')!
 normal.props['onUpdate:modelValue'](1);await flushClearance()
 requests[0].resolve(applySolidBrepTool(requests[0].job.document,requests[0].job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(false)
 requests[1].resolve(applySolidBrepTool(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
 normal.props['onUpdate:modelValue'](2);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(applySolidBrepTool(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(false)
})

it('detects another tab and loads its version while retaining local edits in Undo',async()=>{
 const ui=await mount();await ui.click('Box');const local=ui.doc()
 const external=structuredClone(local);external.bodies[0].name='Other tab body'
 localStorage.setItem('scad-solid-modeler-v1',stringifyMeshJson(external));ui.storageChanged();await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Document changed in another tab')
 await ui.click('Box');expect(ui.doc()).toEqual(external)
 await ui.click('Load saved version · keep local in Undo');expect(ui.doc()).toEqual(external)
 await ui.click('↶');expect(ui.doc().bodies).toHaveLength(local.bodies.length+1)
 expect(ui.doc().bodies[0].name).toBe(local.bodies[0].name)
})

it('retains the last saved draft if safe locking is unavailable and permits retry',async()=>{
 const ui=await mount(),saved=ui.serialized()
 vi.stubGlobal('navigator',{})
 await ui.click('Box');await flushClearance()
 expect(ui.serialized()).toBe(saved);expect(ui.text(ui.all()[0])).toContain('Safe draft locking is unavailable')
 vi.stubGlobal('navigator',{locks:{request:(_name:string,_options:unknown,action:()=>unknown)=>Promise.resolve(action())}})
 await ui.click('Retry saving draft');expect(ui.doc().bodies).toHaveLength(JSON.parse(saved).bodies.length+1)
})

it('does not label corrupt or missing saved geometry as successfully restored',async()=>{
 const corrupt=await mount({},'{broken document')
 await vi.waitFor(()=>expect(corrupt.text(corrupt.all()[0])).toContain('Saved JSON is corrupt'))
 expect(corrupt.all().some(n=>n.props['aria-label']==='Unsaved')).toBe(true)
 expect(corrupt.serialized()).toBe('{broken document')
 vi.stubGlobal('indexedDB',new IDBFactory())
 const marker=JSON.stringify({version:1,solidDraftId:'missing-snapshot'})
 const missing=await mount({},marker)
 await vi.waitFor(()=>expect(missing.text(missing.all()[0])).toContain('Saved draft snapshot is missing'))
 expect(missing.all().some(n=>n.props['aria-label']==='Unsaved')).toBe(true)
 expect(missing.serialized()).toBe(marker)
})

it('keeps history intact on cancelled or failed worker restore and ignores late replies after an edit',async()=>{
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount();await ui.click('Box');const before=ui.doc()
 await ui.click('↶');expect(ui.button('↶').props.disabled).toBe(true)
 expect(ui.text(ui.all()[0])).toContain('Restoring history')
 await commandKey(ui,'Escape');requests[0].resolve(parseDirectDocument(requests[0].job.text));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.button('↶').props.disabled).toBe(false)
 await ui.click('↶');requests[1].reject(Error('restore failed'));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).toContain('restore failed')
 await ui.click('↶');await ui.click('Box');const changed=ui.doc()
 requests[2].resolve(parseDirectDocument(requests[2].job.text));await flushClearance();expect(ui.doc()).toEqual(changed)
 await ui.click('↶');requests[3].resolve(parseDirectDocument(requests[3].job.text));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.button('↷').props.disabled).toBe(false)
})

it('uses the restored worker response for display without another history document clone',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount();await ui.click('Box');const before=ui.doc()
 const read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  await ui.click('↶');await flushClearance()
  expect(read).not.toHaveBeenCalled()
  await ui.click('↷');await flushClearance()
  expect(read).not.toHaveBeenCalled();expect(ui.doc()).toEqual(before)
 }finally{read.mockRestore()}
})

it.each([false,true])('pans without copying geometry or creating history (cancel=%s)',async cancel=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount(),svg=ui.svg(),before=ui.doc(),view=svg.props.viewBox
 const read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  svg.props.onPointerdown({...ui.event(svg,100,100),button:1});await nextTick()
  svg.props.onPointermove({...ui.event(svg,140,120),button:1});await nextTick()
  expect(svg.props.viewBox).not.toEqual(view)
  if(cancel)await commandKey(ui,'Escape')
  else {svg.props.onPointerup({...ui.event(svg,140,120),button:1});await flushClearance()}
  expect(read).not.toHaveBeenCalled();expect(ui.doc()).toEqual(before)
  expect(ui.button('↶').props.disabled).toBe(true)
 }finally{read.mockRestore()}
})

it('imports compact instances through history while preserving cache validation and locks',async()=>{
 const {serializeDirectDocument}=await import('../src/services/directModeling')
 const ui=await mount();await ui.click('Cube');await ui.click('Create linked instance');await commandKey(ui,'Enter')
 const before=ui.doc(),input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 async function load(doc:unknown){const text=JSON.stringify(doc);await input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'project.json'}});await flushClearance()}
 const compact=JSON.parse(serializeDirectDocument(before)),instance=before.bodies.find(b=>b.instance)!
 await load(compact);expect(ui.doc().bodies).toEqual(before.bodies)
 const stable=ui.doc()
 const invalid=structuredClone(compact);invalid.bodies.find((b:any)=>b.instance).brep={}
 await load(invalid);expect(ui.doc()).toEqual(stable)
 await load({...compact,blenderProjectId:null});expect(ui.doc()).toEqual(stable)
 await ui.click('Lock: '+instance.name)
 const changed=structuredClone(compact);changed.bodies.find((b:any)=>b.instance).instance.matrix[0][3]+=10
 await load(changed);expect(ui.doc()).toEqual(stable);expect(ui.text(ui.all()[0])).toContain('Object is locked')
})

it('cancels worker import on Escape and new edits, then applies one undoable import',async()=>{
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount(),before=ui.doc(),input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 const text=JSON.stringify({version:1,sketches:[],bodies:[]})
 const load=()=>input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'project.json'}})
 const first=load();await flushClearance();expect(ui.text(ui.all()[0])).toContain('Checking import')
 await commandKey(ui,'Escape');requests[0].resolve(parseDirectDocument(requests[0].job.text));await first;await flushClearance();expect(ui.doc()).toEqual(before)
 const second=load();await flushClearance();await ui.click('Box');const edited=ui.doc()
 requests[1].resolve(parseDirectDocument(requests[1].job.text));await second;await flushClearance();expect(ui.doc()).toEqual(edited)
 const third=load();await flushClearance();requests[2].resolve(parseDirectDocument(requests[2].job.text));await third;await flushClearance()
 expect(ui.doc().bodies).toHaveLength(0)
 previewWorkerRun.mockReset();await ui.click('↶');expect(ui.doc()).toEqual(edited)
})

it('keeps working geometry while display refinement is pending and rejects replies after smooth toggle and Escape',async()=>{
 const {prepareSolidDisplay}=await import('../src/services/solidDisplayPreparation')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 displayWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({seedDocument:cylinderSeed()});await flushClearance()
 const before=ui.doc(),coarse=before.bodies[0].mesh.indices.length/6
 const faces=()=>ui.all(ui.svg()).filter(n=>n.tag==='polygon'&&n.props['data-body']==='imported-cylinder')
 expect(faces()).toHaveLength(coarse);expect(requests).toHaveLength(1)
 await ui.click('Smooth B-rep display')
 requests[0].resolve(prepareSolidDisplay(requests[0].job.mesh,requests[0].job.brep));await flushClearance()
 expect(faces()).toHaveLength(coarse)
 await ui.click('Smooth B-rep display');expect(requests).toHaveLength(2)
 await commandKey(ui,'Escape')
 requests[1].resolve(prepareSolidDisplay(requests[1].job.mesh,requests[1].job.brep));await flushClearance()
 expect(faces()).toHaveLength(coarse);expect(ui.doc()).toEqual(before)
 await ui.click('Smooth B-rep display');await ui.click('Smooth B-rep display')
 const current=requests.at(-1)!,result=prepareSolidDisplay(current.job.mesh,current.job.brep)
 current.resolve(result);await flushClearance()
 expect(faces()).toHaveLength(result.mesh.indices.length/6)
 expect(faces().length).toBeGreaterThan(coarse);expect(ui.doc()).toEqual(before)
 await ui.click('Imported cylinder');await ui.click('Faces');await ui.pointer(faces()[0]);await flushClearance()
 expect(ui.all(ui.svg()).some(n=>n.tag==='polygon'&&String(n.props.style?.color).startsWith('hsl(40 '))).toBe(true)
 expect(ui.doc()).toEqual(before)
})

it('discards pending display refinement when the panel closes and retries on reopening',async()=>{
 const {prepareSolidDisplay}=await import('../src/services/solidDisplayPreparation')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 displayWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({seedDocument:cylinderSeed()});await flushClearance();expect(requests).toHaveLength(1)
 await ui.setProps({open:false});requests[0].resolve(prepareSolidDisplay(requests[0].job.mesh,requests[0].job.brep));await flushClearance()
 await ui.setProps({open:true});await flushClearance();expect(requests).toHaveLength(2)
 const current=requests[1];current.resolve(prepareSolidDisplay(current.job.mesh,current.job.brep));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='display-refinement')).toBe(false)
})

it('restores a saved baseline in a worker and discards startup replies after Escape or a new edit',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await flushClearance();expect(requests).toHaveLength(1)
 expect(ui.text(ui.all()[0])).toContain('Restoring geometry')
 await commandKey(ui,'Escape');await ui.click('Box');const edited=ui.doc()
 requests[0].resolve(parseDirectDocument(requests[0].job.text));await flushClearance()
 expect(ui.doc()).toEqual(edited);expect(ui.text(ui.all()[0])).not.toContain('Restoring geometry')
 const restored=await mount();await flushClearance();expect(requests).toHaveLength(2)
 requests[1].resolve(parseDirectDocument(requests[1].job.text));await flushClearance()
 expect(restored.all().some(n=>n.props['data-body']==='b')).toBe(true)
 expect(restored.button('↶').props.disabled).toBe(true)
 expect(restored.text(restored.all()[0])).not.toContain('Restoring geometry')
})

it('cancels shared draft loading, ignores late replies and preserves the local branch in Undo',async()=>{
 const ui=await mount();await ui.click('Box');const local=ui.doc()
 const external=structuredClone(local);external.bodies[0].name='Shared replacement'
 localStorage.setItem('scad-solid-modeler-v1',stringifyMeshJson(external));ui.storageChanged();await flushClearance()
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 await ui.click('Load saved version · keep local in Undo');expect(requests).toHaveLength(1)
 expect(ui.text(ui.all()[0])).toContain('Loading shared version')
 await commandKey(ui,'Escape');requests[0].resolve(parseDirectDocument(requests[0].job.text));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('Shared replacement');expect(ui.button('Load saved version · keep local in Undo').props.disabled).toBe(false)
 await ui.click('Load saved version · keep local in Undo');requests[1].reject(Error('shared decode failed'));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('shared decode failed');expect(ui.text(ui.all()[0])).not.toContain('Shared replacement')
 await ui.click('Load saved version · keep local in Undo');requests[2].resolve(parseDirectDocument(requests[2].job.text));await flushClearance()
 expect(ui.doc()).toEqual(external)
 previewWorkerRun.mockReset();await ui.click('↶');expect(ui.doc()).toEqual(local)
})

it('does not replace newer local edits with a delayed shared draft or expose its late error',async()=>{
 const ui=await mount();await ui.click('Box');const external=ui.doc();external.bodies[0].name='Stale shared body'
 localStorage.setItem('scad-solid-modeler-v1',stringifyMeshJson(external));ui.storageChanged();await flushClearance()
 let fail!:(error:Error)=>void
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?new Promise((_resolve,reject)=>fail=reject):undefined)
 await ui.click('Load saved version · keep local in Undo');await ui.click('Box')
 fail(Error('obsolete shared failure'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('Stale shared body');expect(ui.text(ui.all()[0])).not.toContain('obsolete shared failure')
 expect(ui.all().filter(n=>n.props['data-body']).length).toBeGreaterThan(0)
})

it('imports ModelGraph through a cancellable worker and preserves the prior scene in Undo',async()=>{
 const {importSolidModelGraph}=await import('../src/services/solidModelGraphImport')
 const ui=await mount(),before=ui.doc(),text=readFileSync('tests/fixtures/solid-modelgraph-import.json','utf8')
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='modelGraphImport'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const load=()=>input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'modelgraph.json'}})
 const first=load();await flushClearance();expect(requests).toHaveLength(1)
 await commandKey(ui,'Escape');requests[0].resolve(await importSolidModelGraph(requests[0].job.document,text));await first;await flushClearance()
 expect(ui.doc()).toEqual(before)
 const second=load();await flushClearance();requests[1].resolve(await importSolidModelGraph(requests[1].job.document,text));await second;await flushClearance()
 expect(ui.doc().bodies).toEqual(before.bodies)
 expect(ui.doc().curves?.map(c=>c.name)).toEqual(['path']);expect(ui.doc().surfaces?.map(s=>s.name)).toEqual(['skin'])
 const imported=ui.doc();await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(imported)
 const third=load();await flushClearance();await ui.click('Box');const edited=ui.doc()
 requests[2].resolve(await importSolidModelGraph(requests[2].job.document,text));await third;await flushClearance()
 expect(ui.doc()).toEqual(edited)
})

it.each([false,true])('cancels ModelGraph import on group change; late failure=%s',async failure=>{
 const {importSolidModelGraph}=await import('../src/services/solidModelGraphImport')
 const ui=await mount({seedDocument:{version:1,sketches:[],bodies:[],groups:[{name:'Destination',source:''}]}})
 const before=ui.doc(),text=readFileSync('tests/fixtures/solid-modelgraph-import.json','utf8')
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='modelGraphImport'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const load=()=>input.props.onChange({target:{files:[{size:text.length,text:async()=>text}],value:'modelgraph.json'}})
 const first=load();await flushClearance();expect(requests).toHaveLength(1)
 await ui.click('Active group: Destination')
 if(failure)requests[0].reject(Error('obsolete import destination'));else requests[0].resolve(await importSolidModelGraph(requests[0].job.document,text))
 await first;await flushClearance();expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).not.toContain('obsolete import destination')
 const second=load();await flushClearance();expect(requests[1].job.group).toBe('Destination')
 requests[1].resolve(await importSolidModelGraph(requests[1].job.document,text,requests[1].job.group));await second;await flushClearance()
 for(const item of [...ui.doc().curves??[],...ui.doc().surfaces??[]])expect(item.group).toBe('Destination')
 expect(ui.doc().curves).toHaveLength(1);expect(ui.doc().surfaces).toHaveLength(1)
 const after=ui.doc();await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
})

it.each([false,true])('discards ModelGraph file read after group change; failure=%s',async failure=>{
 const ui=await mount({seedDocument:{version:1,sketches:[],bodies:[],groups:[{name:'Destination',source:''}]}})
 const before=ui.doc(),text=readFileSync('tests/fixtures/solid-modelgraph-import.json','utf8')
 const input=ui.all().find(n=>n.tag==='input'&&n.props.accept==='.json,application/json')!
 let resolve!:(text:string)=>void,reject!:(e:Error)=>void
 const read=new Promise<string>((yes,no)=>{resolve=yes;reject=no})
 const pending=input.props.onChange({target:{files:[{size:text.length,text:()=>read}],value:'modelgraph.json'}})
 await flushClearance();await ui.click('Active group: Destination')
 const count=previewWorkerRun.mock.calls.filter(([job])=>job.kind==='modelGraphImport').length
 if(failure)reject(Error('obsolete file read'));else resolve(text)
 await pending;await flushClearance()
 expect(previewWorkerRun.mock.calls.filter(([job])=>job.kind==='modelGraphImport')).toHaveLength(count)
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).not.toContain('obsolete file read')
})

it('cancels primitive construction on Escape, parameter change and closure, then adds one undoable body',async()=>{
 const {addSolidPrimitive}=await import('../src/services/solidPrimitive')
 const ui=await mount(),before=ui.doc()
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 primitiveWorkerRun.mockImplementation(job=>new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 await ui.click('Box');expect(ui.text(ui.all()[0])).toContain('Creating body');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');requests[0].resolve(addSolidPrimitive(requests[0].job.document,requests[0].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Box');const size=ui.all().find(n=>n.tag==='input'&&n.props.max==='10000')!
 size.props['onUpdate:modelValue'](30);await flushClearance();requests[1].resolve(addSolidPrimitive(requests[1].job.document,requests[1].job.options));await flushClearance();expect(ui.doc()).toEqual(before)
 await ui.click('Box');requests[2].resolve(addSolidPrimitive(requests[2].job.document,requests[2].job.options));await flushClearance()
 expect(ui.doc().bodies).toHaveLength(before.bodies.length+1)
 expect(analyzeNurbsBrep(ui.doc().bodies.at(-1)!.brep!).signedVolumeMm3).toBeCloseTo(27000,5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Sphere');await ui.setProps({open:false});requests[3].reject(Error('obsolete primitive failure'));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).not.toContain('obsolete primitive failure')
})

it('discards vertex measurement replies after a changed input and Escape without editing history',async()=>{
 const {measureSolidVertices}=await import('../src/services/solidMeasurements')
 const ui=await mount();await ui.click('Box');const before=ui.doc()
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='measureVertices'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 await ui.click('Properties');await ui.click('Measurements');expect(requests).toHaveLength(1)
 const input=ui.all().find(n=>n.props['aria-label']==='Vertex B')!
 input.props['onUpdate:modelValue'](3);await flushClearance();expect(requests).toHaveLength(2)
 const result=(job:any)=>measureSolidVertices(job.a,job.indexA,job.b,job.indexB)
 requests[0].resolve(result(requests[0].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(false)
 requests[1].resolve(result(requests[1].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(true)
 input.props['onUpdate:modelValue'](2);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(result(requests[2].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(false);expect(ui.doc()).toEqual(before)
})

it('shows only the current edge curvature and ignores errors after the panel closes',async()=>{
 const {measureSolidEdgeCurvature}=await import('../src/services/solidMeasurements')
 const seed=cylinderSeed(),edge=seed.bodies[0].brep!.edges.findIndex(e=>e.curve.degree===2)
 const ui=await mount({seedDocument:seed});await ui.click('Imported cylinder');await ui.click('Edges')
 ui.all().find(n=>n.props['aria-label']==='Select edge')!.props.onChange({target:{value:String(edge)}});await flushClearance()
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='measureEdge'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 await ui.click('Properties');await ui.click('Measurements')
 const field=ui.all().find(n=>n.props['aria-label']==='Edge parameter')!
 field.props['onUpdate:modelValue'](.25);await flushClearance()
 const result=(job:any)=>measureSolidEdgeCurvature(job.body,job.edge,job.parameter)
 requests[0].resolve(result(requests[0].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='curvature')).toBe(false)
 requests[1].resolve(result(requests[1].job));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Local curvature radius: 3.000000 mm')
 field.props['onUpdate:modelValue'](.75);await flushClearance();await ui.setProps({open:false})
 requests[2].reject(Error('obsolete curvature error'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('obsolete curvature error')
})

it('ignores boundary reports after changed boundaries, selection and Escape',async()=>{
 const {measureSurfaceBoundaries}=await import('../src/services/solidSurfaceDiagnostics')
 const fixture=readFileSync('tests/fixtures/solid-surface-boundary.json','utf8'),ui=await mount({},fixture)
 await ui.click('Surface A');await ui.click('Surface B',true);const before=ui.doc()
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 boundaryWorkerRun.mockImplementation(job=>new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 await ui.click('Inspect surface boundary');expect(requests).toHaveLength(1)
 const field=ui.all().find(n=>n.props['aria-label']==='Boundary A')!
 field.props['onUpdate:modelValue']('vMax');await flushClearance();expect(requests).toHaveLength(2)
 const result=(job:any)=>measureSurfaceBoundaries(job.a,job.b,job.options)
 requests[0].resolve(result(requests[0].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-boundary-inspection'])).toBe(false)
 const latest=result(requests[1].job);requests[1].resolve(latest);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Maximum gap: '+latest.maxGapMm.toFixed(6))
 field.props['onUpdate:modelValue']('uMax');await flushClearance();await ui.click('Surface A')
 requests[2].reject(Error('obsolete boundary error'));await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('obsolete boundary error')
 await ui.click('Surface B',true);await flushClearance();expect(requests).toHaveLength(4)
 await commandKey(ui,'Escape');requests[3].resolve(result(requests[3].job));await flushClearance()
 expect(ui.all().some(n=>n.props['data-boundary-inspection'])).toBe(false);expect(ui.doc()).toEqual(before)
})

it('keeps surface display tied to current geometry, cancels and retries without document edits',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {tessellateSolidNurbsSurface}=await import('../src/services/solidNurbs')
 const fixture=readFileSync('tests/fixtures/solid-surface-boundary.json','utf8')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 surfaceDisplayWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({},fixture),before=ui.doc();await flushClearance()
 expect(requests).toHaveLength(1);expect(ui.all().some(n=>n.props['data-surface'])).toBe(false)
 await commandKey(ui,'Escape');requests[0].resolve(tessellateSolidNurbsSurface(requests[0].job.item));await flushClearance()
 expect(ui.all().some(n=>n.props['data-surface'])).toBe(false)
 await ui.click('Refresh surfaces');expect(requests).toHaveLength(2)
 requests[1].resolve(tessellateSolidNurbsSurface(requests[1].job.item));await flushClearance();expect(requests).toHaveLength(3)
 requests[2].resolve(tessellateSolidNurbsSurface(requests[2].job.item));await flushClearance()
 expect(ui.all().filter(n=>n.props['data-surface']==='surface-a')).toHaveLength(64)
 expect(ui.all().filter(n=>n.props['data-surface']==='surface-b')).toHaveLength(64)
 await ui.click('Surface A');const control=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('U segments'))!
 control.props['onUpdate:modelValue'](8);control.props.onChange();await flushClearance();expect(requests).toHaveLength(4)
 expect(ui.all().some(n=>n.props['data-surface']==='surface-a')).toBe(false)
 requests[3].resolve(tessellateSolidNurbsSurface(requests[3].job.item));await flushClearance()
 expect(ui.all().filter(n=>n.props['data-surface']==='surface-a')).toHaveLength(128)
 await ui.click('↶');await flushClearance();expect(ui.doc()).toEqual(before)
 expect(ui.all().filter(n=>n.props['data-surface']==='surface-a')).toHaveLength(64)
})


it('ignores a late topology result after changing the selected body',async()=>{
 await geometryKernel.warmGeometryKernel()
 const make=(id:string,size:number)=>{const brep=createBrepBox([0,0,0],[size,size,size]);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,2)}}
 const a=make('Topology A',10),b=make('Topology B',20)
 const requests:{job:any;resolve:(value:any)=>void}[]=[]
 topologyWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[a,b]}))
 await ui.click(a.name)
 expect(requests.length).toBeGreaterThan(0)
 const old=requests.at(-1)!
 await ui.click(b.name)
 const current=requests.at(-1)!
 expect(current).not.toBe(old)
 old.resolve(solidTopology(old.job.mesh));await flushClearance()
 expect(ui.button(b.name).props['aria-pressed']).toBe(true)
 expect(ui.all().some(n=>n.props['aria-label']==='topology-preparation')).toBe(true)
 current.resolve(solidTopology(current.job.mesh));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='topology-preparation')).toBe(false)
 expect(ui.button(b.name).props['aria-pressed']).toBe(true)
 expect(ui.doc().bodies.map(body=>body.id)).toEqual([a.id,b.id])
})


it('drops authored edge responses after changing the body or leaving edge mode',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {solidBodyEdges}=await import('../src/services/solidBodyEdges')
 const make=(id:string,size:number)=>{const brep=createBrepBox([0,0,0],[size,size,size]);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,2)}}
 const a=make('Edges A',10),b=make('Edges B',20),requests:{job:any;resolve:(value:any)=>void}[]=[]
 bodyEdgesWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[a,b]}))
 await ui.click(a.name);await ui.click('Pick edges')
 const old=requests.at(-1)!;expect(old.job.body.id).toBe(a.id)
 await ui.click(b.name);const current=requests.at(-1)!;expect(current.job.body.id).toBe(b.id)
 old.resolve(solidBodyEdges(old.job.body));await flushClearance()
 expect(ui.all().filter(n=>n.props['data-topology-edge'])).toHaveLength(0)
 expect(ui.all().some(n=>n.props['aria-label']==='body-edges')).toBe(true)
 current.resolve(solidBodyEdges(current.job.body));await flushClearance()
 expect(ui.all().filter(n=>n.props['data-topology-edge'])).toHaveLength(b.brep.edges.length)
 await ui.click(a.name);const late=requests.at(-1)!
 await ui.click('Pick bodies')
 late.resolve(solidBodyEdges(late.job.body));await flushClearance()
 expect(ui.all().filter(n=>n.props['data-topology-edge'])).toHaveLength(0)
 expect(ui.all().some(n=>n.props['aria-label']==='body-edges')).toBe(false)
 expect(ui.doc().bodies.map(body=>body.id)).toEqual([a.id,b.id])
})


it('does not apply a late sketch plane after selecting another body',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {prepareSolidFaceSketch}=await import('../src/services/solidFaceSketch')
 const make=(id:string,size:number)=>{const brep=createBrepBox([0,0,0],[size,size,size]);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,2)}}
 const a=make('Plane A',10),b=make('Plane B',20),requests:{job:any;resolve:(value:any)=>void}[]=[]
 faceSketchWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[a,b]})),svg=ui.svg(),camera=defaultDirectCamera()
 const face=solidTopology(a.mesh).faces.find(f=>f.normal[2]>.99)!,triangle=face.triangles[0]
 const expected=Array.from(a.mesh.indices.slice(triangle*3,triangle*3+3),i=>projectDirectPoint(Array.from(a.mesh.positions.slice(i*3,i*3+3)),camera).slice(0,2).join(',')).join(' ')
 await ui.click('On face')
 await ui.pointer(ui.all(svg).find(n=>n.tag==='polygon'&&n.props['data-body']===a.id&&n.props.points===expected)!)
 await flushClearance();await flushClearance()
 expect(requests).toHaveLength(1)
 expect(ui.all().some(n=>n.props['aria-label']==='face-sketch-preparation')).toBe(true)
 await ui.click(b.name)
 requests[0].resolve(prepareSolidFaceSketch(requests[0].job.body,requests[0].job.face));await flushClearance()
 expect(ui.button(b.name).props['aria-pressed']).toBe(true)
 expect(ui.all().some(n=>n.props['aria-label']==='face-sketch-preparation')).toBe(false)
 expect(ui.text(ui.all()[0])).not.toContain('Draw on the highlighted face')
 expect(ui.doc().sketches).toHaveLength(0)
})


it('discards body snap responses after hiding a body and retries failed visible targets',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {bodySnapGeometry}=await import('../src/services/solidSnapGeometry')
 const make=(id:string,size:number)=>{const brep=createBrepBox([0,0,0],[size,size,size]);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,2)}}
 const a=make('Snaps A',10),b=make('Snaps B',20)
 const requests:{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}[]=[]
 bodySnapsWorkerRun.mockImplementation(job=>new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[a,b]})),before=ui.doc()
 await flushClearance()
 const old=requests.at(-1)!;expect(old.job.body.id).toBe(a.id)
 await ui.click('Hide: '+a.name);await flushClearance()
 const current=requests.at(-1)!;expect(current.job.body.id).toBe(b.id)
 old.resolve(bodySnapGeometry(old.job.body));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(true)
 expect(requests.at(-1)).toBe(current)
 current.reject(Error('controlled snap failure'));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(false)
 expect(ui.text(ui.all()[0])).toContain('Snaps unavailable: '+b.id)
 await ui.click('Refresh snaps');await flushClearance()
 const retry=requests.at(-1)!;expect(retry).not.toBe(current);expect(retry.job.body.id).toBe(b.id)
 retry.resolve(bodySnapGeometry(retry.job.body));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(false)
 expect(ui.all().some(n=>n.props.role==='alert'&&ui.text(n).includes('Snaps unavailable'))).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('rebuilds snap targets after deletion and undo without accepting the previous snapshot result',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {bodySnapGeometry}=await import('../src/services/solidSnapGeometry')
 const make=(id:string,size:number)=>{const brep=createBrepBox([0,0,0],[size,size,size]);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,2)}}
 const a=make('History snaps A',10),b=make('History snaps B',20)
 const requests:{job:any;resolve:(value:any)=>void}[]=[]
 bodySnapsWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[a,b]})),before=ui.doc()
 await flushClearance();const old=requests.at(-1)!;expect(old.job.body.id).toBe(a.id)
 await ui.click(a.name);await ui.click('Delete');await flushClearance()
 expect(ui.doc().bodies.map(body=>body.id)).toEqual([b.id])
 const current=requests.at(-1)!;expect(current.job.body.id).toBe(b.id)
 old.resolve(bodySnapGeometry(old.job.body));await flushClearance()
 expect(requests.at(-1)).toBe(current)
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(true)
 current.resolve(bodySnapGeometry(current.job.body));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(false)
 await ui.click('↶');await flushClearance();expect(ui.doc()).toEqual(before)
 const restored=requests.at(-1)!;expect(restored).not.toBe(current);expect(restored.job.body.id).toBe(a.id)
 restored.resolve(bodySnapGeometry(restored.job.body));await flushClearance()
 expect(requests.at(-1)).toBe(restored)
 expect(ui.all().some(n=>n.props['aria-label']==='snap-preparation')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('blocks drawing with incomplete sketch snaps, discards cancelled replies and retries',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {sketchSnapGeometry}=await import('../src/services/modelingSnaps')
 const requests:{job:any;resolve:(value:any)=>void}[]=[]
 sketchSnapsWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const fixture={version:1,bodies:[],sketches:[{id:'target',name:'Target',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}]}
 const ui=await mount({},JSON.stringify(fixture)),before=ui.doc()
 await ui.click('Rectangle · R')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 await ui.pointer(svg,30,-30);svg.props.onPointermove(ui.event(svg,50,-50));svg.props.onPointerup(ui.event(svg,50,-50));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).toContain('Sketch snaps are not ready')
 const old=requests.at(-1)!
 await commandKey(ui,'Escape');old.resolve(sketchSnapGeometry([old.job.sketch]));await flushClearance()
 expect(ui.all().some(n=>n.props['aria-label']==='sketch-snap-preparation')).toBe(false)
 expect(ui.button('Refresh sketch snaps')).toBeDefined()
 await ui.click('Refresh sketch snaps');const current=requests.at(-1)!;expect(current).not.toBe(old)
 current.resolve(sketchSnapGeometry([current.job.sketch]));await flushClearance()
 await ui.click('Rectangle · R');await ui.pointer(svg,30,-30);svg.props.onPointermove(ui.event(svg,50,-50));svg.props.onPointerup(ui.event(svg,50,-50));await flushClearance()
 expect(ui.doc().sketches).toHaveLength(2);expect(ui.doc().sketches[0]).toEqual(before.sketches[0])
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('previews point trimming, cancels without edits and commits the retained end with undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {evaluateNurbsCurve}=await import('../src/services/nurbsCurve')
 const source={version:1,bodies:[],sketches:[],curves:[{id:'arc',name:'Trim target',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[10,0,0],[10,10,0],[0,10,0]],weights:[1,Math.SQRT1_2,1]}}]}
 const ui=await mount({},JSON.stringify(source)),before=ui.doc()
 await ui.click('Trim target');await ui.click('Trim NURBS at point')
 expect(ui.all().some(n=>n.props['aria-label']==='Cut point X'),ui.text(ui.all()[0])).toBe(true)
 for(const [axis,value] of [['X','8 mm'],['Y','6 mm'],['Z','0 mm']]){ui.all().find(n=>n.props['aria-label']==='Cut point '+axis)!.props['onUpdate:modelValue'](value);await flushClearance()}
 ui.all().find(n=>n.props['aria-label']==='Retain endpoint')!.props.onChange({target:{value:'end'}});await flushClearance()
 expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props['data-preview']==='point-trim'&&n.props.points.split(' ').length===49)).toBe(true)
 expect(ui.all().some(n=>n.props['data-diagnostic']==='point-trim-cut')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props['data-preview']==='point-trim')).toBe(false)
 await ui.click('Trim NURBS at point')
 ui.all().find(n=>n.props['aria-label']==='Cut point X')!.props['onUpdate:modelValue']('10 mm');ui.all().find(n=>n.props['aria-label']==='Cut point Y')!.props['onUpdate:modelValue']('0 mm');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Curve endpoint')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 ui.all().find(n=>n.props['aria-label']==='Cut point X')!.props['onUpdate:modelValue']('8 mm');ui.all().find(n=>n.props['aria-label']==='Cut point Y')!.props['onUpdate:modelValue']('6 mm')
 ui.all().find(n=>n.props['aria-label']==='Retain endpoint')!.props.onChange({target:{value:'end'}});await flushClearance()
 await ui.click('Apply · Enter');const trimmed=ui.doc().curves![0]
 expect(trimmed.id).toBe('arc');expect(trimmed.curve.controlPoints).not.toEqual(before.curves![0].curve.controlPoints)
 const point=evaluateNurbsCurve(trimmed.curve,trimmed.curve.knots[trimmed.curve.degree]).point
 expect(point[0]).toBeCloseTo(8,7);expect(point[1]).toBeCloseTo(6,7)
 const changed=ui.doc();await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(changed)
})

it('discards obsolete point trim previews after input changes and Escape',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {applySolidNurbsEdit}=await import('../src/services/solidNurbsEdit')
 const requests:{job:any;resolve:(value:any)=>void}[]=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'&&job.options.kind==='trim-point'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const source={version:1,bodies:[],sketches:[],curves:[{id:'line',name:'Trim line',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[10,0,0]],weights:[1,1]}}]}
 const ui=await mount({},JSON.stringify(source)),before=ui.doc()
 await ui.click('Trim line');await ui.click('Trim NURBS at point')
 const old=requests.at(-1)!
 ui.all().find(n=>n.props['aria-label']==='Cut point X')!.props['onUpdate:modelValue']('6 mm');await flushClearance()
 const current=requests.at(-1)!;expect(current).not.toBe(old)
 old.resolve(applySolidNurbsEdit(old.job.document,old.job.options));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');current.resolve(applySolidNurbsEdit(current.job.document,current.job.options));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='point-trim')).toBe(false)
})

it('invalidates screen trim requests on another click, numeric mode and Escape',async()=>{
 await geometryKernel.warmGeometryKernel()
 const requests:{job:any;resolve:(value:any)=>void}[]=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='nurbsEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const source={version:1,bodies:[],sketches:[],curves:[{id:'line',name:'Trim line',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,0,10],[10,0,20]],weights:[1,1]}}]}
 const ui=await mount({},JSON.stringify(source)),before=ui.doc()
 await ui.click('Trim line');await ui.click('Trim NURBS at point')
 const target=()=>ui.all(ui.svg()).find(n=>n.tag==='polyline'&&n.props.stroke==='#ffc977'&&n.props.onPointerdown)!
 await ui.pointer(target(),11,12);await flushClearance();const old=requests.at(-1)!
 expect(old.job.options).toMatchObject({kind:'trim-screen-point',point:[11,12],keep:'start'})
 expect(old.job.options.matrix.flat().every(Number.isFinite)).toBe(true)
 await ui.pointer(target(),13,14);await flushClearance();const current=requests.at(-1)!
 expect(current).not.toBe(old)
 old.resolve(structuredClone(before));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await ui.click('Enter coordinates');expect(requests.at(-1)!.job.options.kind).toBe('trim-point')
 current.resolve(structuredClone(before));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 const numeric=requests.at(-1)!
 await commandKey(ui,'Escape');numeric.resolve(structuredClone(before));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='point-trim')).toBe(false)
})

it('updates section diagnostics without materializing a history geometry copy',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const ui=await mount();await ui.click('Cube');const before=ui.doc()
 const read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  await ui.click('Body diagnostics')
  const offset=ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Section offset, mm')!
  offset.props['onUpdate:modelValue']('6 mm');await flushClearance()
  expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
  expect(read).not.toHaveBeenCalled()
  expect(ui.doc()).toEqual(before)
 }finally{read.mockRestore()}
})

it('invalidates section previews on malformed input and explains a zero normal at its fields',async()=>{
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Body diagnostics')
 const normal=(axis:string)=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Normal '+axis)!
 const count=()=>previewWorkerRun.mock.calls.filter(([job])=>job.kind==='brepTool'&&job.options.kind==='display').length
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
 const requests=count();normal('X').props['onUpdate:modelValue']('invalid');await flushClearance()
 expect(normal('X').props['aria-invalid']).toBe(true);expect(count()).toBe(requests)
 const localError=normal('X').props['aria-errormessage']
 expect(localError).toBeTruthy()
 expect(normal('X').props['aria-describedby'].split(' ')).toEqual(['solid-section-error',localError])
 expect(ui.all().some(n=>n.props.id===localError&&ui.text(n)==='Enter a finite number')).toBe(true)
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(false)
 expect(ui.text(ui.all()[0])).toContain('Section not checked. Correct the highlighted plane fields.')
 expect(ui.button('Retry diagnostics').props.disabled).toBe(true)
 normal('X').props['onUpdate:modelValue']('0');normal('Z').props['onUpdate:modelValue']('0');await flushClearance()
 expect(normal('X').props['aria-errormessage']).toBeUndefined()
 expect(normal('X').props['aria-describedby']).toBe('solid-section-error')
 expect(ui.all().some(n=>n.props.id===localError)).toBe(false)
 expect(ui.text(ui.all()[0])).toContain('The plane normal is zero or outside the numeric range.')
 expect(ui.all().find(n=>n.props.role==='group'&&n.props['aria-label']==='Plane normal')!.props['aria-invalid']).toBe(true)
 expect(normal('Z').props['aria-describedby']).toBe('solid-section-error')
 await ui.click('XY plane');expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
 expect(ui.doc()).toEqual(before)
})

it('retries failed diagnostic workers without modifying the body',async()=>{
 let displayRequests=0
 previewWorkerRun.mockImplementation(job=>job.kind==='brepTool'&&job.options.kind==='display'&&displayRequests++===0?Promise.reject(Error('display worker failed')):undefined)
 contactWorkerRun.mockRejectedValueOnce(Error('contact worker failed'))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Body diagnostics')
 expect(ui.text(ui.all()[0])).toContain('Could not inspect the selected body mesh. Retry the inspection.')
 expect(ui.text(ui.all()[0])).toContain('Self-intersection inspection incomplete. Retry;')
 expect(ui.all().find(n=>n.props['data-testid']==='diagnostic-body')!.props['data-body-id']).toBe('b')
 await ui.click('Retry diagnostics')
 expect(displayRequests).toBe(2);expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true)
 expect(ui.text(ui.all()[0])).not.toContain('display worker failed');expect(ui.text(ui.all()[0])).not.toContain('contact worker failed')
 expect(ui.doc()).toEqual(before)
})

it('ignores an in-flight diagnostic result when a plane field becomes invalid',async()=>{
 const {applySolidBrepTool}=await import('../src/services/solidBrepTool')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='brepTool'&&job.options.kind==='display'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Cube');await ui.click('Body diagnostics');const before=ui.doc(),old=requests[0]
 const offset=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Section offset, mm')!
 offset().props['onUpdate:modelValue']('');await flushClearance()
 old.resolve(applySolidBrepTool(old.job.document,old.job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(false)
 expect(ui.text(ui.all()[0])).toContain('Section not checked. Correct the highlighted plane fields.')
 expect(ui.doc()).toEqual(before)
 offset().props['onUpdate:modelValue']('6 mm');await flushClearance();const latest=requests.at(-1)!
 expect(latest).not.toBe(old);latest.resolve(applySolidBrepTool(latest.job.document,latest.job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='section')).toBe(true);expect(ui.doc()).toEqual(before)
})

it('navigates all detected contacts and never calls an incomplete empty scan clean',async()=>{
 const mesh={positions:new Float64Array([0,0,0,2,0,0,0,2,0]),indices:new Uint32Array([0,1,2,0,1,2,0,1,2])}
 const ui=await mount({initialDocument:{version:1,sketches:[],bodies:[{id:'overlap',name:'Overlap',mesh}]}})
 await ui.click('Overlap');await ui.click('Body diagnostics');const before=ui.doc()
 expect(ui.text(ui.all()[0])).toContain('Contacts found: 3')
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='intersection')).toHaveLength(3)
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='intersection-selected')).toHaveLength(2)
 await ui.click('Next contact');expect(ui.text(ui.all()[0])).toContain('Disallowed triangle contact: 1 / 3')
 await ui.click('Next contact');expect(ui.text(ui.all()[0])).toContain('Disallowed triangle contact: 2 / 3')
 expect(ui.button('Next contact').props.disabled).toBe(true)
 await ui.click('Previous contact');expect(ui.text(ui.all()[0])).toContain('Disallowed triangle contact: 1 / 3')
 contactWorkerRun.mockResolvedValueOnce({contacts:[],contact:null,triangleIds:[],lines:[],complete:false,stopReason:'work-limit',work:200_000,maxWork:200_000,maxContacts:10_000,relativeTolerance:1e-9,scope:'display-mesh-all-contacts'})
 await ui.click('Retry diagnostics')
 expect(ui.text(ui.all()[0])).toContain('Inspection is incomplete.')
 expect(ui.text(ui.all()[0])).not.toContain('No disallowed mesh contacts found')
 const effort=ui.all().find(n=>n.props['aria-label']==='Contact inspection effort')!
 effort.props['onUpdate:modelValue'](1_000_000);await flushClearance()
 expect(contactWorkerRun.mock.calls.at(-1)![0].maxWork).toBe(1_000_000)
 expect(ui.text(ui.all()[0])).toContain('Contacts found: 3');expect(ui.doc()).toEqual(before)
})

it('discards a contact report computed with an older inspection effort',async()=>{
 const {inspectSolidIntersections}=await import('../src/services/solidDiagnostics')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 contactWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Cube');await ui.click('Body diagnostics');const before=ui.doc()
 const old=requests[0],effort=ui.all().find(n=>n.props['aria-label']==='Contact inspection effort')!
 effort.props['onUpdate:modelValue'](1_000_000);await flushClearance();const current=requests.at(-1)!
 expect(current.job.maxContacts).toBe(50_000)
 old.resolve(inspectSolidIntersections(old.job.mesh));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Inspecting mesh contacts…')
 expect(ui.text(ui.all()[0])).not.toContain('No disallowed mesh contacts found')
 current.resolve(inspectSolidIntersections(current.job.mesh,{maxWork:current.job.maxWork,maxContacts:current.job.maxContacts}));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('No disallowed mesh contacts found');expect(ui.doc()).toEqual(before)
})


it('shows bounded edge distance, rejects stale inputs, and clears canceled witnesses',async()=>{
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='curveDistance'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount({seedDocument:cylinderSeed()});await ui.click('Imported cylinder');await ui.click('Properties');await ui.click('Measurements');await ui.click('Distance between edges')
 const before=ui.doc(),field=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Edge B')!
 const result={method:'interval-de-boor-pair-subdivision',distanceIntervalMm:[2,2.0001],parameters:[0,0],points:[[0,0,0],[0,0,2]],pointEnclosures:[[[0,0],[0,0],[0,0]],[[0,0],[0,0],[2,2]]],converged:true,reason:'tolerance',cells:5,maxCells:10000,toleranceMm:.001}
 expect(requests).toHaveLength(1)
 field().props['onUpdate:modelValue'](3);await flushClearance()
 requests[0].resolve(result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='edge-distance')).toBe(false)
 requests[1].resolve({...result,converged:false,reason:'work-limit',distanceIntervalMm:[1,3]});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Calculation incomplete')
 expect(ui.all().some(n=>n.props['data-measurement']==='edge-distance')).toBe(true)
 field().props['onUpdate:modelValue'](2);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='edge-distance')).toBe(false)
 await ui.click('Properties');await ui.click('Measurements');await flushClearance()
 field().props['onUpdate:modelValue'](9999);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Choose an existing edge B.')
 expect(ui.doc()).toEqual(before)
})


it('discards surface-distance reports after budget changes, selection changes and Escape',async()=>{
 await geometryKernel.warmGeometryKernel()
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='surfaceDistance'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const fixture=readFileSync('tests/fixtures/solid-surface-boundary.json','utf8'),ui=await mount({},fixture)
 await ui.click('Surface A');await ui.click('Surface B',true);const before=ui.doc()
 await ui.click('Distance between surfaces')
 const field=()=>ui.all().find(n=>n.props['aria-label']==='Surface calculation budget')!
 const result={method:'interval-tensor-de-boor-pair-subdivision',scope:'untrimmed-surfaces',distanceIntervalMm:[.2499,.2501],parameters:[[1,0],[0,0]],points:[[10,0,0],[10.25,0,0]],pointEnclosures:[[[10,10],[0,0],[0,0]],[[10.25,10.25],[0,0],[0,0]]],converged:true,reason:'tolerance',cells:1,maxCells:10000,toleranceMm:.001}
 field().props['onUpdate:modelValue'](100000);await flushClearance()
 requests[0].resolve(result);await flushClearance();expect(ui.all().some(n=>n.props['data-measurement']==='surface-distance')).toBe(false)
 requests[1].resolve({...result,maxCells:100000,converged:false,reason:'work-limit',distanceIntervalMm:[.1,.3]});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Calculation incomplete')
 expect(ui.all().some(n=>n.props['data-measurement']==='surface-distance')).toBe(true)
 field().props['onUpdate:modelValue'](10000);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(result);await flushClearance();expect(ui.all().some(n=>n.props['data-measurement']==='surface-distance')).toBe(false)
 await ui.click('Distance between surfaces');await ui.click('Surface A')
 requests[3].resolve(result);await flushClearance();expect(ui.all().some(n=>n.props['data-measurement']==='surface-distance')).toBe(false)
 expect(ui.doc()).toEqual(before)
})

it('discards stale face witnesses and keeps missing witnesses out of the viewport',async()=>{
 await geometryKernel.warmGeometryKernel()
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='faceDistance'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const fixture=JSON.parse(readFileSync('tests/fixtures/face-distance.json','utf8'))
 const ui=await mount({},JSON.stringify(fixture.document))
 await ui.click('Plate with hole');await ui.click('Properties');await ui.click('Measurements');await ui.click('Distance between faces')
 const before=ui.doc(),field=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Face A')!
 expect(requests).toHaveLength(1)
 field().props['onUpdate:modelValue'](10);await flushClearance()
 requests[0].resolve(fixture.result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='face-distance')).toBe(false)
 requests[1].resolve({...fixture.result,converged:false,reason:'domain-work-limit',distanceIntervalMm:[0,null],points:null,parameters:null,pointEnclosures:null});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Calculation incomplete')
 expect(ui.all().some(n=>n.props['data-measurement']==='face-distance')).toBe(false)
 field().props['onUpdate:modelValue'](9);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(fixture.result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='face-distance')).toBe(false)
 await ui.click('Properties');await ui.click('Measurements');await flushClearance()
 field().props['onUpdate:modelValue'](9999);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Choose an existing face A.')
 expect(ui.doc()).toEqual(before)
})

it('invalidates global shell reports on budget changes, cancellation and invalid targets',async()=>{
 await geometryKernel.warmGeometryKernel()
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 measurementWorkerRun.mockImplementation(job=>job.kind==='shellDistance'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const fixture=JSON.parse(readFileSync('tests/fixtures/face-distance.json','utf8'))
 const ui=await mount({},JSON.stringify(fixture.document))
 await ui.click('Plate with hole');await ui.click('Properties');await ui.click('Measurements');await ui.click('Distance between shells')
 expect(ui.text(ui.all()[0])).toContain('Choose a different body B.')
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(false)
 expect(requests).toHaveLength(0)
 const target=()=>ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Body B')!
 const budget=()=>ui.all().find(n=>n.props['aria-label']==='Shell calculation budget')!
 target().props['onUpdate:modelValue']('probe');await flushClearance()
 const before=ui.doc(),result={...fixture.result,method:'interval-trimmed-face-pairs',scope:'boundary-shells-bounded-joins',containment:'not-classified',faces:[fixture.faceA,fixture.faceB],pairs:60,evaluatedPairs:1}
 budget().props['onUpdate:modelValue'](1000000);await flushClearance()
 requests[0].resolve(result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='shell-distance')).toBe(false)
 requests[1].resolve({...result,converged:false,reason:'domain-work-limit',distanceIntervalMm:[0,null],points:null,parameters:null,pointEnclosures:null,faces:null});await flushClearance()
 await vi.waitFor(()=>expect(ui.text(ui.all()[0])).toContain('upper bound is unknown'))
 expect(ui.all().some(n=>n.props['data-measurement']==='shell-distance')).toBe(false)
 budget().props['onUpdate:modelValue'](100000);await flushClearance();await commandKey(ui,'Escape')
 requests[2].resolve(result);await flushClearance()
 expect(ui.all().some(n=>n.props['data-measurement']==='shell-distance')).toBe(false)
 await ui.click('Properties');await ui.click('Measurements');await flushClearance()
 target().props['onUpdate:modelValue']('plate');await flushClearance()
 expect(target().props['aria-invalid']).toBe(true)
 expect(ui.text(ui.all()[0])).toContain('Choose a different body B.')
 expect(ui.doc()).toEqual(before)
})

it('invalidates a pending transform on malformed input and rebuilds when validity returns',async()=>{
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sceneEdit'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Transform selection')
 quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance()
 const old=requests.at(-1)!,count=requests.length
 quantityField(ui,'X').props['onUpdate:modelValue']('-');await flushClearance()
 expect(requests).toHaveLength(count)
 old.resolve(applySolidSceneEdit(old.job.document,old.job.options));await flushClearance()
 expect(ui.all().some(n=>n.props['data-preview-body'])).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 quantityField(ui,'X').props['onUpdate:modelValue']('5');await flushClearance()
 expect(requests).toHaveLength(count+1)
 const fresh=requests.at(-1)!;fresh.resolve(applySolidSceneEdit(fresh.job.document,fresh.job.options));await flushClearance()
 await commandKey(ui,'Enter')
 expect(Math.min(...ui.doc().bodies[0].mesh.positions.filter((_,i)=>i%3===0))).toBeCloseTo(5)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each([
 ['Extrude','Height, mm','extrusion','12'],
 ['Revolve','Angle, °','revolve','180'],
 ['Fillet','Radius, mm','sketchEdit','2'],
] as const)('discards %s preview on invalid quantity and resumes at the same value',async(command,field,kind,value)=>{
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const {applySolidSketchEdit}=await import('../src/services/solidSketchEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind===kind?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 const compute=(job:any)=>kind==='revolve'?applySolidRevolve(job.document,job.options):kind==='extrusion'?applyDirectExtrusionProfile(job.document,job.options):applySolidSketchEdit(job.document,job.options)
 vi.useFakeTimers()
 try{
  await ui.click(command);quantityField(ui,field).props['onUpdate:modelValue'](value)
  await vi.advanceTimersByTimeAsync(100);await flushClearance()
  const old=requests.at(-1)!,count=requests.length;expect(old).toBeDefined()
  quantityField(ui,field).props['onUpdate:modelValue']('-');await vi.advanceTimersByTimeAsync(100);await flushClearance()
  expect(requests).toHaveLength(count)
  old.resolve(compute(old.job));await flushClearance()
  await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
  expect(ui.button('Apply · Enter').props.disabled).toBe(true)
  quantityField(ui,field).props['onUpdate:modelValue'](value);await vi.advanceTimersByTimeAsync(100);await flushClearance()
  expect(requests).toHaveLength(count+1)
  const fresh=requests.at(-1)!;fresh.resolve(compute(fresh.job));await flushClearance()
  await ui.click('Apply · Enter');expect(ui.doc()).not.toEqual(before)
  await ui.click('↶');expect(ui.doc()).toEqual(before)
 }finally{vi.useRealTimers()}
})


it.each([
 ['Lock',false],['Lock',true],['Hide',false],['Hide',true],
] as const)('invalidates body preview on %s with completed=%s',async(action,completed)=>{
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise(resolve=>requests.push({job,resolve})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('5 mm');await flushClearance()
 const pending=[...requests],latest=pending.at(-1)!
 if(completed){latest.resolve(applySolidBodyEdit(latest.job.document,latest.job.options));await flushClearance();expect(ui.button('Apply · Enter').props.disabled).toBe(false)}
 await ui.click(action+': Cube')
 for(const request of pending)request.resolve(applySolidBodyEdit(request.job.document,request.job.options))
 await flushClearance();await commandKey(ui,'Enter')
 expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props.class==='operation-card')).toBe(false)
 expect(ui.all().some(n=>n.props['data-preview-body'])).toBe(false)
 expect(ui.button('Cube').props.disabled).toBe(true)
 await ui.click((action==='Lock'?'Unlock':'Show')+': Cube');await ui.click('Cube');await ui.click('Split')
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('5 mm');await flushClearance()
 const current=requests.at(-1)!;expect(pending).not.toContain(current)
 current.resolve(applySolidBodyEdit(current.job.document,current.job.options));await flushClearance()
 await commandKey(ui,'Enter');expect(ui.doc().bodies).toHaveLength(2)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})


it('retries a failed body calculation without changing inputs or adding history before Apply',async()=>{
 const {applySolidBodyEdit}=await import('../src/services/solidBodyEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((resolve,reject)=>requests.push({job,resolve,reject})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('5 mm');await flushClearance()
 const failed=requests.at(-1)!;failed.reject(Error('worker unavailable'));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await ui.click('Retry calculation');const retry=requests.at(-1)!
 expect(retry).not.toBe(failed);expect(retry.job).toEqual(failed.job)
 expect(ui.button('Retry calculation').props.disabled).toBe(true)
 retry.resolve(applySolidBodyEdit(retry.job.document,retry.job.options));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.text(ui.all()[0])).not.toContain('worker unavailable')
 expect(ui.button('Retry calculation').props.disabled).toBe(false)
 const count=requests.length;await commandKey(ui,'Enter');expect(requests).toHaveLength(count)
 expect(ui.doc().bodies).toHaveLength(2);await ui.click('↶');expect(ui.doc()).toEqual(before)
})


it.each([
 ['CAD_CRASH','The calculation stopped unexpectedly'],
 ['CAD_TIMEOUT','The calculation timed out'],
 ['CAD_TRANSPORT','A valid calculation result could not be received'],
 ['CAD_PROTOCOL','A valid calculation result could not be received'],
 ['BREP_RESOURCE_LIMIT','The calculation budget was exhausted'],
 ['BREP_ANALYSIS_INDETERMINATE','Calculation accuracy is unconfirmed'],
] as const)('explains %s body failures with a recovery action',async(code,message)=>{
 const requests:Array<{reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((_resolve,reject)=>requests.push({reject})))
 const ui=await mount();await ui.click('Cube');const before=ui.doc();await ui.click('Split')
 const keys=ui.all().find(n=>String(n.props.class??'').includes('command-keys'))!
 expect(ui.text(keys)).not.toContain('Enter');expect(ui.text(keys)).toContain('Esc')
 requests.at(-1)!.reject(Object.assign(Error('internal implementation detail'),{code}));await flushClearance()
 const text=ui.text(ui.all()[0]);expect(text).toContain(message);expect(text).toContain('The model is unchanged')
 expect(text).not.toContain('internal implementation detail');expect(ui.doc()).toEqual(before)
 if(code==='BREP_RESOURCE_LIMIT'||code==='BREP_ANALYSIS_INDETERMINATE')expect(text).toContain('Cube: ')
 expect(ui.button('Retry calculation').props.disabled).toBe(false)
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
})


it.each([
 ['BREP_RESOURCE_LIMIT','Исчерпан лимит вычислений'],
 ['BREP_ANALYSIS_INDETERMINATE','Точность расчёта не подтверждена'],
] as const)('localizes %s body failures in Russian without changing the model',async(code,message)=>{
 const requests:Array<{reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='restoreDocument'?undefined:new Promise((_resolve,reject)=>requests.push({reject})))
 const ui=await mount({locale:'ru'});await ui.click('Cube');const before=ui.doc();await ui.click('Разрезать')
 requests.at(-1)!.reject(Object.assign(Error('internal implementation detail'),{code}));await flushClearance()
 const text=ui.text(ui.all()[0]);expect(text).toContain('Cube: '+message);expect(text).toContain('Модель не изменена')
 expect(text).not.toContain('internal implementation detail');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})


it('opens guided subtraction without cloning the history for every selected body',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 const first=await mount(),cube=first.doc().bodies[0]
 const bodies=['A','B','C'].map(id=>({...structuredClone(cube),id,name:id}))
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies}))
 await ui.click('A');await ui.click('B',true);await ui.click('C',true)
 const before=ui.doc(),read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  await ui.click('Subtract: A − B')
  expect(read).not.toHaveBeenCalled()
  expect(ui.doc()).toEqual(before)
  await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 }finally{read.mockRestore()}
})

it.each(['CAD_CRASH','CAD_TIMEOUT','CAD_TRANSPORT','CAD_PROTOCOL'])('recovers Boolean from %s without changing operands or history on failure',async code=>{
 const {applySolidBoolean}=await import('../src/services/solidBoolean')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='boolean'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const brep=createBrepCylinder(3,5),mesh=tessellateNurbsBrep(brep,4)
 const body=(id:string)=>({id,name:id,brep:structuredClone(brep),mesh:{positions:mesh.positions,indices:mesh.indices}})
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[body('Stock'),body('Cutter')]}))
 const before=ui.doc()
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep Union')
 requests[0].reject(Object.assign(Error('transport implementation detail'),{code}));await flushClearance()
 expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).toContain('The model is unchanged')
 expect(ui.text(ui.all()[0])).not.toContain('transport implementation detail')
 if(code==='CAD_CRASH')expect(ui.text(ui.all()[0])).toContain('Run the operation again with the selected bodies')
 await ui.click('B-rep Union');expect(requests[1].job).toEqual(requests[0].job)
 requests[1].resolve(applySolidBoolean(requests[1].job.document,requests[1].job.options));await flushClearance()
 expect(ui.doc().bodies).toHaveLength(1)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each(['Extrude · E','Revolve'])('retries %s preview after a worker crash and commits only the successful result',async command=>{
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>['extrusion','revolve'].includes(job.kind)?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 vi.useFakeTimers()
 try{
  await ui.click(command);await vi.advanceTimersByTimeAsync(100)
  requests[0].reject(Object.assign(Error('internal crash'),{code:'CAD_CRASH'}));await flushClearance()
  expect(ui.text(ui.all()[0])).toContain('The calculation stopped unexpectedly')
  expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
  await ui.click('Retry calculation');expect(ui.button('Retry calculation').props.disabled).toBe(true)
  await vi.advanceTimersByTimeAsync(100)
  const {id:oldId,...oldOptions}=requests[0].job.options,{id:newId,...newOptions}=requests[1].job.options
  expect(newOptions).toEqual(oldOptions);expect(requests[1].job.document).toEqual(requests[0].job.document)
  const job=requests[1].job
  requests[1].resolve(job.kind==='extrusion'?applyDirectExtrusionProfile(job.document,job.options):applySolidRevolve(job.document,job.options));await flushClearance()
  expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(false)
  await commandKey(ui,'Enter');expect(requests).toHaveLength(2);expect(ui.doc().bodies).toHaveLength(before.bodies.length+1)
  await ui.click('↶');expect(ui.doc()).toEqual(before)
 }finally{vi.useRealTimers()}
})

it.each([
 ['Extrude · E',false],['Extrude · E',true],['Revolve',false],['Revolve',true],
] as const)('ignores cancelled retry for %s with late failure=%s after reopening',async(command,lateFailure)=>{
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>['extrusion','revolve'].includes(job.kind)?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc()
 vi.useFakeTimers()
 const result=(job:any)=>job.kind==='extrusion'?applyDirectExtrusionProfile(job.document,job.options):applySolidRevolve(job.document,job.options)
 try{
  await ui.click(command);await vi.advanceTimersByTimeAsync(100)
  requests[0].reject(Object.assign(Error('first failure'),{code:'CAD_CRASH'}));await flushClearance()
  await ui.click('Retry calculation');await vi.advanceTimersByTimeAsync(100)
  const cancelled=requests[1]
  await commandKey(ui,'Escape');await ui.click(command);await vi.advanceTimersByTimeAsync(100)
  expect(ui.all().some(n=>n.tag==='button'&&ui.text(n)==='Retry calculation')).toBe(false)
  if(lateFailure)cancelled.reject(Error('obsolete retry failure'));else cancelled.resolve(result(cancelled.job))
  await flushClearance();await commandKey(ui,'Enter')
  expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
  expect(ui.text(ui.all()[0])).not.toContain('obsolete retry failure')
  expect(ui.all().some(n=>n.props['data-preview-body'])).toBe(false)
  requests[2].resolve(result(requests[2].job));await flushClearance()
  await commandKey(ui,'Enter');expect(ui.doc().bodies).toHaveLength(before.bodies.length+1)
  await ui.click('↶');expect(ui.doc()).toEqual(before)
 }finally{vi.useRealTimers()}
})

it.each([
 ['Extrude · E',false],['Extrude · E',true],['Revolve',false],['Revolve',true],
] as const)('refreshes %s after destination group change with late failure=%s',async(command,lateFailure)=>{
 const {applyDirectExtrusionProfile}=await import('../src/services/directExtrusion')
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>['extrusion','revolve'].includes(job.kind)?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount({seedDocument:{version:1,sketches:[{id:'s',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[],groups:[{name:'Destination',source:''}]}});await ui.click('Profile');const before=ui.doc()
 const result=(job:any)=>job.kind==='extrusion'?applyDirectExtrusionProfile(job.document,job.options):applySolidRevolve(job.document,job.options)
 vi.useFakeTimers()
 try{
  await ui.click(command);await vi.advanceTimersByTimeAsync(100);expect(requests).toHaveLength(1)
  await ui.click('Active group: Destination');await vi.advanceTimersByTimeAsync(100)
  expect(requests).toHaveLength(2)
  if(lateFailure)requests[0].reject(Error('obsolete destination preview'));else requests[0].resolve(result(requests[0].job))
  await flushClearance();expect(ui.doc()).toEqual(before)
  expect(ui.button('Apply · Enter').props.disabled).toBe(true)
  expect(ui.text(ui.all()[0])).not.toContain('obsolete destination preview')
  expect(ui.all().some(n=>n.props['data-preview-body'])).toBe(false)
  requests[1].resolve(result(requests[1].job));await flushClearance();await commandKey(ui,'Enter')
  expect(ui.doc().bodies).toHaveLength(1);expect(ui.doc().bodies[0].group).toBe('Destination')
  await ui.click('↶');expect(ui.doc()).toEqual(before)
 }finally{vi.useRealTimers()}
})

it.each(['Fillet','DogEar','Circular copies'])('retries sketch command %s after a worker failure',async command=>{
 const {applySolidSketchEdit}=await import('../src/services/solidSketchEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sketchEdit'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount();await ui.click('Profile');const before=ui.doc();await ui.click(command)
 const failed=requests.at(-1)!
 failed.reject(Object.assign(Error('internal transport detail'),{code:'CAD_TRANSPORT'}));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('A valid calculation result could not be received')
 expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await ui.click('Retry calculation');const retry=requests.at(-1)!
 const {copyIds:oldIds,...oldOptions}=failed.job.options,{copyIds:newIds,...newOptions}=retry.job.options
 expect(newOptions).toEqual(oldOptions);expect(retry.job.document).toEqual(failed.job.document)
 expect(ui.button('Retry calculation').props.disabled).toBe(true)
 retry.resolve(applySolidSketchEdit(retry.job.document,retry.job.options));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 const count=requests.length;await commandKey(ui,'Enter');expect(requests).toHaveLength(count)
 expect(ui.doc()).not.toEqual(before);await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each(['Fillet','DogEar','Circular copies'].flatMap(command=>[false,true].map(failure=>({command,failure}))))('refreshes sketch $command after group change; late failure=$failure',async({command,failure})=>{
 const {applySolidSketchEdit}=await import('../src/services/solidSketchEdit')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='sketchEdit'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const ui=await mount({seedDocument:{version:1,sketches:[{id:'s',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[],groups:[{name:'Destination',source:''}]}})
 await ui.click('Profile');const before=ui.doc();await ui.click(command);const old=requests.at(-1)!
 await ui.click('Active group: Destination');const current=requests.at(-1)!
 expect(current).not.toBe(old)
 if(failure)old.reject(Error('obsolete sketch group'));else old.resolve(applySolidSketchEdit(old.job.document,old.job.options))
 await flushClearance();expect(ui.doc()).toEqual(before);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.text(ui.all()[0])).not.toContain('obsolete sketch group')
 current.resolve(applySolidSketchEdit(current.job.document,current.job.options));await flushClearance();await commandKey(ui,'Enter')
 expect(ui.doc()).not.toEqual(before)
 if(command==='Circular copies')for(const sketch of ui.doc().sketches.filter(s=>s.id!=='s'))expect(sketch.group).toBe('Destination')
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('cancels obsolete pair inspection when enabling within-face diagnostics',async()=>{
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>['faceContacts','selfIntersection'].includes(job.kind)?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const brep=createBrepBox([0,0,0],[10,10,10]),mesh=tessellateNurbsBrep(brep,2)
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies:[{id:'exact',name:'Exact',brep,mesh}]}))
 await ui.click('Exact');const before=ui.doc();await ui.click('Body diagnostics');await ui.click('Inspect face contacts')
 const old=requests.at(-1)!
 const toggle=ui.all().find(n=>n.tag==='input'&&n.props.type==='checkbox'&&n.parent&&ui.text(n.parent).includes('Inspect within faces'))!
 toggle.props['onUpdate:modelValue'](true);await flushClearance()
 const latest=requests.at(-1)!;expect(latest.job.kind).toBe('selfIntersection');expect(latest.job.maxSpans).toBe(10000)
 old.resolve({pairs:[],contactPairCount:99});await flushClearance()
 expect(ui.text(ui.all()[0])).not.toContain('Contacts: 99')
 latest.resolve({pairs:[],absenceProven:false,faces:[{face:2,result:null}],contactPairCount:0,sharedBoundaryPairCount:0,unresolvedPairCount:1,unvisitedPairs:0});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Absence of self-intersections is unproven')
 expect(ui.text(ui.all()[0])).toContain('Unvisited or unproven faces: 3')
 expect(ui.doc()).toEqual(before)
})

it('hides rear faces of a closed B-rep in CPU view while preserving two-sided mesh display',async()=>{
 const closed=await mount({seedDocument:cylinderSeed()});await flushClearance()
 const count=(ui:Awaited<ReturnType<typeof mount>>)=>ui.all(ui.svg()).filter(n=>n.tag==='polygon'&&n.props['data-body']==='imported-cylinder').length
 const doc=closed.doc(),before=JSON.stringify(doc)
 expect(count(closed)).toBeGreaterThan(0)
 const meshOnly=structuredClone(doc);delete meshOnly.bodies[0].brep
 const open=await mount({seedDocument:meshOnly});await flushClearance()
 expect(count(open)).toBe(meshOnly.bodies[0].mesh.indices.length/3)
 expect(JSON.stringify(closed.doc())).toBe(before)
})

it.each(['variable','corner'])('locates an oversized %s fillet and recovers without losing history',async mode=>{
 const ui=await mount();await ui.click('Box');const before=ui.doc(),body=before.bodies.at(-1)!,brep=body.brep!
 const max=[0,1,2].map(axis=>Math.max(...brep.vertices.map(v=>v.point[axis])))
 const vertex=brep.vertices.findIndex(v=>v.point.every((x,i)=>x===max[i]))
 const indexes=mode==='corner'?brep.edges.flatMap((e,i)=>e.vertices.includes(vertex)?[i]:[]):[brep.edges.findIndex(e=>{const [a,b]=e.vertices.map(v=>brep.vertices[v].point);return a[0]===b[0]&&a[1]===b[1]})]
 await ui.click('Edges')
 for(const [i,index] of indexes.entries()){
  const edge=ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!
  edge.props.onPointerdown({...ui.event(edge),shiftKey:i>0});await nextTick()
 }
 await ui.click('Fillet 3D')
 ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Fillet type')!.props['onUpdate:modelValue'](mode);await flushClearance()
 quantityField(ui,mode==='variable'?'Radius A, mm':'Radius / size, mm').props['onUpdate:modelValue']('30');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(body.name+' · edges ')
 expect(ui.text(ui.all()[0])).toContain('Reduce the radius or chamfer size')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 for(const index of indexes)expect(ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[index])!.props.stroke).toBe('#f87171')
 quantityField(ui,mode==='variable'?'Radius A, mm':'Radius / size, mm').props['onUpdate:modelValue']('1');await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 await ui.click('Apply · Enter');const after=ui.doc();expect(after).not.toEqual(before)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(after)
})

it('renders material opacity and keeps geometry intact through history and reload',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Properties')
 const before=ui.doc(),body=before.bodies.at(-1)!
 const change=async(value:string)=>{ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Opacity')!.props.onChange({target:{value}});await flushClearance()}
 await change('0.35')
 expect(ui.doc().bodies.at(-1)!.material?.opacity).toBe(.35)
 expect(ui.doc().bodies.at(-1)!.brep).toEqual(body.brep)
 expect(ui.doc().bodies.at(-1)!.mesh).toEqual(body.mesh)
 const polygons=()=>ui.all(ui.svg()).filter(n=>n.tag==='polygon'&&n.props['data-body']===body.id)
 expect(polygons().length).toBeGreaterThanOrEqual(12);expect(polygons().every(n=>n.props.opacity===.35&&n.props.stroke==='none')).toBe(true)
 const after=ui.doc();await change('1.1');expect(ui.doc()).toEqual(after)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(after)
 const restored=await mount({},ui.serialized());expect(restored.doc()).toEqual(after)
})

it('maps CPU split fragments back to their original selectable face',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Properties')
 ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Opacity')!.props.onChange({target:{value:'0.35'}});await flushClearance()
 const before=ui.doc(),body=before.bodies.at(-1)!,faces=solidTopology(body.mesh).faces
 await ui.click('Faces');await flushClearance()
 const fragments=ui.all(ui.svg()).filter(n=>n.tag==='polygon'&&n.props['data-body']===body.id)
 expect(fragments.length).toBeGreaterThan(body.mesh.indices.length/3)
 for(const fragment of fragments){
  const triangle=Number(fragment.props['data-triangle']),expected=faces.findIndex(f=>f.triangles.includes(triangle))
  expect(expected).toBeGreaterThanOrEqual(0)
  ui.all().find(n=>n.props['aria-label']==='Select face')!.props['onUpdate:modelValue'](-1);await flushClearance()
  fragment.props.onPointerdown(ui.event(fragment));await flushClearance()
  const highlighted=ui.all(ui.svg()).filter(n=>n.tag==='polygon'&&n.props['data-body']===body.id&&String(n.props.style?.color).startsWith('hsl(40 '))
  expect(highlighted.length).toBeGreaterThan(0)
  expect([...new Set(highlighted.map(n=>Number(n.props['data-triangle'])))].sort((a,b)=>a-b)).toEqual([...faces[expected].triangles].sort((a,b)=>a-b))
 }
 expect(ui.doc()).toEqual(before)
})


it('reports approximate CPU transparency and clears the status after reducing the scene',async()=>{
 const {TransparentBsp}=await import('../src/services/transparentBsp')
 const ui=await mount();await ui.click('Box');await ui.click('Properties')
 const fail=vi.spyOn(TransparentBsp.prototype,'ordered').mockImplementation(()=>{throw Error('Transparency operation limit exceeded')})
 try{
  ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Opacity')!.props.onChange({target:{value:'0.35'}});await flushClearance()
  const before=ui.doc()
  expect(ui.all().some(n=>n.props['aria-label']==='transparency-limit')).toBe(true)
  expect(ui.all(ui.svg()).some(n=>n.props['data-body'])).toBe(true)
  await ui.click('Scene');await ui.click('Hide: '+before.bodies.at(-1)!.name)
  expect(ui.all().some(n=>n.props['aria-label']==='transparency-limit')).toBe(false)
  expect(ui.doc()).toEqual(before)
 }finally{fail.mockRestore()}
})

it('reuses CPU transparency geometry during orbit and invalidates it after scene and material edits',async()=>{
 const {TransparentBsp}=await import('../src/services/transparentBsp')
 const build=vi.spyOn(TransparentBsp.prototype as any,'partition')
 try{
  const ui=await mount();await ui.click('Box');await ui.click('Properties')
  const opacity=()=>ui.all().find(n=>n.tag==='input'&&n.props['aria-label']==='Opacity')!
  opacity().props.onChange({target:{value:'0.35'}});await flushClearance()
  const before=ui.doc(),svg=ui.svg(),down={...ui.event(svg,0,0),button:2}
  svg.props.onPointerdown(down);await flushClearance()
  const calls=build.mock.calls.length,points=()=>ui.all(svg).filter(n=>n.props['data-body']).map(n=>n.props.points)
  const initial=points()
  for(let i=1;i<=5;i++){svg.props.onPointermove({...ui.event(svg,i*10,i*4),button:2});await flushClearance()}
  expect(build.mock.calls.length).toBe(calls)
  expect(points()).not.toEqual(initial);expect(ui.doc()).toEqual(before)
  svg.props.onPointerup({...ui.event(svg,50,20),button:2});await flushClearance()
  const settled=build.mock.calls.length
  await ui.click('Box');await flushClearance()
  expect(build.mock.calls.length).toBeGreaterThan(settled)
  const edited=ui.doc();await ui.click('↶');expect(ui.doc()).toEqual(before)
  await ui.click('↷');expect(ui.doc()).toEqual(edited)
  await ui.click('Scene');const target=edited.bodies.at(-1)!;ui.all().find(n=>n.props['data-scene-key']==='object:'+target.id)!.children.find(n=>n.tag==='button')!.props.onClick({shiftKey:false});await flushClearance();await ui.click('Properties');const materialCalls=build.mock.calls.length
  opacity().props.onChange({target:{value:'0.5'}});await flushClearance()
  expect(build.mock.calls.length).toBeGreaterThan(materialCalls)
  expect(ui.doc().bodies).toHaveLength(edited.bodies.length);expect(ui.doc().bodies.find(b=>b.id===target.id)!.material?.opacity).toBe(.5)
 }finally{build.mockRestore()}
})

it('refreshes transparent world fragments after smooth refinement and orbit settle without changing the cylinder',async()=>{
 const {TransparentBsp}=await import('../src/services/transparentBsp'),{prepareSolidDisplay}=await import('../src/services/solidDisplayPreparation')
 const build=vi.spyOn(TransparentBsp.prototype as any,'partition'),requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 displayWorkerRun.mockImplementation(job=>new Promise(resolve=>requests.push({job,resolve})))
 try{
  const seed=cylinderSeed();seed.bodies[0].material={name:'Glass',color:'#2288dd',opacity:.35}
  const ui=await mount({seedDocument:seed});await flushClearance();const before=ui.doc(),svg=ui.svg()
  const geometry=()=>ui.all(svg).filter(n=>n.props['data-body']==='imported-cylinder').map(n=>n.props.points)
  const coarse=geometry(),calls=build.mock.calls.length;expect(requests).toHaveLength(1)
  requests[0].resolve(prepareSolidDisplay(requests[0].job.mesh,requests[0].job.brep));await flushClearance()
  expect(build.mock.calls.length).toBeGreaterThan(calls);const smooth=geometry();expect(smooth).not.toEqual(coarse)
  svg.props.onPointerdown({...ui.event(svg),button:2});await flushClearance();const moving=build.mock.calls.length
  svg.props.onPointermove({...ui.event(svg,20,10),button:2});await flushClearance();expect(build.mock.calls.length).toBe(moving)
  svg.props.onPointerup({...ui.event(svg,20,10),button:2});await flushClearance();expect(build.mock.calls.length).toBeGreaterThan(moving)
  expect(ui.doc()).toEqual(before)
  await ui.click('Smooth B-rep display');expect(geometry()).not.toEqual(smooth);expect(ui.doc()).toEqual(before)
 }finally{build.mockRestore()}
})

it('invalidates transparent world fragments for changed Push Pull preview and clears them on Escape',async()=>{
 const {TransparentBsp}=await import('../src/services/transparentBsp'),build=vi.spyOn(TransparentBsp.prototype as any,'partition')
 try{
  const ui=await mount();await ui.click('Cube');await ui.click('Properties')
  ui.all().find(n=>n.props['aria-label']==='Opacity')!.props.onChange({target:{value:'0.35'}});await flushClearance()
  const before=ui.doc();await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.props['data-body'])!)
  const calls=build.mock.calls.length;await ui.click('Push / Pull');await flushClearance()
  expect(build.mock.calls.length).toBeGreaterThan(calls)
  const preview=()=>ui.all(ui.svg()).filter(n=>n.props['data-preview-body']).map(n=>n.props.points)
  const initial=preview();expect(initial.length).toBeGreaterThan(0);const previewCalls=build.mock.calls.length
  quantityField(ui,'Distance, mm').props['onUpdate:modelValue']('3');await flushClearance()
  expect(build.mock.calls.length).toBeGreaterThan(previewCalls);expect(preview()).not.toEqual(initial);expect(ui.doc()).toEqual(before)
  await commandKey(ui,'Escape');expect(preview()).toEqual([]);expect(ui.doc()).toEqual(before)
  expect(ui.all(ui.svg()).some(n=>n.props['data-body']===before.bodies[0].id)).toBe(true)
 }finally{build.mockRestore()}
})

it('keeps working cylinder fragment lighting equal to the original face shade across orbit cameras',async()=>{
 const {directFaceShade}=await import('../src/services/directModelingTools')
 const seed=cylinderSeed();seed.bodies[0].material={name:'Glass',color:'#2288dd',opacity:.35}
 const ui=await mount({seedDocument:seed});await ui.click('Smooth B-rep display');await ui.click('Imported cylinder')
 const before=ui.doc(),svg=ui.svg(),initial=defaultDirectCamera()
 svg.props.onPointerdown({...ui.event(svg),button:2});await flushClearance()
 for(const [x,y] of [[0,0],[30,10],[-40,35],[85,-40]]){
  svg.props.onPointermove({...ui.event(svg,x,y),button:2});await flushClearance()
  const camera={yaw:initial.yaw+x*.007,pitch:Math.max(-1.5,Math.min(1.5,initial.pitch+y*.007))}
  const fragments=ui.all(svg).filter(n=>n.props['data-body']==='imported-cylinder');expect(fragments.length).toBeGreaterThan(0)
  for(const fragment of fragments){const shade=Number(String(fragment.props.style.color).match(/45% ([\d.]+)%/)?.[1]);expect(shade).toBe(directFaceShade(before.bodies[0].mesh,Number(fragment.props['data-triangle']),camera))}
 }
 svg.props.onPointerup({...ui.event(svg,85,-40),button:2});await flushClearance();expect(ui.doc()).toEqual(before)
})

it('explains how to fix an insufficient closed sweep section count and keeps the document unchanged',async()=>{
 const profile={id:'profile',name:'Profile',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1]}}
 const path={id:'path',name:'Path',curve:{degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0],[-1,1,0],[-1,0,0],[-1,-1,0],[0,-1,0],[1,-1,0],[1,0,0]],weights:Array.from({length:9},(_,i)=>i%2?Math.SQRT1_2:1)}}
 const ui=await mount({seedDocument:{version:1,bodies:[],sketches:[],curves:[profile,path]}})
 await ui.click('Profile');await ui.click('Path',true);const before=ui.doc();await ui.click('Sweep')
 ui.all().find(n=>n.props['aria-label']==='Sweep orientation')!.props['onUpdate:modelValue']('framed');await flushClearance()
 ui.all().find(n=>n.props['aria-label']==='Sweep sections')!.props['onUpdate:modelValue'](3);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Use at least 4 sections for a closed path.')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
})

it('prepares incompatible Coons weights with budget refusal, cancel and exact source history',async()=>{
 const points=[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=points.map((controlPoints,i)=>({id:`prepared-edge-${i}`,name:`Prepared edge ${i}`,curve:{degree:1,knots:[0,0,1,1],controlPoints,weights:i===0?[1,.5]:[1,1]}}))
 const ui=await mount({seedDocument:{version:1,bodies:[],sketches:[],curves}})
 for(let i=0;i<4;i++)await ui.click(`Prepared edge ${i}`,i>0)
 const before=ui.doc();await ui.click('Coons patch')
 await vi.waitFor(()=>expect(ui.all().some(n=>n.props['aria-label']==='Prepare patch boundary weights')).toBe(true))
 const enabled=ui.all().find(n=>n.props['aria-label']==='Prepare patch boundary weights')!
 enabled.props.onChange({target:{checked:true}});await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='patch-preparation')).toBe(true)
 expect(ui.doc()).toEqual(before)
 quantityField(ui,'Preparation budget, mm').props['onUpdate:modelValue'](0);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain('Boundary preparation exceeds the budget')
 expect(ui.text(ui.all()[0])).toContain('Bottom · vMin:')
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='patch-budget')).toHaveLength(1)
 expect(ui.button('Apply · Enter').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 quantityField(ui,'Preparation budget, mm').props['onUpdate:modelValue'](1e-6);await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='patch-budget')).toBe(false)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Coons patch');await commandKey(ui,'Enter')
 expect(ui.doc().surfaces).toHaveLength(1);expect(ui.doc().curves).toEqual(before.curves)
 const committed=ui.doc();await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('↷');expect(ui.doc()).toEqual(committed)
})

it('ignores stale Coons preparation bounds and errors after retry and cancellation',async()=>{
 const {buildSolidSurface}=await import('../src/services/solidSurfaceConstruction')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='surfaceBuild'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const points=[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=points.map((controlPoints,i)=>({id:`async-patch-${i}`,name:`Async boundary ${i}`,curve:{degree:1,knots:[0,0,1,1],controlPoints,weights:i===0?[1,.5]:[1,1]}}))
 const ui=await mount({seedDocument:{version:1,bodies:[],sketches:[],curves}})
 for(let i=0;i<4;i++)await ui.click(`Async boundary ${i}`,i>0)
 const before=ui.doc();await ui.click('Coons patch')
 await vi.waitFor(()=>expect(ui.all().some(n=>n.props['aria-label']==='Prepare patch boundary weights')).toBe(true))
 ui.all().find(n=>n.props['aria-label']==='Prepare patch boundary weights')!.props.onChange({target:{checked:true}});await flushClearance()
 quantityField(ui,'Preparation budget, mm').props['onUpdate:modelValue'](0);await flushClearance()
 quantityField(ui,'Preparation budget, mm').props['onUpdate:modelValue'](1e-6);await flushClearance()
 expect(requests).toHaveLength(4);expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 const latest=buildSolidSurface(requests[3].job.document,requests[3].job.options)
 requests[3].resolve(latest);await flushClearance()
 const diagnostic=ui.text(ui.all().find(n=>n.props['data-diagnostic']==='patch-preparation')!)
 requests[2].resolve(buildSolidSurface(requests[2].job.document,requests[2].job.options));await flushClearance()
 requests[1].resolve(buildSolidSurface(requests[1].job.document,requests[1].job.options));await flushClearance()
 requests[0].reject(Error('stale boundary failure'));await flushClearance()
 expect(ui.button('Apply · Enter').props.disabled).toBe(false)
 expect(ui.text(ui.all().find(n=>n.props['data-diagnostic']==='patch-preparation')!)).toBe(diagnostic)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(latest.document)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
 await ui.click('Coons patch');expect(requests).toHaveLength(5)
 await commandKey(ui,'Escape')
 requests[4].resolve(buildSolidSurface(requests[4].job.document,requests[4].job.options));await flushClearance()
 expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props['data-diagnostic']==='patch-preparation')).toBe(false)
})

it.each(['en','ru'])('recovers Coons roles and directions with local error markers (%s)',async locale=>{
 const points=[[[0,0,0],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,2,0]]]
 const curves=points.map((controlPoints,i)=>({id:`role-${i}`,name:`Role ${i}`,curve:{degree:1,knots:[0,0,1,1],controlPoints,weights:[1,1]}}))
 const ui=await mount({locale,seedDocument:{version:1,bodies:[],sketches:[],curves}})
 for(let i=0;i<4;i++)await ui.click(`Role ${i}`,i>0)
 const before=ui.doc();await ui.click('Coons patch')
 const topLabel=locale==='ru'?'Верх · vMax':'Top · vMax'
 const top=ui.all().find(n=>n.tag==='select'&&n.props['aria-label']===topLabel)!
 top.props.onKeydown({key:'Home',preventDefault(){},stopPropagation(){}});await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Для каждой роли выберите отдельную кривую.':'Choose a distinct curve for each role.')
 expect(ui.button(locale==='ru'?'Готово · Enter':'Apply · Enter').props.disabled).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 top.props.onKeydown({key:'ArrowDown',preventDefault(){},stopPropagation(){}});await flushClearance()
 const reverse=ui.all().find(n=>n.tag==='input'&&n.props.type==='checkbox'&&n.parent&&ui.text(n.parent).includes(locale==='ru'?'Развернуть: Справа':'Reverse: Right'))!
 reverse.props['onUpdate:modelValue'](true);await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Угол 2':'Corner 2')
 const markers=ui.all().find(n=>n.props['data-diagnostic']==='patch-gap')!
 expect(markers.children.filter(n=>n.tag==='circle')).toHaveLength(2)
 expect(ui.button(locale==='ru'?'Готово · Enter':'Apply · Enter').props.disabled).toBe(true)
 reverse.props['onUpdate:modelValue'](false);await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='patch-gap')).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc().surfaces).toHaveLength(1);expect(ui.doc().curves).toEqual(before.curves)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})


it('refuses a drag with pending snaps without materializing or cloning history geometry',async()=>{
 const {DirectHistory}=await import('../src/services/directModeling')
 bodySnapsWorkerRun.mockImplementation(()=>new Promise(()=>{}))
 const ui=await mount();await ui.click('Cube');const before=ui.doc(),svg=ui.svg()
 const gizmo=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 expect(gizmo).toBeDefined()
 const read=vi.spyOn(DirectHistory.prototype,'document','get')
 try{
  await ui.pointer(gizmo,0,0)
  expect(ui.text(ui.all()[0])).toContain('Body snaps are not ready')
  expect(read).not.toHaveBeenCalled();expect(ui.doc()).toEqual(before)
 }finally{read.mockRestore()}
})
it('revolves a retained holed profile in exact mode and preserves it through cancel and Undo',async()=>{
 await geometryKernel.warmGeometryKernel()
 const {authorBrepProfile}=await import('../src/services/geometry/brepProfile')
 const {withRetainedProfile}=await import('../src/services/retainedSketchProfile')
 const profile=authorBrepProfile({kind:'polygon',rings:[[[3,0],[6,0],[6,4],[3,4]],[[4,1],[4,3],[5,3],[5,1]]]})
 const sketch=withRetainedProfile({id:'retained',name:'Retained holes',closed:true,points:[]},profile)
 const ui=await mount({},JSON.stringify({version:1,bodies:[],sketches:[sketch]}))
 await ui.click(sketch.name);const before=ui.doc()
 await ui.click('Revolve')
 const geometry=ui.all().find(n=>n.tag==='select'&&n.parent&&ui.text(n.parent).startsWith('Revolve surfaces'))!
 expect(geometry.props['onUpdate:modelValue']).toBeDefined()
 expect(geometry.children.find(n=>n.tag==='option'&&n.props.value==='faceted')!.props.disabled).toBe(true)
 geometry.props.onKeydown({key:'Home',preventDefault(){},stopPropagation(){}})
 quantityField(ui,'Angle, °').props['onUpdate:modelValue']('90')
 await vi.waitFor(()=>expect(ui.button('Apply · Enter').props.disabled).toBe(false),{timeout:3000})
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 await ui.click('Revolve')
 await vi.waitFor(()=>expect(ui.button('Apply · Enter').props.disabled).toBe(false),{timeout:3000})
 await ui.click('Apply · Enter')
 expect(ui.doc().bodies[0].name).toMatch(/exact B-rep/)
 expect(ui.doc().bodies[0].brep!.faces.filter(face=>face.holes.length===1)).toHaveLength(2)
 expect(ui.doc().sketches[0].retainedProfile).toEqual(profile)
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it('marks rational profile crossings in both views and refuses Enter without changing source geometry',async()=>{
 await geometryKernel.warmGeometryKernel()
 const arch={id:'arch',name:'Rational arch',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,5],[2,0]],weights:[1,.8,1]}}
 const lines={id:'lines',name:'Closing lines',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}
 const ui=await mount({},JSON.stringify({version:1,sketches:[lines],curves:[arch],bodies:[]}))
 await ui.click(arch.name);await ui.click(lines.name,true);const before=ui.doc()
 await ui.click('Prepare profile');await flushClearance()
 const markers=ui.all().filter(n=>n.props['data-diagnostic']==='profile-curve-intersection')
 expect(markers).toHaveLength(4)
 expect(markers.every(n=>n.props['data-segments']==='0,2')).toBe(true)
 expect(ui.all().filter(n=>n.props['data-diagnostic']==='profile-curve-segment'&&n.props['data-status']==='intersection')).toHaveLength(4)
 expect(ui.text(ui.all()[0])).toContain('The marked segments intersect')
 expect(ui.text(ui.all()[0])).toContain('Self-intersections within a single curve are outside this check')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 await commandKey(ui,'Enter');expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.props['data-diagnostic']==='profile-curve-intersection')).toBe(false)
})

it('discards a late rational crossing diagnostic after Escape',async()=>{
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='profilePrepare'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const arch={id:'arch',name:'Late arch',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,5],[2,0]],weights:[1,.8,1]}}
 const lines={id:'lines',name:'Late lines',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}
 const ui=await mount({},JSON.stringify({version:1,sketches:[lines],curves:[arch],bodies:[]}))
 await ui.click(arch.name);await ui.click(lines.name,true);const before=ui.doc()
 await ui.click('Prepare profile');expect(requests).toHaveLength(1)
 await commandKey(ui,'Escape')
 const request=requests[0]!;request.resolve(prepareSolidProfile(request.job.document,request.job.ids,request.job.tolerance));await flushClearance()
 expect(ui.all().some(n=>n.props['data-diagnostic']==='profile-curve-intersection')).toBe(false)
 expect(ui.doc()).toEqual(before)
})


it.each(['en','ru'])('keeps the complete command registry identified and explains disabled actions in %s',async locale=>{
 const ui=await mount({locale})
 let ids:string[]=[]
 for(const selection of [null,'Cube','Profile']){
  if(selection)await ui.click(selection)
  const commands=ui.commands(),current=commands.map(c=>c.id)
  expect(commands).toHaveLength(96);expect(new Set(current).size).toBe(current.length)
  if(ids.length)expect(current).toEqual(ids);else ids=current
  for(const command of commands){
   expect(command.label.trim(),command.id).not.toBe('')
   if(command.enabled===false)expect(command.disabledReason?.trim(),command.id).toBeTruthy()
  }
 }
 expect(ids).toContain('repeat')
 expect(ids).toContain('scene-panel')
 for(const kind of ['box','wedge','cylinder','frustum','tube','cone','sphere','torus'])expect(ids).toContain('add-'+kind)
})

for(const locale of ['en','ru'])it('localizes source deletion and group deletion dependencies without changing history: '+locale,async()=>{
 await geometryKernel.warmGeometryKernel()
 const base=extrudeDirectSketch({id:'profile',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'source')
 const {createSolidInstance}=await import('../src/services/solidInstances')
 const seed=createSolidInstance({version:1,sketches:[],bodies:[{...base,name:'Source',group:'Sources'}],groups:[{name:'Sources',source:''}]},'source','linked',[[1,0,0,20],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 seed.bodies[1].name='Linked';seed.bodies[1].group='Copies'
 const ui=await mount({locale},stringifyMeshJson(seed)),before=ui.doc()
 await ui.click('Source');await ui.click(locale==='ru'?'Удалить':'Delete')
 expect(ui.doc()).toEqual(before)
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Нельзя удалить источник «Source»: остаются связанные экземпляры — 1':'Cannot delete source “Source”: linked instances remain — 1')
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Сделайте их независимыми или удалите вместе с источником.':'Make them independent or delete them together with the source.')
 await ui.click((locale==='ru'?'Удалить группу ':'Delete group ')+'Sources');expect(ui.doc()).toEqual(before)
 await ui.click('Linked');await ui.click(locale==='ru'?'Сделать независимым':'Make independent')
 const detached=ui.doc();expect(detached.bodies[1].instance).toBeUndefined()
 await ui.click((locale==='ru'?'Удалить группу ':'Delete group ')+'Sources')
 const removed=ui.doc();expect(removed.bodies.map(body=>body.id)).toEqual(['linked']);expect(removed.bodies[0]).toEqual(detached.bodies[1])
 await ui.click('↶');expect(ui.doc()).toEqual(detached)
 await ui.click('↷');expect(ui.doc()).toEqual(removed)
})

it('preserves 1000 linked instances on source refusal and restores deleting the entire group',async()=>{
 await geometryKernel.warmGeometryKernel()
 const source={...extrudeDirectSketch({id:'profile',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'source'),name:'Source',group:'Sources'}
 const bodies=[source,...Array.from({length:1000},(_,i)=>({id:'linked-'+i,name:'Linked '+i,group:'Copies',instance:{sourceId:source.id,matrix:[[1,0,0,(i%40)*20],[0,1,0,Math.floor(i/40)*20],[0,0,1,0],[0,0,0,1]]}}))]
 const ui=await mount({},stringifyMeshJson({version:1,sketches:[],bodies,groups:[{name:'Sources',source:''},{name:'Copies',source:''}]})),before=ui.serialized()
 await ui.click('Source');await ui.click('Delete')
 expect(ui.serialized()).toBe(before);expect(ui.text(ui.all()[0])).toContain('linked instances remain — 1000')
 await ui.click('Delete group Sources');expect(ui.serialized()).toBe(before)
 await ui.click('Active group: Copies');await ui.click('Move selection to group')
 const grouped=ui.serialized();expect(JSON.parse(grouped).bodies.every((body:any)=>body.group==='Copies')).toBe(true)
 await ui.click('Delete group Copies');expect(ui.doc().bodies).toEqual([])
 await ui.click('↶');expect(ui.serialized()).toBe(grouped)
 await ui.click('↷');expect(ui.doc().bodies).toEqual([])
})

for(const locale of ['en','ru'])it('locates invalid vertex and curvature input without worker calls: '+locale,async()=>{
 const seed=cylinderSeed(),ui=await mount({locale,seedDocument:seed});await ui.click('Imported cylinder')
 await ui.click(locale==='ru'?'Измерить вершины / ребро':'Measure vertices / edge')
 const field=(en:string,ru:string)=>ui.all().find(n=>n.props['aria-label']===(locale==='ru'?ru:en))!
 const before=ui.doc(),vertex=field('Vertex B','Вершина B'),calls=measurementWorkerRun.mock.calls.length
 vertex.props['onUpdate:modelValue'](9999);await flushClearance()
 expect(vertex.props['aria-invalid']).toBe(true);expect(vertex.props['aria-describedby']).toBe('vertex-measurement-error')
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Укажите существующую вершину B.':'Choose an existing vertex B.')
 expect(measurementWorkerRun.mock.calls.length).toBe(calls)
 vertex.props['onUpdate:modelValue'](2);await flushClearance()
 await ui.click(locale==='ru'?'Рёбра':'Edges')
 field('Select edge','Выбрать ребро').props.onChange({target:{value:String(seed.bodies[0].brep!.edges.findIndex(edge=>edge.curve.degree===2))}});await flushClearance()
 const parameter=field('Edge parameter','Параметр ребра'),edgeCalls=measurementWorkerRun.mock.calls.length
 for(const invalid of [-1,1.1,NaN,'']){
  parameter.props['onUpdate:modelValue'](invalid);await flushClearance()
  expect(parameter.props['aria-invalid']).toBe(true);expect(parameter.props['aria-describedby']).toBe('curvature-measurement-error')
  expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Задайте параметр ребра от 0 до 1.':'Set an edge parameter between 0 and 1.')
  expect(measurementWorkerRun.mock.calls.length).toBe(edgeCalls)
 }
 parameter.props['onUpdate:modelValue'](.25);await flushClearance();expect(parameter.props['aria-invalid']).toBe(false)
 const picker=field('Select edge','Выбрать ребро')
 picker.props.onKeydown({key:'Home',preventDefault(){},stopPropagation(){}});await flushClearance();expect(picker.props.value).toBe(-1)
 picker.props.onKeydown({key:'ArrowDown',preventDefault(){},stopPropagation(){}});await flushClearance();expect(picker.props.value).toBe(seed.bodies[0].brep!.edges.findIndex(edge=>edge.curve.degree===2))
 expect(ui.doc()).toEqual(before)
})

for(const locale of ['en','ru'])it('localizes current measurement worker failures and retries both reports: '+locale,async()=>{
 const {measureSolidVertices,measureSolidEdgeCurvature}=await import('../src/services/solidMeasurements')
 const ui=await mount({locale,seedDocument:cylinderSeed()});await ui.click('Imported cylinder');await ui.click(locale==='ru'?'Рёбра':'Edges')
 const picker=ui.all().find(n=>n.props['aria-label']===(locale==='ru'?'Выбрать ребро':'Select edge'))!
 picker.props.onKeydown({key:'ArrowDown',preventDefault(){},stopPropagation(){}});await flushClearance()
 let failVertices=true,failCurvature=true
 measurementWorkerRun.mockImplementation(async job=>{
  if(job.kind==='measureVertices'){if(failVertices){failVertices=false;return Promise.reject(Error('private vertex failure'))}return measureSolidVertices(job.a,job.indexA,job.b,job.indexB)}
  if(failCurvature){failCurvature=false;return Promise.reject(Error('private curvature failure'))}return measureSolidEdgeCurvature(job.body,job.edge,job.parameter)
 })
 const before=ui.doc();await ui.click(locale==='ru'?'Измерить вершины / ребро':'Measure vertices / edge')
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Не удалось измерить вершины.':'Could not measure these vertices.')
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Не удалось измерить кривизну.':'Could not measure curvature.')
 expect(ui.text(ui.all()[0])).not.toContain('private')
 await ui.click(locale==='ru'?'Повторить измерение вершин':'Retry vertex measurement')
 await ui.click(locale==='ru'?'Повторить измерение кривизны':'Retry curvature measurement')
 expect(ui.all().some(n=>n.props['data-measurement']==='distance')).toBe(true)
 expect(ui.all().some(n=>n.props['data-measurement']==='curvature')).toBe(true)
 expect(ui.doc()).toEqual(before)
})

for(const locale of ['en','ru'] as const)it('localizes mesh clearance failures and retries without edits: '+locale,async()=>{
 await geometryKernel.warmGeometryKernel()
 const body=(id:string,min:number[],max:number[])=>{const brep=createBrepBox(min,max);return {id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)}}
 const ui=await mount({locale},stringifyMeshJson({version:1,sketches:[],bodies:[body('Mesh A',[0,0,0],[10,10,10]),body('Mesh B',[13,0,0],[23,10,10])]}))
 await ui.click('Mesh A');await ui.click('Mesh B',true);const before=ui.doc()
 clearanceWorkerRun.mockRejectedValueOnce(new Error('Private mesh worker transport failure'))
 await ui.click(locale==='ru'?'Зазор двух тел по сетке':'Two-body mesh clearance');await flushClearance()
 const text=ui.text(ui.all()[0]);expect(text).toContain(locale==='ru'?'Не удалось вычислить зазор по сетке.':'Could not compute mesh clearance.');expect(text).not.toContain('Private mesh worker')
 expect(ui.all().some(n=>n.props['data-measurement']==='clearance')).toBe(false)
 await ui.click(locale==='ru'?'Повторить расчёт зазора':'Retry mesh clearance');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Зазор: 3.000000 mm':'Clearance: 3.000000 mm');expect(ui.all().some(n=>n.props['data-measurement']==='clearance')).toBe(true);expect(ui.doc()).toEqual(before)
 expect(ui.all().some(n=>n.tag==='button'&&ui.text(n)===(locale==='ru'?'Повторить расчёт зазора':'Retry mesh clearance'))).toBe(false)
})

for(const locale of ['en','ru'] as const)it('validates and retries surface boundary inspection: '+locale,async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount({locale},readFileSync('tests/fixtures/solid-surface-boundary.json','utf8'))
 await ui.click('Surface A');await ui.click('Surface B',true);const before=ui.doc()
 boundaryWorkerRun.mockRejectedValueOnce(new Error('Private surface boundary transport failure'))
 await ui.click(locale==='ru'?'Проверить стык поверхностей':'Inspect surface boundary');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Не удалось проверить стык поверхностей.':'Could not inspect the surface boundary.');expect(ui.text(ui.all()[0])).not.toContain('Private surface')
 await ui.click(locale==='ru'?'Повторить проверку стыка':'Retry surface boundary inspection');await flushClearance()
 expect(ui.all().filter(n=>n.props['data-boundary-inspection'])).toHaveLength(2)
 for(const [name,bad,good] of [[locale==='ru'?'Точек проверки':'Sample count',1,65],[locale==='ru'?'Допуск зазора, мм':'Gap tolerance, mm',0,.01],[locale==='ru'?'Допуск угла, °':'Angle tolerance, °',91,1]] as const){
  const field=ui.all().find(n=>n.props['aria-label']===name)!,requests=boundaryWorkerRun.mock.calls.length
  field.props['onUpdate:modelValue'](bad);await flushClearance();expect(field.props['aria-invalid']).toBe(true);expect(field.props['aria-describedby']).toBe('surface-boundary-error');expect(boundaryWorkerRun.mock.calls.length).toBe(requests);expect(ui.all().filter(n=>n.props['data-boundary-inspection'])).toHaveLength(0)
  field.props['onUpdate:modelValue'](good);await flushClearance();expect(ui.all().filter(n=>n.props['data-boundary-inspection'])).toHaveLength(2)
 }
 expect(ui.doc()).toEqual(before)
})

for(const locale of ['en','ru'] as const)it('retries surface distance after localized transport failure: '+locale,async()=>{
 await geometryKernel.warmGeometryKernel()
 const ui=await mount({locale},readFileSync('tests/fixtures/solid-surface-boundary.json','utf8'))
 await ui.click('Surface A');await ui.click('Surface B',true);const before=ui.doc()
 measurementWorkerRun.mockImplementationOnce(async(job:any)=>{expect(job.kind).toBe('surfaceDistance');throw new Error('Private surface distance transport failure')})
 await ui.click(locale==='ru'?'Расстояние между поверхностями':'Distance between surfaces');await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Не удалось измерить поверхности.':'Could not measure the surfaces.');expect(ui.text(ui.all()[0])).not.toContain('Private surface')
 await ui.click(locale==='ru'?'Повторить измерение поверхностей':'Retry surface distance');await flushClearance()
 expect(ui.all().some(n=>n.props['data-surface-distance']!==undefined)).toBe(true);expect(ui.doc()).toEqual(before)
})

it('creates an exact polyline contour through coordinate inputs and preserves history',async()=>{
 await geometryKernel.warmGeometryKernel();const ui=await mount();await ui.click('Polyline');const before=ui.doc()
 const field=(axis:string)=>ui.all().find(n=>n.props['aria-label']==='Point coordinate '+axis)!
 for(const [x,y] of [[0,0],[20,0],[20,10],[0,10]]){field('X').props['onUpdate:modelValue'](x);field('Y').props['onUpdate:modelValue'](y);await nextTick();await ui.click('Add point')}
 expect(ui.button('Add point').props.disabled).toBe(true);expect(ui.doc()).toEqual(before)
 await ui.click('Close contour');const after=ui.doc();expect(after.sketches).toHaveLength(before.sketches.length+1);expect(after.sketches.at(-1)!.points).toEqual([[0,0],[20,0],[20,10],[0,10]]);expect(after.sketches.at(-1)!.closed).toBe(true)
 await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
 await ui.click('Polyline');field('X').props['onUpdate:modelValue']('bad');await nextTick();expect(ui.button('Add point').props.disabled).toBe(true);expect(ui.doc()).toEqual(after)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after)
})

it('creates an exact circle from numeric inputs without committing its preview',async()=>{
 await geometryKernel.warmGeometryKernel();const ui=await mount();await commandKey(ui,'c');const before=ui.doc()
 const field=(name:string)=>ui.all().find(n=>n.props['aria-label']===name)!
 field('Point coordinate X').props['onUpdate:modelValue']('2 cm');field('Point coordinate Y').props['onUpdate:modelValue'](-5);field('Circle radius').props['onUpdate:modelValue']('6 mm');await nextTick()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-circle')).toBe(true)
 await ui.click('Create circle');const after=ui.doc();expect(after.sketches.at(-1)!.analytic).toEqual({kind:'circle',center:[20,-5],radius:6,start:0,sweep:360})
 await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
 await commandKey(ui,'c');field('Circle radius').props['onUpdate:modelValue']('bad');await nextTick();expect(ui.button('Create circle').props.disabled).toBe(true);expect(ui.all().some(n=>n.props['data-preview']==='numeric-circle')).toBe(false)
 field('Circle radius').props['onUpdate:modelValue']('9 mm');await nextTick();expect(ui.all().some(n=>n.props['data-preview']==='numeric-circle')).toBe(true)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after);expect(ui.all().some(n=>n.props['data-preview']==='numeric-circle')).toBe(false)
})

it.each([120,-120])('authors an exact directed arc with sweep %s and cancels preview',async(sweep)=>{
 await geometryKernel.warmGeometryKernel();const ui=await mount();await ui.click('Arc');const before=ui.doc()
 const field=(name:string)=>ui.all().find(n=>n.props['aria-label']===name)!
 field('Point coordinate X').props['onUpdate:modelValue']('2 cm');field('Point coordinate Y').props['onUpdate:modelValue'](-5);field('Arc radius').props['onUpdate:modelValue']('6 mm');field('Arc start angle').props['onUpdate:modelValue']('30 deg');field('Arc sweep').props['onUpdate:modelValue'](sweep);await nextTick()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-arc')).toBe(true)
 await ui.click('Create arc');const after=ui.doc();expect(after.sketches.at(-1)!.analytic).toEqual({kind:'arc',center:[20,-5],radius:6,start:30,sweep});expect(after.sketches.at(-1)!.closed).toBe(false)
 await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
 await ui.click('Arc');for(const invalid of ['bad',0,.01,-.05,360,-360]){field('Arc sweep').props['onUpdate:modelValue'](invalid);await nextTick();expect(ui.button('Create arc').props.disabled).toBe(true);expect(ui.all().some(n=>n.props['data-preview']==='numeric-arc')).toBe(false)}
 field('Arc sweep').props['onUpdate:modelValue'](-90);await nextTick();expect(ui.all().some(n=>n.props['data-preview']==='numeric-arc')).toBe(true);await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after);expect(ui.all().some(n=>n.props['data-preview']==='numeric-arc')).toBe(false)
})

it('creates a numeric rectangle with exact dimensions, preview, cancel and history',async()=>{
 const ui=await mount();await commandKey(ui,'r');const before=ui.doc()
 const field=(name:string)=>ui.all().find(n=>n.props['aria-label']===name)!
 field('Origin coordinate X').props['onUpdate:modelValue']('2 cm');field('Origin coordinate Y').props['onUpdate:modelValue'](-5);field('Rectangle size Width').props['onUpdate:modelValue']('10 mm');field('Rectangle size Height').props['onUpdate:modelValue'](6);await nextTick()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-rectangle')).toBe(true)
 await ui.click('Create rectangle');const after=ui.doc();expect(after.sketches.at(-1)!.points).toEqual([[20,-5],[30,-5],[30,1],[20,1]])
 await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
 await commandKey(ui,'r');for(const value of ['bad',0,-1]){field('Rectangle size Width').props['onUpdate:modelValue'](value);await nextTick();expect(ui.button('Create rectangle').props.disabled).toBe(true)}
 field('Rectangle size Width').props['onUpdate:modelValue'](10);await nextTick();await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after);expect(ui.all().some(n=>n.props['data-preview']==='numeric-rectangle')).toBe(false)
})
it('creates a retained numeric slot from its prepared preview and preserves history',async()=>{
 const ui=await mount();await ui.click('Slot');const before=ui.doc()
 const field=(name:string)=>ui.all().find(n=>n.props['aria-label']===name)!
 field('Origin coordinate X').props['onUpdate:modelValue'](0);field('Origin coordinate Y').props['onUpdate:modelValue'](0);field('End coordinate X').props['onUpdate:modelValue'](10);field('End coordinate Y').props['onUpdate:modelValue'](0);field('Slot width, mm').props['onUpdate:modelValue']('0.4 cm');await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-slot')).toBe(true)
 await ui.click('Create slot');const after=ui.doc();expect(after.sketches.at(-1)!.retainedProfile!.areaMm2).toBeCloseTo(40+4*Math.PI,10)
 await ui.click('↶');expect(ui.doc()).toEqual(before);await ui.click('↷');expect(ui.doc()).toEqual(after)
 await ui.click('Slot');field('End coordinate X').props['onUpdate:modelValue'](0);await flushClearance();expect(ui.button('Create slot').props.disabled).toBe(true);expect(ui.all().some(n=>n.props['data-preview']==='numeric-slot')).toBe(false)
 await commandKey(ui,'Escape');expect(ui.doc()).toEqual(after)
})
it('discards cancelled and superseded numeric slot preparation replies',async()=>{
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation');const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='profilePrepare'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Slot');const before=ui.doc();expect(requests).toHaveLength(1)
 const field=ui.all().find(n=>n.props['aria-label']==='End coordinate X')!;field.props['onUpdate:modelValue'](10);await flushClearance();expect(requests).toHaveLength(2)
 const reply=(r:typeof requests[number])=>prepareSolidProfile(r.job.document,r.job.ids,r.job.tolerance)
 requests[0].resolve(reply(requests[0]));await flushClearance();expect(ui.button('Create slot').props.disabled).toBe(true)
 await commandKey(ui,'Escape');requests[1].resolve(reply(requests[1]));await flushClearance();expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-slot')).toBe(false)
})
it.each(['en','ru'])('localizes numeric slot worker failure and recovers through retry in %s',async(locale)=>{
 let fail=true;previewWorkerRun.mockImplementation(job=>{if(job.kind==='profilePrepare'&&fail){fail=false;return Promise.reject(Error('PRIVATE TRANSPORT'))}return undefined})
 const ui=await mount({locale});await ui.click(locale==='ru'?'Паз':'Slot');const before=ui.doc()
 const expected=locale==='ru'?'Не удалось подготовить паз.':'Could not prepare the slot.'
 expect(ui.all().some(n=>n.props.role==='alert'&&ui.text(n).includes(expected))).toBe(true);expect(ui.all().some(n=>ui.text(n).includes('PRIVATE TRANSPORT'))).toBe(false)
 await ui.click(locale==='ru'?'Повторить расчёт паза':'Retry slot calculation');expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-slot')).toBe(true)
})

it('cancels automatic slot creation after a mouse drag while its profile worker is pending',async()=>{
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation');const requests:Array<{job:any;resolve:(value:any)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='profilePrepare'?new Promise(resolve=>requests.push({job,resolve})):undefined)
 const ui=await mount();await ui.click('Slot');const before=ui.doc(),svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 svg.props.onPointerdown({...ui.event(svg,40,-40),altKey:true});svg.props.onPointermove({...ui.event(svg,50,-40),altKey:true});svg.props.onPointerup({...ui.event(svg,50,-40),altKey:true});await flushClearance();expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Escape');for(const r of requests)r.resolve(prepareSolidProfile(r.job.document,r.job.ids,r.job.tolerance));await flushClearance()
 expect(ui.doc()).toEqual(before);expect(ui.all().some(n=>n.props['data-preview']==='numeric-slot')).toBe(false)
})

it.each([
 ['en','Select distinct open sketches or NURBS curves.','Remove duplicate inputs'],
 ['ru','Select distinct open sketches or NURBS curves.','Уберите повторяющиеся входы'],
 ['en','Profile input private-id must lie in the same sketch plane; control deviation exceeds 0.0000001 mm.','Control points deviate from the profile plane'],
 ['ru','Profile input private-id must lie in the same sketch plane; control deviation exceeds 0.0000001 mm.','Управляющие точки выходят из плоскости профиля'],
 ['en','Profile input private-id requires exactly clamped non-periodic endpoints.','NURBS endpoints must be clamped'],
 ['ru','Profile input private-id requires exactly clamped non-periodic endpoints.','Концы NURBS должны быть зажаты узлами'],
 ['en','Profile inputs must use the same sketch plane.','All lines must use one sketch plane'],
 ['ru','Profile inputs must use the same sketch plane.','Все линии должны использовать одну плоскость'],
 ['en','Profile preparation requires open polylines, arcs or NURBS curves.','Select open polylines, arcs or NURBS'],
 ['ru','Profile preparation requires open polylines, arcs or NURBS curves.','Выберите открытые ломаные, дуги или NURBS'],
])('localizes profile preparation refusal and retries without consuming inputs (%s, %s)',async(locale,message,visible)=>{
 await geometryKernel.warmGeometryKernel()
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const requests:Array<{job:any;resolve:(value:any)=>void;reject:(error:Error)=>void}>=[]
 previewWorkerRun.mockImplementation(job=>job.kind==='profilePrepare'?new Promise((resolve,reject)=>requests.push({job,resolve,reject})):undefined)
 const sketches=[{id:'first',name:'First chain',closed:false,points:[[0,0],[10,0],[10,6]]},{id:'second',name:'Second chain',closed:false,points:[[10,6],[0,6],[0,0]]}]
 const ui=await mount({locale},JSON.stringify({version:1,sketches,bodies:[]}))
 await ui.click('First chain');await ui.click('Second chain',true);const before=ui.doc()
 await ui.click(locale==='ru'?'Собрать профиль':'Prepare profile');await flushClearance()
 expect(requests).toHaveLength(1);requests[0]!.reject(Error(message));await flushClearance()
 expect(ui.text(ui.all()[0])).toContain(visible)
 expect(ui.text(ui.all()[0])).toContain('First chain, Second chain:')
 expect(ui.text(ui.all()[0])).not.toContain(message)
 expect(ui.doc()).toEqual(before)
 await ui.click(locale==='ru'?'Повторить вычисление':'Retry calculation');await flushClearance()
 expect(requests).toHaveLength(2);expect(requests[1]!.job).toEqual(requests[0]!.job)
 const latest=requests[1]!;latest.resolve(prepareSolidProfile(latest.job.document,latest.job.ids,latest.job.tolerance));await flushClearance()
 expect(ui.doc()).toEqual(before)
 await commandKey(ui,'Enter');expect(ui.doc().sketches).toHaveLength(1)
 expect(ui.doc().sketches[0].id).toBe('first')
 await ui.click('↶');expect(ui.doc()).toEqual(before)
})

it.each(['en','ru'])('blocks undersized analytic arc sweeps before worker dispatch in %s',async locale=>{
 await geometryKernel.warmGeometryKernel()
 const arc={id:'edit-arc',name:'Editable arc',closed:false,points:[],analytic:{kind:'arc',center:[0,0],radius:2,start:0,sweep:180}}
 const ui=await mount({locale},JSON.stringify(parseDirectDocument(JSON.stringify({version:1,sketches:[arc],bodies:[]}))));await ui.click(arc.name)
 await ui.click(locale==='ru'?'Параметры кривой':'Curve parameters');await flushClearance();const before=ui.doc()
 const field=quantityField(ui,locale==='ru'?'Угол дуги':'Arc sweep')
 for(const sweep of ['0','0.01','-0.05']){
  const jobs=previewWorkerRun.mock.calls.filter(([job])=>job.kind==='profileEdit').length
  field.props['onUpdate:modelValue'](sweep);await flushClearance()
  expect(ui.button(locale==='ru'?'Готово · Enter':'Apply · Enter').props.disabled).toBe(true)
  expect(ui.text(ui.all()[0])).toContain(locale==='ru'?'Модуль угла дуги':'Arc sweep magnitude')
  expect(previewWorkerRun.mock.calls.filter(([job])=>job.kind==='profileEdit').length).toBe(jobs)
  const requests=previewWorkerRun.mock.calls.length;await commandKey(ui,'Enter');expect(previewWorkerRun.mock.calls.length).toBe(requests);expect(ui.doc()).toEqual(before)
 }
 field.props['onUpdate:modelValue']('-0.1 deg');await flushClearance()
 expect(ui.button(locale==='ru'?'Готово · Enter':'Apply · Enter').props.disabled).toBe(false)
 await commandKey(ui,'Enter');expect(ui.doc().sketches[0].analytic?.sweep).toBe(-.1)
 await ui.click('↶');expect(parseDirectDocument(JSON.stringify(ui.doc()))).toEqual(parseDirectDocument(JSON.stringify(before)))
})


it('keeps partial annular mode preview-only and ignores a response after Escape',async()=>{
 const brep=createBrepTube(20,5,6),mesh=tessellateNurbsBrep(brep,2)
 const seed:DirectDocument={version:1,sketches:[],bodies:[{id:'annular',name:'Annular',brep,mesh}]}
 const ui=await mount({seedDocument:seed});await flushClearance();const before=ui.doc()
 await ui.click('Edges')
 const edge=ui.all(ui.svg()).find(n=>n.props['data-topology-edge']===brep.topologyIds!.edges[2])!
 edge.props.onPointerdown(ui.event(edge));await nextTick();await ui.click('Fillet 3D')
 let resolvePreview:(value:any)=>void=()=>{}
 let request:any
 previewWorkerRun.mockImplementation(job=>{
  if(job.kind!=='partialAnnularPreview')return undefined
  request=job;return new Promise(resolve=>{resolvePreview=resolve})
 })
 try {
  ui.all().find(n=>n.tag==='select'&&n.props['aria-label']==='Fillet type')!.props['onUpdate:modelValue']('partial-preview')
  await flushClearance()
  expect(request?.kind).toBe('partialAnnularPreview');expect(request.edge).toBe(2)
  expect(ui.text(ui.all()[0])).toContain('Preview only: geometry checks remain incomplete')
  expect(ui.button('Apply · Enter').props.disabled).toBe(true)
  await ui.click('Apply · Enter');expect(ui.doc()).toEqual(before)
  await ui.click('Esc')
  resolvePreview({body:seed.bodies[0],evidence:{qualification:{status:'preview-only',commitAllowed:false}}})
  await flushClearance();expect(ui.doc()).toEqual(before)
  expect(ui.all().some(n=>n.tag==='select'&&n.props['aria-label']==='Fillet type')).toBe(false)
 }finally {previewWorkerRun.mockReset()}
})
