import {createRenderer,nextTick} from 'vue'
import {it,expect,vi,afterEach} from 'vitest'
import MainModelingOverlay from '../src/features/MainModelingOverlay.vue'
import MainSketchTools from '../src/features/MainSketchTools.vue'
import CadWorkbenchPanel from '../src/features/CadWorkbenchPanel.vue'
import {previewMeshes} from '../src/services/mainModeling'
import {extrudeDirectSketch,type DirectDocument} from '../src/services/directModeling'
import {sampleCurve} from '../src/services/directSketchGeometry'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
class Node {
 parent:Node|null=null;children:Node[]=[];props:Record<string,any>={};style:Record<string,any>={};text='';value:any='';selected=false
 constructor(public tag:string){}
 get tagName(){return this.tag.toUpperCase()}
 get options(){return this.children.filter(n=>n.tag==='option')}
 get ownerSVGElement():Node|null{return this.tag==='svg'?this:this.parent?.ownerSVGElement??null}
 getBoundingClientRect(){return {left:0,top:0,width:1000,height:1000}}
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
afterEach(()=>{mounts.splice(0).forEach(f=>f());vi.unstubAllGlobals()})

async function mount(component:any,props:any){
 vi.stubGlobal('localStorage',{getItem:()=>null,setItem:()=>{}})
 vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {});vi.stubGlobal('document',{activeElement:null});vi.stubGlobal('window',{document:{activeElement:null}})
 const root=new Node('root'),app=renderer.createApp(component,props);app.mount(root);mounts.push(()=>app.unmount());await nextTick()
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)],text=(n:Node):string=>n.text+n.children.map(text).join('')
 const event=(n:Node,x=0,y=0)=>({button:0,target:n,currentTarget:n,clientX:x,clientY:y,pointerId:1,preventDefault(){},stopPropagation(){}})
 return {all,text,event,click:async(name:string)=>{const n=all().find(n=>n.tag==='button'&&text(n)===name)!;n.props.onClick({});await nextTick()}}
}
const body=extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'0')
const parameters={amount:2,x:0,y:0,z:0,axis:'z',edge:0,shape:'rectangle',width:2,height:2,cut:false}
it('fits a bounded nominal opening and preserves geometry when print constraints conflict',async()=>{
 const ui=await mount(CadWorkbenchPanel,{meshes:[],selection:[],hit:null,source:'',ready:true,locale:'en',initialAction:'lighten'})
 const input=(label:string)=>ui.all().find(n=>n.tag==='label'&&ui.text(n).startsWith(label))!.children.find(n=>n.tag==='input')!
 const set=async(label:string,value:unknown)=>{input(label).props['onUpdate:modelValue'](value);await nextTick()}
 await set('Limit nominal opening',true)
 await set('Bridge limit',5)
 await ui.click('Fit geometry to print settings')
 expect(input('Cell, mm').value).toBeCloseTo(6.35,12)
 expect(input('Rib, mm').value).toBe(1.35)
 await set('Bridge limit',1)
 await ui.click('Fit geometry to print settings')
 expect(ui.all().find(n=>n.props.role==='alert')?.text).toContain('incompatible')
 expect(input('Cell, mm').value).toBeCloseTo(6.35,12)
 await set('Bridge limit','')
 expect(input('Cell, mm').value).toBeCloseTo(6.35,12)
 await set('Bridge limit',4)
 await ui.click('Fit geometry to print settings')
 expect(input('Cell, mm').value).toBeCloseTo(5.35,12)
 expect(ui.all().some(n=>n.props.role==='alert')).toBe(false)
})
it('offers centered lattice patterns with spatial controls and no ineffective randomization',async()=>{
 const ui=await mount(CadWorkbenchPanel,{meshes:[],selection:[],hit:null,source:'',ready:true,locale:'en',initialAction:'lighten'})
 const select=ui.all().find(n=>n.tag==='select'&&n.options.some(o=>o.value==='bcc'))!
 expect(select).toBeDefined()
 expect(select.options.some(o=>o.value==='octet')).toBe(true)
 expect(select.options.some(o=>o.value==='isogrid')).toBe(true)
 for(const pattern of ['bcc','octet']){
  select.props['onUpdate:modelValue'](pattern);await nextTick()
  const labels=ui.all().filter(n=>n.tag==='label').map(ui.text)
  expect(labels.some(s=>s.includes('Outer skin, mm'))).toBe(true)
  expect(labels.some(s=>s.includes('Sampling step, mm'))).toBe(true)
  expect(labels.some(s=>s.includes('Channel axis'))).toBe(false)
  expect(labels.some(s=>s.includes('Randomness'))).toBe(false)
  expect(ui.all().some(n=>n.tag==='button'&&ui.text(n)==='Next variation')).toBe(false)
 }
 select.props['onUpdate:modelValue']('spatial');await nextTick()
 expect(ui.all().some(n=>n.tag==='label'&&ui.text(n).includes('Randomness'))).toBe(true)
 select.props['onUpdate:modelValue']('grid');await nextTick()
 expect(ui.all().some(n=>n.tag==='label'&&ui.text(n).includes('Channel axis'))).toBe(true)
 select.props['onUpdate:modelValue']('isogrid');await nextTick()
 expect(ui.all().some(n=>n.tag==='label'&&ui.text(n).includes('Channel axis'))).toBe(true)
 expect(ui.all().some(n=>n.tag==='label'&&ui.text(n).includes('Outer skin, mm'))).toBe(false)
})
it('drags a main viewport gizmo and commits once on release',async()=>{
 const values:any[]=[],preview=vi.fn(),apply=vi.fn()
 const ui=await mount(MainModelingOverlay,{meshes:previewMeshes({version:1,sketches:[],bodies:[body]}),selected:0,selection:[0],hit:null,operation:'move',parameters,project:(p:number[])=>[p[0]*10,p[1]*10],revision:0,box:false,onParameters:(p:any)=>values.push(p),onPreview:preview,onApply:apply})
 const svg=ui.all().find(n=>n.tag==='svg')!,g=ui.all().find(n=>n.tag==='g'&&n.props.onPointerdown)!
 g.props.onPointerdown(ui.event(g,0,0));svg.props.onPointermove(ui.event(svg,20,0));expect(values.at(-1).x).toBeCloseTo(2);expect(preview).toHaveBeenCalledTimes(1);expect(apply).not.toHaveBeenCalled();svg.props.onPointerup();expect(apply).toHaveBeenCalledTimes(1)
})
it('draws and extrudes a sketch directly on the main workplane',async()=>{
 const emitted:any[]=[],ui=await mount(MainSketchTools,{plane:null,project:(p:number[])=>[p[0],p[1]],ray:(x:number,y:number)=>({origin:[x,y,100],direction:[0,0,-1]}),revision:0,locale:'en',onBody:(b:any)=>emitted.push(b)})
 await ui.click('rectangle');const svg=ui.all().find(n=>n.tag==='svg')!
 svg.props.onPointerdown(ui.event(svg,0,0));svg.props.onPointermove(ui.event(svg,10,10));svg.props.onPointerup();await nextTick();await ui.click('Extrude')
 expect(emitted).toHaveLength(1);expect(inspectPolygonMesh(emitted[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
})
