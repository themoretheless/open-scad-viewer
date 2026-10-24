import {readFileSync} from 'node:fs'
import {createRenderer,nextTick,ref,h} from 'vue'
import {it,expect,vi} from 'vitest'
const state=vi.hoisted(()=>({tasks:[] as any[],worker:null as any}))
vi.mock('../src/services/solidPreviewWorker',()=>({createSolidPreviewWorker:()=>{
 state.tasks=[];state.worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn((job:any)=>new Promise((resolve,reject)=>state.tasks.push({resolve,reject,job})))}
 return state.worker
}}))
import Panel from '../src/components/ProfileBridgeCheck.vue'
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json','utf8'))
it('preserves keyboard focus through invalid input, cancel, Retry and closing; clears results on source edits',async()=>{
 vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {});vi.stubGlobal('HTMLInputElement',class {})
 let focused:any
 const node=(type:string)=>({type,children:[] as any[],props:{} as any,parent:null as any,focus(){focused=this},getRootNode(){return {}},addEventListener(){},removeEventListener(){},setAttribute(){},removeAttribute(){},tagName:type.toUpperCase(),style:{},options:[]})
 const renderer=createRenderer<any,any>({createElement:node,createText:text=>({...node('#text'),text}),createComment:text=>({...node('#comment'),text}),
 setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.text=t},parentNode:n=>n.parent,nextSibling:n=>{const a=n.parent?.children??[];return a[a.indexOf(n)+1]??null},
 insert(n,p,a){n.parent=p;const i=a?p.children.indexOf(a):-1;if(i<0)p.children.push(n);else p.children.splice(i,0,n)},
 remove(n){const p=n.parent;if(p)p.children.splice(p.children.indexOf(n),1)},patchProp:(n,k,_p,v)=>{n.props[k]=v}})
 const surfaces=ref(structuredClone(fixture.request.surfaces)),ru=ref(false),active=ref(true),root=node('root')
 const app=renderer.createApp({render:()=>h(Panel,{surfaces:surfaces.value,active:active.value,ru:ru.value})});app.mount(root)
 const all=(n:any):any[]=>[n,...n.children.flatMap(all)]
 const button=(text:string)=>all(root).find(n=>n.type==='button'&&n.text===text)
 const flush=async()=>{await nextTick();await nextTick()}
 const escape=()=>({key:'Escape',preventDefault:vi.fn(),stopPropagation:vi.fn()})
 try{
  button('Inspect profile transition').props.onClick();await flush()
  const angle=all(root).find(n=>n.type==='input'&&String(n.props.min)==='0')
  angle.props['onUpdate:modelValue'](1);await flush()
  await button('Inspect · Enter').props.onClick();await flush()
  expect(focused?.props.min).toBe(angle.props.min);expect(focused?.props['aria-invalid']).toBe(true);expect(angle.props['aria-invalid']).toBe(true);expect(state.tasks).toHaveLength(0)
  angle.props['onUpdate:modelValue'](0.0001);await flush()
  void button('Inspect · Enter').props.onClick();await flush()
  expect(state.tasks).toHaveLength(1);expect(focused.text).toBe('Cancel · Esc')
  const section=all(root).find(n=>n.type==='section'),event=escape()
  section.props.onKeydown(event);await flush()
  expect(event.preventDefault).toHaveBeenCalledOnce();expect(focused.text).toBe('Inspect · Enter')
  state.tasks[0].resolve(fixture.report);await flush()
  expect(all(root).some(n=>n.props.role==='status')).toBe(false)
  void button('Inspect · Enter').props.onClick();await flush()
  state.tasks[1].reject(Object.assign(new Error('native failure'),{code:'CAD_OPERATION'}));await flush()
  expect(button('Retry · Enter')).toBeTruthy();expect(focused.text).toBe('Retry · Enter')
  for(const [code,hint,russianHint] of [
   ['CAD_REQUEST','Correct the section parameters','Исправьте параметры сечений'],['CAD_TIMEOUT','Reduce the work budget','Уменьшите бюджет'],
   ['CAD_STARTUP','Retry the inspection','Повторите проверку'],['CAD_CRASH','Retry the inspection','Повторите проверку'],
   ['CAD_DISPOSED','Save the project and reload','Сохраните проект и перезагрузите'],['CAD_TRANSPORT','could not be transferred','Не удалось передать запрос'],
   ['CAD_PROTOCOL','Reload the application and Retry','Обновите приложение и повторите'],
  ]){
   void button('Retry · Enter').props.onClick();await flush()
   state.tasks.at(-1).reject(Object.assign(new Error('opaque native failure'),{code}));await flush()
   const alert=all(root).find(n=>n.props.role==='alert')
   expect(all(alert).map(n=>n.text??'').join(' ')).toContain(hint)
   expect(focused.text).toBe('Retry · Enter')
   const dispatched=state.tasks.length
   ru.value=true;await flush()
   const localized=all(root).find(n=>n.props.role==='alert')
   const localizedText=all(localized).map(n=>n.text??'').join(' ')
   expect(localizedText).toContain(russianHint);expect(localizedText).toContain(code)
   expect(localizedText).not.toContain('opaque native failure')
   expect(button('Повторить · Enter')).toBeTruthy()
   expect(state.tasks).toHaveLength(dispatched)
   ru.value=false;await flush()
  }
  void button('Retry · Enter').props.onClick();await flush()
  state.tasks.at(-1).resolve(fixture.stoppedCoverage.report);await flush()
  expect(all(root).some(n=>n.props.role==='status')).toBe(true)
  const region=all(root).find(n=>n.props['aria-label']==='Unresolved transition pair')
  expect(region.props.max).toBe(fixture.stoppedCoverage.report.coverage.unresolvedPairs.length)
  region.props['onUpdate:modelValue'](0);await flush();expect(region.props['aria-invalid']).toBe(true)
  region.props['onUpdate:modelValue'](1);await flush();expect(region.props['aria-invalid']).toBeUndefined()

  surfaces.value[0].controlPoints[0][0][0]+=0.01;await flush()
  expect(all(root).some(n=>n.props.role==='status')).toBe(false)
  void button('Inspect · Enter').props.onClick();await flush()
  const obsolete=state.tasks.at(-1)
  surfaces.value[0].weights[0][0]+=0.01;await flush()
  expect(button('Inspect · Enter')).toBeTruthy()
  obsolete.resolve(fixture.report);await flush()
  expect(all(root).some(n=>n.props.role==='status')).toBe(false)
  surfaces.value=structuredClone(fixture.request.surfaces);await flush()
  expect(all(root).some(n=>n.props.role==='status')).toBe(false)
  active.value=false;await flush()
  const dispatched=state.tasks.length
  expect(button('Inspect · Enter').props.disabled).toBe(true)
  expect(all(root).filter(n=>n.type==='input'||n.type==='select').every(n=>n.props.disabled===true)).toBe(true)
  expect(all(root).some(n=>n.text==='Finish the current command to inspect the transition.')).toBe(true)
  await button('Inspect · Enter').props.onClick();await flush()
  expect(state.tasks).toHaveLength(dispatched)
  active.value=true;await flush()
  expect(button('Inspect · Enter').props.disabled).toBe(false)
  section.props.onKeydown(escape());await flush()
  expect(focused.text).toBe('Inspect profile transition');expect(focused.props['aria-expanded']).toBe(false)
 }finally{app.unmount();vi.unstubAllGlobals()}
 expect(state.worker.dispose).toHaveBeenCalledOnce()
})
