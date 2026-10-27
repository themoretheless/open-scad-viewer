import {createRenderer,h,nextTick,shallowReactive,type Component} from 'vue'
import {afterEach,beforeEach,expect,it,vi} from 'vitest'
import ThermalStrengthEditor from '../src/features/ThermalStrengthEditor.vue'
import NominalTrussPanel from '../src/features/NominalTrussPanel.vue'
import type {MeshData} from '../src/core/mesh'
import type {PrintStrengthProfile,ThermalEditorState,ThermalStrengthResult,ValidatedPrintProfile} from '../src/services/trussScreening'
import type {NominalLatticeGraph} from '../src/services/latticeGraphProtocol'
import type {TrussResponse} from '../src/services/trussAnalysis'

const worker=vi.hoisted(()=>({
  thermal:vi.fn(),graph:vi.fn(),print:vi.fn(),catalog:vi.fn(),field:vi.fn(),scenario:vi.fn(),screening:vi.fn(),topology:vi.fn(),
}))
vi.mock('../src/services/mainSolidWorker',()=>({
  computeThermalStrength:worker.thermal,
  computeNominalLatticeGraph:worker.graph,
  computePrintStrengthProfile:worker.print,
  computeStructuralProfileCatalog:worker.catalog,
  computeTrussField:worker.field,
  computeTrussScenario:worker.scenario,
  computeTrussScreening:worker.screening,
  computeTrussTopology:worker.topology,
}))

class Node {
  parent:Node|null=null
  children:Node[]=[]
  props:Record<string,any>={}
  text=''
  value:any=''
  selected=false
  constructor(public tag:string){}
  get tagName(){return this.tag.toUpperCase()}
  get options(){return this.children.filter(node=>node.tag==='option')}
  getRootNode(){return {activeElement:null}}
  addEventListener(){}
  removeEventListener(){}
}
const renderer=createRenderer<Node,Node>({
  createElement:tag=>new Node(tag),
  createText:text=>{const node=new Node('#text');node.text=text;return node},
  createComment:()=>new Node('#comment'),
  setText:(node,text)=>{node.text=text},
  setElementText:(node,text)=>{node.children=[];node.text=text},
  parentNode:node=>node.parent,
  nextSibling:node=>node.parent?.children[node.parent.children.indexOf(node)+1]??null,
  patchProp:(node,key,_old,value)=>{node.props[key]=value;if(key==='value')node.value=value},
  insert:(node,parent,anchor)=>{
    if(node.parent)node.parent.children=node.parent.children.filter(child=>child!==node)
    node.parent=parent
    const index=anchor?parent.children.indexOf(anchor):-1
    index<0?parent.children.push(node):parent.children.splice(index,0,node)
  },
  remove:node=>{if(node.parent)node.parent.children=node.parent.children.filter(child=>child!==node);node.parent=null},
  setScopeId:()=>{},
  insertStaticContent:(content,parent,anchor)=>{
    const node=new Node('#static');node.text=String(content)
    if(node.parent)node.parent.children=node.parent.children.filter(child=>child!==node)
    node.parent=parent
    const index=anchor?parent.children.indexOf(anchor):-1
    index<0?parent.children.push(node):parent.children.splice(index,0,node)
    return [node,node]
  },
})

const unmounts:Array<()=>void>=[]
async function mount(component:Component,initial:Record<string,unknown>){
  const props=shallowReactive({...initial}),states:ThermalEditorState[]=[]
  const root=new Node('root'),app=renderer.createApp({setup:()=>()=>h(component,{...props,onChange:(state:ThermalEditorState)=>states.push(state)})})
  let mounted=true
  const unmount=()=>{if(!mounted)return;mounted=false;app.unmount();const index=unmounts.indexOf(unmount);if(index>=0)unmounts.splice(index,1)}
  app.mount(root);unmounts.push(unmount);await nextTick()
  const all=(node:Node=root):Node[]=>[node,...node.children.flatMap(all)]
  const text=(node:Node=root):string=>node.text+node.children.map(child=>text(child)).join('')
  const button=(name:string)=>{const node=all().find(node=>node.tag==='button'&&text(node)===name);expect(node,`button ${name}`).toBeDefined();return node!}
  const input=(label:string)=>{const node=all().find(node=>node.props['aria-label']===label);expect(node,`input ${label}`).toBeDefined();return node!}
  const field=(label:string)=>{
    const container=all().find(node=>node.tag==='label'&&text(node)===label)
    expect(container,`field ${label}`).toBeDefined()
    const control=all(container!).find(node=>['input','select'].includes(node.tag))
    expect(control,`control ${label}`).toBeDefined();return control!
  }
  const edit=async(node:Node,value:unknown)=>{node.props['onUpdate:modelValue'](value);await nextTick()}
  const editField=(label:string,value:unknown)=>edit(field(label),value)
  const press=(name:string)=>{const node=button(name);expect(node.props.disabled).toBeFalsy();return node.props.onClick({}) as Promise<void>|undefined}
  const click=async(name:string)=>{await press(name);await nextTick()}
  return {props,states,all,text,button,input,field,edit,editField,press,click,unmount}
}
async function flush(){for(let i=0;i<4;i++)await Promise.resolve();await nextTick()}
function deferred<T>(){
  let resolve!:(value:T)=>void,reject!:(reason?:unknown)=>void
  const promise=new Promise<T>((res,rej)=>{resolve=res;reject=rej})
  return {promise,resolve,reject}
}

const printProfile=():PrintStrengthProfile=>({material:'PETG',grade:'Synthetic specimen',propertySource:'Measured table',nozzleMm:.4,lineWidthMm:.45,layerHeightMm:.2,nozzleTempC:240,bedTempC:80})
const validated=(profile:PrintStrengthProfile):ValidatedPrintProfile=>({profile:{...profile},warnings:[],propertyModel:'user-supplied-isotropic-axial'})
const thermalResult=():ThermalStrengthResult=>({modelKind:'measured-bilinear-isotropic-axial-v1',youngMpa:1500,tensionMpa:30,compressionMpa:45,nozzleBracketC:[240,240],serviceBracketC:[20,60]})
const graph=():NominalLatticeGraph=>({modelKind:'nominal-bounding-box-axial',nodes:[[0,0,0],[10,10,10]],edges:[[0,1]]})
const trussResponse=():TrussResponse=>({displacementsMm:[[0,0,0],[.25,0,0]],reactionsN:[[-100,0,0],[0,0,0]],axialForcesN:[100],axialStressesMpa:[50],maxDeflectionMm:.25,maxRelativeResidual:0,freeDofs:1})
const scenarioResult=()=>({activeId:'case-1',terms:[{caseId:'case-1',factor:1}],model:{nodesMm:[[0,0,0],[10,10,10]] as [number,number,number][],members:[{nodes:[0,1] as [number,number],youngMpa:2000,areaMm2:3}],restrained:Array.from({length:2},()=>[false,false,false]),loads:[{nodes:[],originMm:[0,0,0] as [number,number,number],forceN:[0,0,0] as [number,number,number],momentNmm:[0,0,0] as [number,number,number]}]},result:trussResponse()})
const mesh=():MeshData=>({vertices:new Float32Array([0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1,0,0]),indices:new Uint32Array([0,1,2]),transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])} as MeshData)
const options={pattern:'spatial' as const,axis:'z' as const,cell:10,rib:1,rim:1,bottom:1,top:1,seed:1,jitter:0,lineWidth:.45,perimeters:2}
const printSettings={nozzle:.4,layer:.2,lines:2,skinLayers:2,maxBridge:5,openTop:false}

beforeEach(()=>{
  vi.clearAllMocks()
  worker.graph.mockResolvedValue(graph())
  worker.print.mockImplementation(async(profile:PrintStrengthProfile)=>validated(profile))
  worker.catalog.mockResolvedValue([])
  worker.field.mockResolvedValue([])
  worker.scenario.mockResolvedValue(scenarioResult())
  worker.screening.mockResolvedValue([])
  worker.topology.mockResolvedValue(null)
  worker.thermal.mockResolvedValue(thermalResult())
  vi.stubGlobal('Document',class {})
  vi.stubGlobal('ShadowRoot',class {})
  vi.stubGlobal('document',{activeElement:null,createElement:()=>({click(){},set href(_value:string){},set download(_value:string){}})})
  vi.stubGlobal('URL',{createObjectURL:()=> 'blob:mock',revokeObjectURL(){}})
})
afterEach(()=>{unmounts.splice(0).forEach(unmount=>unmount());vi.unstubAllGlobals()})

async function populateThermal(ui:Awaited<ReturnType<typeof mount>>){
  await ui.edit(ui.all().find(node=>node.tag==='input'&&node.props.type==='checkbox')!,true)
  await ui.editField('Part service temperature, °C',40)
  const values=[
    {nozzleTempC:240,serviceTempC:20,youngMpa:2000,tensionMpa:40,compressionMpa:60},
    {nozzleTempC:240,serviceTempC:60,youngMpa:1000,tensionMpa:20,compressionMpa:30},
  ]
  for(const [index,sample] of values.entries())for(const [key,value] of Object.entries(sample))await ui.edit(ui.input(`Sample ${index+1} ${key}`),value)
}

it('thermal editor exposes pending state, aborts superseded work and ignores stale success',async()=>{
  const requests:Array<ReturnType<typeof deferred<ThermalStrengthResult>>>=[]
  worker.thermal.mockImplementation(()=>{const pending=deferred<ThermalStrengthResult>();requests.push(pending);return pending.promise})
  const ui=await mount(ThermalStrengthEditor,{profile:printProfile(),locale:'en'})
  expect(ui.states.at(-1)).toEqual({enabled:false,evaluation:null,error:''})
  await populateThermal(ui)
  expect(worker.thermal).not.toHaveBeenCalled()
  await ui.click('Bind measurements to current profile')
  expect(ui.text()).toContain('Evaluating properties')
  expect(ui.states.at(-1)).toMatchObject({enabled:true,evaluation:null,error:''})
  expect(worker.thermal).toHaveBeenCalledTimes(1)
  const firstSignal=worker.thermal.mock.calls[0][1].signal as AbortSignal
  await ui.edit(ui.input('Sample 1 youngMpa'),1800)
  expect(firstSignal.aborted).toBe(true)
  expect(worker.thermal).toHaveBeenCalledTimes(2)
  requests[0].resolve({...thermalResult(),youngMpa:9999});await flush()
  expect(ui.text()).not.toContain('9999')
  expect(ui.states.at(-1)).toMatchObject({enabled:true,evaluation:null})
  requests[1].resolve(thermalResult());await flush()
  expect(ui.states.at(-1)).toMatchObject({enabled:true,error:'',evaluation:{result:{youngMpa:1500}}})
  expect(ui.text()).toContain('E = 1500 MPa')
  expect(ui.text()).not.toContain('Evaluating properties')

  await ui.editField('Part service temperature, °C',41)
  const finalSignal=worker.thermal.mock.calls[2][1].signal as AbortSignal
  const emitted=ui.states.length
  ui.unmount()
  expect(finalSignal.aborted).toBe(true)
  requests[2].resolve(thermalResult());await flush()
  expect(ui.states).toHaveLength(emitted)
})

it('thermal editor reports cancellation of the current request without publishing properties',async()=>{
  worker.thermal.mockImplementation(async()=>{throw Object.assign(new Error('cancelled'),{name:'AbortError'})})
  const ui=await mount(ThermalStrengthEditor,{profile:printProfile(),locale:'en'})
  await populateThermal(ui)
  await ui.click('Bind measurements to current profile');await flush()
  expect(ui.states.at(-1)).toEqual({enabled:true,evaluation:null,error:'Property evaluation was cancelled.'})
  expect(ui.text()).toContain('Property evaluation was cancelled.')
  expect(ui.text()).not.toContain('Interpolated properties')
})

it('nominal panel sequences profile and solve jobs and discards an edit-invalidated solution',async()=>{
  const firstPrint=deferred<ValidatedPrintProfile>(),pendingScenario=deferred<ReturnType<typeof scenarioResult>>()
  worker.print.mockImplementationOnce(()=>firstPrint.promise)
  worker.scenario.mockImplementationOnce(()=>pendingScenario.promise)
  const ui=await mount(NominalTrussPanel,{meshes:[mesh()],selection:[0],source:'cube(10);',ready:true,locale:'en',options,printSettings})
  await ui.click('Generate graph');await flush()
  await ui.edit(ui.input('Plastic type'),'PETG')
  await ui.editField('Grade / manufacturer / composition','Synthetic specimen')
  await ui.editField('Source of E and limits for these conditions','Measured table')
  await ui.editField('Nozzle temperature, °C',240)
  await ui.editField('Bed temperature, °C',80)
  await ui.editField('Young modulus, MPa',2000)
  await ui.editField('Member area, mm²',2)

  const stale=ui.press('Solve model');await nextTick()
  expect(worker.print).toHaveBeenCalledTimes(1)
  expect(worker.scenario).not.toHaveBeenCalled()
  expect(ui.text()).toContain('Computing')
  const staleSignal=worker.print.mock.calls[0][1].signal as AbortSignal
  await ui.editField('Member area, mm²',3)
  expect(staleSignal.aborted).toBe(true)
  firstPrint.resolve(validated(printProfile()));await stale;await flush()
  expect(worker.scenario).not.toHaveBeenCalled()
  expect(ui.text()).not.toContain('Nominal axial model')

  const solving=ui.press('Solve model');await flush()
  expect(worker.print).toHaveBeenCalledTimes(2)
  expect(worker.scenario).toHaveBeenCalledTimes(1)
  expect(worker.print.mock.invocationCallOrder[1]).toBeLessThan(worker.scenario.mock.invocationCallOrder[0])
  const freshSignal=worker.print.mock.calls[1][1].signal as AbortSignal
  expect(worker.scenario.mock.calls[0][1].signal).toBe(freshSignal)
  expect(worker.scenario.mock.calls[0][0].members[0]).toMatchObject({youngMpa:2000,areaMm2:3})
  pendingScenario.resolve(scenarioResult());await solving;await flush()
  expect(ui.text()).toContain('Nominal axial model')
  expect(ui.text()).toContain('0.25')
})
