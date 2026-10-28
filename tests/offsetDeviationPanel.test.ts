import {createRenderer,nextTick,ref,h} from 'vue'
import {it,expect,vi} from 'vitest'
const state=vi.hoisted(()=>({tasks:[] as any[],worker:null as any}))
vi.mock('../src/services/solidPreviewWorker',()=>({createSolidPreviewWorker:()=>{
 state.tasks=[];state.worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn((job:any)=>new Promise((resolve,reject)=>state.tasks.push({resolve,reject,job})))}
 return state.worker
}}))
import Panel from '../src/components/OffsetDeviationCheck.vue'
const surface=()=>({degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
 controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]})
it('invalidates stale results on edits, handles cancel and retry, and preserves selected geometry',async()=>{
 vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {});vi.stubGlobal('HTMLInputElement',class {})
 vi.stubGlobal('document',{activeElement:null})
 const node=(type:string)=>({type,children:[] as any[],props:{} as any,parent:null as any,
  getRootNode(){return {}},addEventListener(){},removeEventListener(){},setAttribute(){},removeAttribute(){},tagName:type.toUpperCase(),style:{},options:[]})
 const renderer=createRenderer<any,any>({createElement:node,createText:text=>({...node('#text'),text}),createComment:text=>({...node('#comment'),text}),
 setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.text=t},parentNode:n=>n.parent,nextSibling:n=>{const a=n.parent?.children??[];return a[a.indexOf(n)+1]??null},
 insert(n,p,a){n.parent=p;const i=a?p.children.indexOf(a):-1;if(i<0)p.children.push(n);else p.children.splice(i,0,n)},
 remove(n){const p=n.parent;if(p)p.children.splice(p.children.indexOf(n),1)},patchProp:(n,k,_p,v)=>{n.props[k]=v}})
 const surfaces=ref([surface(),surface()] as [ReturnType<typeof surface>,ReturnType<typeof surface>]),active=ref(true),ru=ref(false),root=node('root')
 const before=JSON.stringify(surfaces.value)
 const app=renderer.createApp({render:()=>h(Panel,{surfaces:surfaces.value,active:active.value,ru:ru.value})});app.mount(root)
 const all=(n:any):any[]=>[n,...n.children.flatMap(all)]
 const button=(text:string)=>all(root).find(n=>n.type==='button'&&n.text===text)
 const flush=async()=>{await nextTick();await nextTick()}
 const text=()=>all(root).map(n=>n.text??'').join(' ')
 try{
  button('Inspect offset A → B').props.onClick();await flush()
  void button('Inspect · Enter').props.onClick();await flush()
  expect(state.tasks[0].job).toMatchObject({kind:'offsetDeviation',options:{distance:1,tolerance:.001}})
  button('Cancel · Esc').props.onClick();await flush()
  state.tasks[0].resolve({decision:'within_tolerance',errorUpper:0,idealLocalFoldFree:true});await flush()
  expect(text()).not.toContain('Tolerance reached')
  void button('Inspect · Enter').props.onClick();await flush()
  state.tasks[1].reject(new Error('native failure'));await flush()
  expect(button('Retry · Enter')).toBeTruthy()
  void button('Retry · Enter').props.onClick();await flush()
  state.tasks[2].resolve({decision:'within_tolerance',errorUpper:.00001,idealLocalFoldFree:true});await flush()
  expect(text()).toContain('Tolerance reached')
  expect(JSON.stringify(surfaces.value)).toBe(before)
  surfaces.value[1].controlPoints[1][1][2]=.25;await flush()
  expect(text()).not.toContain('Tolerance reached')
  void button('Inspect · Enter').props.onClick();await flush()
  active.value=false;await flush()
  state.tasks[3].resolve({decision:'within_tolerance',errorUpper:0,idealLocalFoldFree:true});await flush()
  expect(text()).not.toContain('Tolerance reached')
  active.value=true;await flush()
  const tolerance=all(root).find(n=>n.props['aria-label']==='Tolerance, mm')
  tolerance.props['onUpdate:modelValue'](0);await flush()
  expect(button('Inspect · Enter').props.disabled).toBe(true)
  ru.value=true;await flush()
  expect(button('Проверить смещение A → B')).toBeTruthy()
 }finally{app.unmount();vi.unstubAllGlobals()}
 expect(state.worker.dispose).toHaveBeenCalledOnce()
})
