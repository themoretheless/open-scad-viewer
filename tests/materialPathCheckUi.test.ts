import {createRenderer,h,nextTick,shallowReactive} from 'vue'
import {it,expect,vi} from 'vitest'
import {readFileSync} from 'node:fs'
const mock=vi.hoisted(()=>({run:vi.fn(),cancel:vi.fn(),dispose:vi.fn()}))
vi.mock('../src/services/solidPreviewWorker',()=>({createSolidPreviewWorker:()=>mock}))
import Panel from '../src/components/MaterialPathCheck.vue'
const cases=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/whole-wall/contract.json','utf8')).cases
class Node{style:Record<string,string>={};get options(){return this.children.filter(n=>n.tag==='option')}parent:Node|null=null;children:Node[]=[];props:Record<string,any>={};text='';value:any;checked=false;constructor(public tag:string){}addEventListener(){}removeEventListener(){}get tagName(){return this.tag.toUpperCase()}}
const renderer=createRenderer<Node,Node>({
 createElement:t=>new Node(t),createText:t=>{const n=new Node('#text');n.text=t;return n},createComment:()=>new Node('#comment'),
 setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.children=[];n.text=t},parentNode:n=>n.parent,nextSibling:n=>n.parent?.children[n.parent.children.indexOf(n)+1]??null,
 patchProp:(n,k,_o,v)=>{n.props[k]=v;if(k==='value')n.value=v},
 insert:(n,p,a)=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n);n.parent=p;const i=a?p.children.indexOf(a):-1;i<0?p.children.push(n):p.children.splice(i,0,n)},
 remove:n=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n)},setScopeId:()=>{},insertStaticContent:()=>{throw Error('Unexpected static content')}
})
const flush=async()=>{await nextTick();await Promise.resolve();await nextTick()}
function mount(){
 const props=shallowReactive({active:true,ru:true,model:cases[0].request.model})
 const states:Array<[boolean,any]>=[]
 const root=new Node('root'),app=renderer.createApp({render:()=>h(Panel,{...props,onState:(active:boolean,point:any)=>states.push([active,point])})});app.mount(root)
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)],text=(n:Node=root):string=>n.text+n.children.map(text).join('')
 const click=async(s:string)=>{all().find(n=>n.tag==='button'&&text(n).includes(s))!.props.onClick();await flush()}
 return {props,root,app,all,text,click,states}
}
it('cancels a pending calculation, ignores its late result and permits retry',async()=>{
 mock.run.mockReset();const replies:Array<(v:any)=>void>=[];mock.run.mockImplementation(()=>new Promise(r=>replies.push(r)))
 const ui=mount(),before=JSON.stringify(ui.props.model)
 try{
  await ui.click('Проверка материала вдоль линии')
  ui.all().find(n=>n.tag==='select')!.props['onUpdate:modelValue']('whole');await flush()
  ui.all().find(n=>n.tag==='input'&&n.props.type==='checkbox')!.props['onUpdate:modelValue'](false);await flush()
  await ui.click('Проверить');expect(mock.run).toHaveBeenCalledTimes(1)
  expect(mock.run.mock.calls[0][0].kind).toBe('wholeWall')
  expect(ui.text()).toContain('Проверяю материал')
  await ui.click('Отменить расчёт')
  replies[0](cases[0].result);await flush()
  expect(ui.text()).not.toContain('Проверяю материал')
  expect(ui.all().some(n=>n.props['data-whole-wall-thickness']!==undefined)).toBe(false)
  expect(ui.states.at(-1)).toEqual([true,null])
  await ui.click('Проверить');expect(mock.run).toHaveBeenCalledTimes(2)
  replies[1](cases[0].result);await flush()
  expect(ui.all().some(n=>n.props['data-whole-wall-thickness']!==undefined)).toBe(true)
  expect(ui.states.at(-1)?.[1]).not.toBeNull()
  await ui.click('Закрыть');expect(ui.states.at(-1)).toEqual([false,null])
  expect(mock.cancel).toHaveBeenCalled();expect(JSON.stringify(ui.props.model)).toBe(before)
 }finally{ui.app.unmount()}
 expect(mock.dispose).toHaveBeenCalled()
})
it('shows a failed calculation and retries without changing the source body',async()=>{
 mock.run.mockReset();mock.run.mockRejectedValueOnce(Error('worker failed')).mockResolvedValueOnce(cases[0].result.candidate)
 const ui=mount(),before=JSON.stringify(ui.props.model)
 try{
  await ui.click('Проверка материала вдоль линии')
  await ui.click('Проверить');await flush()
  expect(ui.text()).toContain('Проверка не выполнена')
  expect(ui.states.at(-1)).toEqual([true,null])
  await ui.click('Повторить проверку');await flush()
  expect(ui.text()).not.toContain('Проверка не выполнена')
  expect(ui.states.at(-1)?.[1]).not.toBeNull()
  expect(JSON.stringify(ui.props.model)).toBe(before)
 }finally{ui.app.unmount()}
})
it('drops a late result after the active body changes',async()=>{
 mock.run.mockReset();let reply!:(v:any)=>void;mock.run.mockImplementation(()=>new Promise(r=>{reply=r}))
 const ui=mount()
 try{
  await ui.click('Проверка материала вдоль линии');await ui.click('Проверить')
  ui.props.model=cases[1].request.model;await flush()
  reply(cases[0].result.candidate);await flush()
  expect(ui.text()).not.toContain('Проверяю материал')
  expect(ui.states.at(-1)).toEqual([true,null])
  expect(ui.all().some(n=>n.props['data-whole-wall-thickness']!==undefined)).toBe(false)
 }finally{ui.app.unmount()}
})
