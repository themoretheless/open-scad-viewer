import {createRenderer,nextTick,ref,h} from 'svet'
import {it,expect,vi} from 'vitest'
const state=vi.hoisted(()=>({tasks:[] as any[],worker:null as any}))
vi.mock('../src/kernel/solidPreviewWorker',()=>({createSolidPreviewWorker:()=>{state.tasks=[];state.worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn((job:any)=>new Promise((resolve,reject)=>state.tasks.push({resolve,reject,job})))};return state.worker}}))
import Panel from '../src/components/FrozenGraphBodyPanel.svt'
it('loads a display-only body, selects an edge and invalidates late data on cancel',async()=>{
 const node=(type:string):any=>({type,tagName:type.toUpperCase(),children:[],props:{},parent:null,text:'',get options(){return this.children.filter((n:any)=>n.type==='option')},addEventListener(){},removeEventListener(){},getRootNode(){return {}},setAttribute(){},removeAttribute(){}})
 const renderer=createRenderer<any,any>({createElement:node,createText:t=>({...node('text'),text:t}),createComment:node,setText:(n,t)=>n.text=t,setElementText:(n,t)=>n.text=t,parentNode:n=>n.parent,nextSibling:n=>{const a=n.parent?.children??[];return a[a.indexOf(n)+1]??null},insert(n,p,a){if(n.parent)n.parent.children.splice(n.parent.children.indexOf(n),1);n.parent=p;const i=a?p.children.indexOf(a):-1;if(i<0)p.children.push(n);else p.children.splice(i,0,n)},remove(n){const p=n.parent;if(p)p.children.splice(p.children.indexOf(n),1);n.parent=null},patchProp:(n,k,_p,v)=>{n.props[k]=v;if(k==='value'){n._value=v;n.value=v}}})
 const root=node('root'),active=ref(true),selected=ref<number|null>(null),displays:any[]=[]
 const app=renderer.createApp({render:()=>h(Panel,{active:active.value,ru:false,selectedEdge:selected.value,'onUpdate:selectedEdge':(v:number|null)=>selected.value=v,onDisplay:(v:any)=>displays.push(v)})});app.mount(root)
 const all=(n:any):any[]=>[n,...n.children.flatMap(all)],button=(s:string)=>all(root).find(n=>n.type==='button'&&n.text===s)
 const flush=async()=>{for(let i=0;i<20;i++)await Promise.resolve();await nextTick();await nextTick()}
 const response:any={report:{faces:9,edges:27,curvatureLimitPerMm:10},artifact:{geometry:{identityLinks:{edges:[{edge:0,sourceId:'original-edge'}]}}},display:{edges:[{edge:0,root:1,points:[[0,0,0],[1,1,1]]}]}}
 try{
  all(root).find(n=>n.type==='details').props.onToggle({target:{open:true}});await flush()
  const input=all(root).find(n=>n.type==='input'&&n.props.type==='file')
  const file={name:'test.json',size:2,stream:()=>new ReadableStream({start(c){c.enqueue(new TextEncoder().encode('{}'));c.close()}})}
  void input.props.onChange({target:{files:[file],value:'test.json'}});await flush();expect(state.tasks).toHaveLength(1)
  button('Cancel · Esc').props.onClick();state.tasks[0].resolve(response);await flush();expect(displays.at(-1)).toBe(null)
  void button('Retry · Enter').props.onClick();await flush();state.tasks[1].resolve(response);await flush();expect(displays.at(-1)).toBe(response)
  all(root).find(n=>n.type==='select').props['onUpdate:modelValue'](0);await flush();expect(selected.value).toBe(0)
  active.value=false;await flush();expect(displays.at(-1)).toBe(null);expect(selected.value).toBe(null)
 }finally{app.unmount();expect(state.worker.dispose).toHaveBeenCalledOnce()}
})
