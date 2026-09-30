import {createRenderer,h,nextTick,shallowReactive} from 'vue'
import {it,expect,vi} from 'vitest'
import {readFileSync} from 'node:fs'
const mock=vi.hoisted(()=>({run:vi.fn(),cancel:vi.fn(),dispose:vi.fn()}))
vi.mock('../src/services/solidPreviewWorker',()=>({createSolidPreviewWorker:()=>mock}))
import Panel from '../src/components/SolidVolumeDistance.vue'
const cases=JSON.parse(readFileSync('docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json','utf8')).cases
class Node{parent:Node|null=null;children:Node[]=[];props:Record<string,any>={};text='';value:any;checked=false;constructor(public tag:string){}addEventListener(){}removeEventListener(){}get tagName(){return this.tag.toUpperCase()}}
const renderer=createRenderer<Node,Node>({
 createElement:t=>new Node(t),createText:t=>{const n=new Node('#text');n.text=t;return n},createComment:()=>new Node('#comment'),
 setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.children=[];n.text=t},parentNode:n=>n.parent,nextSibling:n=>n.parent?.children[n.parent.children.indexOf(n)+1]??null,
 patchProp:(n,k,_o,v)=>{n.props[k]=v;if(k==='value')n.value=v},
 insert:(n,p,a)=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n);n.parent=p;const i=a?p.children.indexOf(a):-1;i<0?p.children.push(n):p.children.splice(i,0,n)},
 remove:n=>{if(n.parent)n.parent.children=n.parent.children.filter(c=>c!==n)},setScopeId:()=>{},insertStaticContent:()=>{throw Error('Unexpected static content')}
})
const flush=async()=>{await nextTick();await Promise.resolve();await nextTick()}
function mount(){
 const props=shallowReactive({active:true,ru:true,a:cases[0].request.a,b:cases[0].request.b,same:false,names:['A','B'] as [string,string]})
 const states:Array<[boolean,[number,number,number]|null]>=[]
 const root=new Node('root'),app=renderer.createApp({render:()=>h(Panel,{...props,onState:(active:boolean,point:[number,number,number]|null)=>states.push([active,point])})});app.mount(root)
 const all=(n:Node=root):Node[]=>[n,...n.children.flatMap(all)],text=(n:Node=root):string=>n.text+n.children.map(text).join('')
 const click=async(s:string)=>{all().find(n=>n.tag==='button'&&text(n).includes(s))!.props.onClick();await flush()}
 return {props,root,app,all,text,click,states}
}
it('cancels on close/context change and ignores late results',async()=>{
 mock.run.mockReset();const replies:Array<(v:any)=>void>=[];mock.run.mockImplementation(()=>new Promise(r=>replies.push(r)))
 const ui=mount(),before=JSON.stringify([ui.props.a,ui.props.b])
 try{
  await ui.click('Расстояние между объёмами');expect(mock.run).toHaveBeenCalledTimes(1)
  expect(ui.text()).toContain('Проверяю объёмы')
  await ui.click('Закрыть');replies[0](cases[0].result);await flush()
  expect(ui.all().some(n=>n.props['data-solid-distance']!==undefined)).toBe(false)
  await ui.click('Расстояние между объёмами');ui.props.active=false;await flush()
  replies[1](cases[0].result);await flush();expect(ui.text()).not.toContain('Общие точки тел подтверждены')
  expect(mock.cancel).toHaveBeenCalled();expect(JSON.stringify([ui.props.a,ui.props.b])).toBe(before)
 }finally{ui.app.unmount()}
})
it('reports unresolved input by body and allows retry after failure',async()=>{
 mock.run.mockReset();mock.run.mockRejectedValueOnce(Error('worker')).mockResolvedValueOnce(cases[1].result).mockResolvedValueOnce(cases[0].result)
 const ui=mount()
 try{
  await ui.click('Расстояние между объёмами');await flush();expect(ui.text()).toContain('Расчёт не выполнен')
  await ui.click('Повторить');await flush();expect(ui.text()).toContain('B: ориентация оболочек не подтверждена')
  await ui.click('Повторить');await flush();expect(ui.text()).toContain('Расстояние — 0 мм')
  ui.props.same=true;await flush();expect(ui.text()).toContain('Выберите два разных тела')
  expect(ui.text()).not.toContain('Расстояние — 0 мм')
 }finally{ui.app.unmount()}
})

it('publishes only current contact markers and clears them on source changes and close',async()=>{
 mock.run.mockReset();mock.run.mockResolvedValueOnce(cases[2].result).mockImplementationOnce(()=>new Promise(()=>{}))
 const ui=mount()
 try{
  await ui.click('Расстояние между объёмами');await flush()
  expect(ui.states.at(-1)).toEqual([true,cases[2].result.contact.pointIntervalMm.map(([lo,hi]:[number,number])=>lo/2+hi/2)])
  ui.props.b=cases[2].request.b;await flush();expect(ui.states.at(-1)).toEqual([true,null])
  await ui.click('Закрыть');expect(ui.states.at(-1)).toEqual([false,null])
 }finally{ui.app.unmount()}
 expect(ui.states.at(-1)).toEqual([false,null])
})
