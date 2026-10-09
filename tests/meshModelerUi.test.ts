import {createRenderer, nextTick} from 'vue'
import {expect, it, vi} from 'vitest'
import MeshModeler from '../src/features/MeshModeler.vue'

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
  return {projectMeshModelerFaces:(...args:Parameters<typeof actual.projectMeshModelerFaces>)=>{
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
