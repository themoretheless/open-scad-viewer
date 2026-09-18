import {createRenderer,h,nextTick,shallowReactive,shallowRef} from 'vue'
import {it,expect,vi,afterEach} from 'vitest'
import DirectModeler from '../src/features/DirectModeler.vue'
import {extrudeDirectSketch,type DirectDocument} from '../src/services/directModeling'
import {sampleCurve} from '../src/services/directSketchGeometry'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
import {createBrepBox,analyzeNurbsBrep,createBrepCylinder,tessellateNurbsBrep} from '../src/services/geometry/brep'
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
 const currentProps=shallowReactive({open:true,locale:'en',canAppend:true,remainingSource:100000,...props})
 const instance=shallowRef<any>(null)
 const root=new Node('root'),app=renderer.createApp({setup:()=>()=>h(DirectModeler,{...currentProps,ref:instance})});app.mount(root);mounts.push(()=>app.unmount());await nextTick()
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)]
 const text=(n:Node):string=>n.text+n.children.map(text).join('')
 const findButton=(name:string)=>all().find(n=>n.tag==='button'&&(text(n)===name||n.props['aria-label']===name||n.props['aria-label']===name.replace(/^\+ /,'')))
 const button=(name:string)=>{const n=findButton(name);if(!n)throw Error('Missing button '+name);return n}
 // Context actions live in the command palette; fall back to executing the matching command by its label.
 const command=(name:string)=>{const label=name.replace(/ · .*$/,'');return (instance.value?.solidCommands as Array<{id:string;label:string;aliases?:readonly string[]}>|undefined)?.find(c=>c.label===label||c.aliases?.includes(label))}
 const click=async(name:string,shiftKey=false)=>{const n=findButton(name);if(n){n.props.onClick({shiftKey});await nextTick();return}const c=command(name);if(!c)throw Error('Missing button '+name);instance.value.executeSolidCommand(c.id);await nextTick()}
 const svg=()=>all().find(n=>n.tag==='svg'&&n.props['aria-label']==='3D body canvas')!
 const event=(n:Node,x=0,y=0)=>({button:0,target:n,currentTarget:n,clientX:x,clientY:y,pointerId:1,preventDefault(){},stopPropagation(){}})
 const pointer=async(n:Node,x=0,y=0)=>{n.props.onPointerdown(event(n,x,y));await nextTick()}
 return {all,text,button,click,svg,pointer,event,setProps:async(next:Record<string,unknown>)=>{Object.assign(currentProps,next);await nextTick()},doc:()=>JSON.parse(stored) as DirectDocument,field:async(value:number)=>{const input=all().find(n=>n.tag==='input'&&n.props['onUpdate:modelValue']&&n.props.step===.5)??all().find(n=>n.tag==='input'&&n.props.type==='number'&&n.props['onUpdate:modelValue']);if(!input)throw Error('Missing field');input.props['onUpdate:modelValue'](value);await nextTick()}}
}

function cylinderSeed(): DirectDocument {
 const brep=createBrepCylinder(3,5),built=tessellateNurbsBrep(brep,2)
 return {version:1,sketches:[],bodies:[{id:'imported-cylinder',name:'Imported cylinder',brep,
  mesh:{positions:built.positions,indices:built.indices}}]}
}

it('changes retained B-rep display detail and restores the previous mesh with Undo',async()=>{
 const seed=cylinderSeed(),before=structuredClone(seed.bodies[0])
 const ui=await mount({seedDocument:seed})
 await ui.click('Imported cylinder');await ui.click('B-rep detail')
 const field=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('B-rep detail'))!
 field.props['onUpdate:modelValue'](8);await nextTick();await ui.click('Retessellate')
 expect(ui.doc().bodies[0].mesh.indices.length).toBeGreaterThan(before.mesh.indices.length)
 expect(ui.doc().bodies[0].brep).toEqual(before.brep)
 expect(ui.button('↶').props.disabled).toBe(false)
 await ui.click('↶')
 expect(ui.doc().bodies[0]).toEqual(before)
 expect(seed.bodies[0]).toEqual(before)
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
 await ui.click('Stock');await ui.click('Cutter',true);await ui.click('B-rep A − B')
 expect(ui.doc().bodies).toEqual([])
 await ui.click('↶');expect(ui.doc().bodies.map(b=>b.id)).toEqual(['Stock','Cutter'])
})
it('binds an edge selection to chamfer and creates a shell with the selected opening',async()=>{
 const ui=await mount();await ui.click('Cube');await ui.click('Edges');const edge=ui.all(ui.svg()).find(n=>n.tag==='polyline'&&n.props.onPointerdown)!;await ui.pointer(edge)
 await ui.click('Chamfer 3D');await ui.click('Apply · Enter');expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(980)
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Shell');await ui.click('Apply · Enter')
 expect(inspectPolygonMesh(ui.doc().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000-6*6*8)
})
it('runs authored B-rep boolean and refuses retained fillet without mutation',async()=>{
 const ui=await mount();await ui.click('Box');await ui.click('Box')
 const objectBoxes=ui.all().filter(n=>n.tag==='button'&&ui.text(n).startsWith('Box · 3D'))
 objectBoxes[0].props.onClick({shiftKey:false});await nextTick();objectBoxes[1].props.onClick({shiftKey:true});await nextTick()
 await ui.click('B-rep Union')
 expect(ui.doc().bodies.filter(b=>b.brep)).toHaveLength(1)
 const before=ui.doc()
 await ui.click('Edges');const edges=ui.all(ui.svg()).filter(n=>n.tag==='polyline'&&n.props.onPointerdown),edge=edges[0],ends=new Set(String(edge.props.points).split(' '))
 await ui.pointer(edge);const connected=edges.slice(1).find(item=>String(item.props.points).split(' ').some(point=>ends.has(point)))!
 connected.props.onPointerdown({...ui.event(connected),shiftKey:true});await nextTick();await ui.click('Fillet 3D')
 const segments=ui.all().find(n=>n.tag==='input'&&n.parent&&ui.text(n.parent).startsWith('Fillet segments'))!
 segments.props['onUpdate:modelValue'](6);await nextTick();await ui.click('Apply · Enter')
 expect(ui.text(ui.all()[0])).toContain('refuse faceted fallback')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.doc()).toEqual(before)
})
it('authors a full sketch revolve as an explicitly faceted B-rep',async()=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Revolve')
 await new Promise(resolve=>setTimeout(resolve,80));await nextTick();await ui.click('Apply · Enter')
 const body=ui.doc().bodies.at(-1)!
 expect(body.name).toMatch(/faceted B-rep/)
 expect(body.brep).toBeDefined()
 expect(body.brep?.faces.length).toBeGreaterThan(6)
 expect(inspectPolygonMesh(body.mesh).closed).toBe(true)
})
it.each(['x','y'])('keeps exact partial revolve outward oriented around %s and refuses implicit mesh combination',async axis=>{
 const ui=await mount();await ui.click('Profile');await ui.click('Revolve')
 const set=async(label:string,value:unknown)=>{const field=ui.all().find(n=>['input','select'].includes(n.tag)&&n.parent&&ui.text(n.parent).startsWith(label)&&n.props['onUpdate:modelValue'])!;field.props['onUpdate:modelValue'](value);await nextTick()}
 await set('Sketch axis',axis);await set('Axis offset, mm',-5);await set('Revolve surfaces','exact');await set('Angle, °',180);await ui.click('Add')
 await new Promise(resolve=>setTimeout(resolve,100));await nextTick()
 expect(ui.text(ui.all()[0])).toContain('Exact B-rep revolve combination requires an authored B-rep target.')
 await ui.click('Apply · Enter');expect(ui.doc().bodies).toHaveLength(1)
 await ui.click('New');await new Promise(resolve=>setTimeout(resolve,100));await nextTick();await ui.click('Apply · Enter')
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
 const inputs=ui.all().filter(n=>n.tag==='input'&&n.props['onUpdate:modelValue']);const radius=inputs.find(n=>n.parent&&ui.text(n.parent).startsWith('Radius / size, mm'))!
 radius.props['onUpdate:modelValue'](5);await nextTick();await ui.click('Apply · Enter');expect(ui.doc().sketches.find(s=>s.id==='circle')?.analytic?.radius).toBe(5)
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
 await ui.click('Box select')
 await ui.pointer(svg,-1,-11);svg.props.onPointermove(ui.event(svg,11,11));svg.props.onPointerup(ui.event(svg,11,11));await nextTick()
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
 await nextTick()
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

it('preserves authored B-rep for movement and push while refusing retained shell',async()=>{
 const ui=await mount();await ui.click('Box');const original=ui.doc().bodies.at(-1)!
 await ui.click('Move · G');const svg=ui.svg();await ui.pointer(ui.all(svg).find(n=>n.tag==='polygon'&&n.props.onPointerdown)!,0,0)
 svg.props.onPointermove(ui.event(svg,4,3));svg.props.onPointerup(ui.event(svg,4,3));await nextTick()
 const moved=ui.doc().bodies.find(b=>b.id===original.id)!
 expect(moved.brep).toBeDefined();expect(moved.brep!.topologyIds).toEqual(original.brep!.topologyIds);expect(moved.brep!.vertices).not.toEqual(original.brep!.vertices)
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Push / Pull');await ui.click('Apply · Enter')
 expect(ui.doc().bodies.find(b=>b.id===original.id)!.brep).toBeDefined()
 await ui.click('↶');await ui.click('Faces');await ui.pointer(ui.all(ui.svg()).find(n=>n.tag==='polygon')!);await ui.click('Shell');await ui.click('Apply · Enter')
 expect(ui.text(ui.all()[0])).toContain('refuse faceted fallback')
 expect(ui.button('Apply · Enter').props.disabled).toBe(true)
 expect(ui.doc().bodies.find(b=>b.id===original.id)).toEqual(original)
})

it('previews and commits retained splits with positive-side identity and reversible history',async()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),mesh=tessellateNurbsBrep(brep,1)
 const seed:DirectDocument={version:1,sketches:[],bodies:[{id:'retained',name:'Retained stock',brep,mesh}]}
 const before=structuredClone(seed),ui=await mount({seedDocument:seed})
 await ui.click('Retained stock');await ui.click('Split')
 expect(ui.doc()).toEqual(before)
 await ui.click('Apply · Enter')
 const result=ui.doc();expect(result.bodies).toHaveLength(2)
 expect(result.bodies[0].id).toBe('retained');expect(result.bodies[1].id).not.toBe('preview-split')
 expect(result.bodies[1].id).not.toBe('retained')
 expect(analyzeNurbsBrep(result.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(800,7)
 expect(analyzeNurbsBrep(result.bodies[1].brep!).signedVolumeMm3).toBeCloseTo(200,7)
 for(const body of result.bodies)expect(inspectPolygonMesh(body.mesh).signedVolumeMm3).toBeCloseTo(analyzeNurbsBrep(body.brep!).signedVolumeMm3,7)
 await ui.click('↶');expect(ui.doc().bodies).toEqual(before.bodies)
 expect(seed).toEqual(before)
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
