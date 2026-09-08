import {createRenderer,nextTick} from 'vue'
import {it,expect,vi,afterEach} from 'vitest'
import DirectModeler from '../src/features/DirectModeler.vue'
import {extrudeDirectSketch,type DirectDocument} from '../src/services/directModeling'
import {sampleCurve} from '../src/services/directSketchGeometry'
import {inspectPolygonMesh} from '../src/services/polygonKernel'
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
afterEach(()=>{mounts.splice(0).forEach(f=>f());vi.unstubAllGlobals()})
async function mount(props: Record<string,unknown> = {}){
 const sketch={id:'s',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]] as [number,number][]}
 let stored=JSON.stringify({version:1,sketches:[sketch,{id:'circle',name:'Circle',closed:true,analytic:{kind:'circle',center:[20,5],radius:3,start:0,sweep:360},points:sampleCurve({kind:'circle',center:[20,5],radius:3,start:0,sweep:360})},{id:'line',name:'Line',closed:false,points:[[0,-5],[2,-5]]},{id:'boundary',name:'Boundary',closed:false,points:[[5,-10],[5,0]]}],bodies:[{...extrudeDirectSketch(sketch,10,'b'),name:'Cube'}]})
 vi.stubGlobal('localStorage',{getItem:()=>stored,setItem:(_k:string,v:string)=>{stored=v}})
 vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {});vi.stubGlobal('document',{activeElement:null});vi.stubGlobal('window',{document:{activeElement:null}});vi.stubGlobal('SVGSVGElement',Node)
 vi.stubGlobal('DOMPoint',class {constructor(public x:number,public y:number){}matrixTransform(){return this}})
 const root=new Node('root'),app=renderer.createApp(DirectModeler,{open:true,locale:'en',canAppend:true,remainingSource:100000,...props});app.mount(root);mounts.push(()=>app.unmount());await nextTick()
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)]
 const text=(n:Node):string=>n.text+n.children.map(text).join('')
 const button=(name:string)=>{const n=all().find(n=>n.tag==='button'&&text(n)===name);if(!n)throw Error('Missing button '+name);return n}
 const click=async(name:string,shiftKey=false)=>{button(name).props.onClick({shiftKey});await nextTick()}
 const svg=()=>all().find(n=>n.tag==='svg'&&n.props['aria-label']==='3D body canvas')!
 const event=(n:Node,x=0,y=0)=>({button:0,target:n,currentTarget:n,clientX:x,clientY:y,pointerId:1,preventDefault(){},stopPropagation(){}})
 const pointer=async(n:Node,x=0,y=0)=>{n.props.onPointerdown(event(n,x,y));await nextTick()}
 return {all,text,button,click,svg,pointer,event,doc:()=>JSON.parse(stored) as DirectDocument,field:async(value:number)=>{const input=all().find(n=>n.tag==='input'&&n.props['onUpdate:modelValue']&&n.props.step===.5)??all().find(n=>n.tag==='input'&&n.props.type==='number'&&n.props['onUpdate:modelValue']);if(!input)throw Error('Missing field');input.props['onUpdate:modelValue'](value);await nextTick()}}
}
it('binds face selection, Push/Pull preview, confirm and undo to the document',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces');const polygon=ui.all(ui.svg()).find(n=>n.tag==='polygon')!;await ui.pointer(polygon)
 await ui.click('Push / Pull');expect(ui.doc().bodies[0].mesh).toBeDefined();await ui.click('Apply · Enter')
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1200)
 await ui.click('↶');expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
})
it('binds an edge selection to chamfer and creates a shell with the selected opening',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Edges');const edge=ui.all(ui.svg()).find(n=>n.tag==='polyline'&&n.props.onPointerdown)!;await ui.pointer(edge)
 await ui.click('Chamfer 3D');await ui.click('Apply · Enter');expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(980)
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Shell');await ui.click('Apply · Enter')
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000-6*6*8)
})
it('binds splitting and Shift selection to shared transforms',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Split');await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(2)
 const names=ui.doc().bodies.map(b=>b.name);await ui.click(names[0]);await ui.click(names[1],true);await ui.click('Transform selection');await ui.click('Esc');expect(ui.doc().bodies).toHaveLength(2)
 await ui.click('Duplicate · ⌘/Ctrl D');expect(ui.doc().bodies).toHaveLength(4)
})
it('binds analytic radius editing and offset without baking the curve',async()=>{
 const ui=await mount();await ui.click('Circle');await ui.click('Curve parameters')
 const inputs=ui.all().filter(n=>n.tag==='input'&&n.props['onUpdate:modelValue']);const radius=inputs.find(n=>n.parent&&ui.text(n.parent).startsWith('Radius / size, mm'))!
 radius.props['onUpdate:modelValue'](5);await nextTick();await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(5)
 await ui.click('Offset');await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(7)
 await ui.click('↶');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(5)
})
it('binds extend and trim to the selected 2D edge',async()=>{
 const ui=await mount();await ui.click('Line');await ui.click('Extend');await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='line')?.points.at(-1)).toEqual([5,-5])
 await ui.click('✂ Trim');const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!,path=ui.all(svg).find(n=>n.tag==='path'&&n.props.d==='M 0,5 L 5,5')!
 await ui.pointer(path,2,5);expect(ui.doc().sketches.some(s=>s.id==='line')).toBe(false)
 await ui.click('↶');expect(ui.doc().sketches.find(s=>s.id==='line')?.points.at(-1)).toEqual([5,-5])
})
it('creates a sketch on the selected face and preserves its plane through extrusion',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Sketch on face')
 const svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 await ui.pointer(svg,1,-1);svg.props.onPointermove(ui.event(svg,3,-3));svg.props.onPointerup(ui.event(svg,3,-3));await nextTick()
 const sketch=ui.doc().sketches.at(-1)!;expect(sketch.plane).toBeDefined();expect(sketch.points).toHaveLength(4)
 await ui.click('Extrude · E');await new Promise(resolve=>setTimeout(resolve,80));await nextTick();await ui.click('Apply · Enter')
 expect(ui.doc().bodies).toHaveLength(2);expect(inspectPolygonMesh(ui.doc().bodies[1].mesh).signedVolumeMm3).toBeCloseTo(40)
})
it('commits a gizmo drag once and undoes it',async()=>{
 const ui=await mount();await ui.click('Cube');const svg=ui.svg(),gizmo=ui.all(svg).find(n=>n.tag==='g'&&n.props.onPointerdown&&n.children.some(c=>c.tag==='line'))!
 const before=ui.doc();await ui.pointer(gizmo,0,0);svg.props.onPointermove(ui.event(svg,20,0));svg.props.onPointerup(ui.event(svg,20,0));await nextTick()
 expect(ui.doc().bodies[0].mesh.positions).not.toEqual(before.bodies[0].mesh.positions);await ui.click('↶');expect(ui.doc()).toEqual(before)
})
it('box-selects multiple sketches and deletes the selection atomically',async()=>{
 const ui=await mount(),svg=ui.all().find(n=>n.tag==='svg'&&n.props['aria-label']==='2D sketch canvas')!
 const box=ui.all().find(n=>n.tag==='button'&&ui.text(n)==='Box select')!;box.props.onClick();await nextTick()
 await ui.pointer(svg,-1,-11);svg.props.onPointermove(ui.event(svg,11,11));svg.props.onPointerup(ui.event(svg,11,11));await nextTick()
 await ui.click('Delete');expect(ui.doc().sketches.map(s=>s.name)).toEqual(['Circle']);await ui.click('↶');expect(ui.doc().sketches).toHaveLength(4)
})

it('creates closed primitives with undo in the embedded main scene editor',async()=>{
 const emitted:string[]=[]
 const ui=await mount({embedded:true,initialDocument:{version:1,sketches:[],bodies:[]},onAppend:(source:string)=>emitted.push(source)})
 for(const name of ['Box','Cylinder','Cone','Sphere']) {
  await ui.click(name)
  const body=ui.doc().bodies.at(-1)!,report=inspectPolygonMesh(body.mesh)
  expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(0)
 }
 expect(ui.doc().bodies).toHaveLength(4)
 await ui.click('↶');expect(ui.doc().bodies).toHaveLength(3)
 await ui.click('Apply to code');expect(emitted).toHaveLength(1);expect(emitted[0].match(/polyhedron\(/g)).toHaveLength(3)
})
