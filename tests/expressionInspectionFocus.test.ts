import {readFileSync} from 'node:fs'
import {createRenderer,nextTick,ref,shallowRef,h} from 'vue'
import {it,expect,vi} from 'vitest'
const state=vi.hoisted(()=>({result:null as any,pending:null as any,error:null as any,run:vi.fn(),cancel:vi.fn()}))
vi.mock('../src/services/useExpressionInspection',()=>({useExpressionInspection:()=>{
 state.pending=ref(false)
 state.result=shallowRef(null)
 state.cancel=vi.fn(()=>{state.pending.value=false})
 state.error=ref('')
 state.run=vi.fn(()=>{state.pending.value=true})
 return {result:state.result,pending:state.pending,error:state.error,errorCode:ref(''),run:state.run,retry:()=>{},cancel:state.cancel}
}}))
import Panel from '../src/components/ExpressionShellCheck.vue'
it('focuses the replacement cancel button and routes Escape to cancellation',async()=>{
 const windowTarget=new EventTarget();vi.stubGlobal('window',windowTarget);vi.stubGlobal('Document',class {});vi.stubGlobal('ShadowRoot',class {})
 let focused:any
 const node=(type:string)=>({type,children:[] as any[],props:{} as any,parent:null as any,focus(){focused=this},getRootNode(){return {}},addEventListener(){},removeEventListener(){},setAttribute(){},removeAttribute(){},tagName:type.toUpperCase(),style:{},options:[]})
 const renderer=createRenderer<any,any>({
  createElement:node,createText:text=>({...node('#text'),text}),createComment:text=>({...node('#comment'),text}),
  setText:(n,t)=>{n.text=t},setElementText:(n,t)=>{n.text=t},
  parentNode:n=>n.parent,nextSibling:n=>{const a=n.parent?.children??[];return a[a.indexOf(n)+1]??null},
  insert(n,p,a){n.parent=p;const i=a?p.children.indexOf(a):-1;if(i<0)p.children.push(n);else p.children.splice(i,0,n)},
  remove(n){const p=n.parent;if(p)p.children.splice(p.children.indexOf(n),1)},
  patchProp:(n,k,_p,v)=>{n.props[k]=v},
 })
 const faces=vi.fn()
 const active=ref(true),options=ref({} as any)
 const root=node('root'),app=renderer.createApp({render:()=>h(Panel,{options:options.value,active:active.value,ru:false,onFaces:faces})});app.mount(root)
 const all=(n:any):any[]=>[n,...n.children.flatMap(all)]
 const inspect=all(root).find(n=>n.type==='button'&&n.props.onClick)
 inspect.props.onClick();await nextTick();await nextTick()
 expect(state.pending.value).toBe(true)
 inspect.props.onClick();await nextTick()
 expect(state.run).toHaveBeenCalledTimes(1)
 expect(focused?.type).toBe('button')
 expect(focused).not.toBe(inspect)
 const escape=new Event('keydown',{cancelable:true});Object.defineProperty(escape,'key',{value:'Escape'})
 active.value=false;await nextTick()
 windowTarget.dispatchEvent(escape)
 expect(state.cancel).not.toHaveBeenCalled()
 expect(escape.defaultPrevented).toBe(false)
 active.value=true;await nextTick()
 const enter=new Event('keydown',{cancelable:true});Object.defineProperty(enter,'key',{value:'Enter'})
 windowTarget.dispatchEvent(enter)
 expect(state.cancel).not.toHaveBeenCalled()
 expect(enter.defaultPrevented).toBe(false)
 windowTarget.dispatchEvent(escape)
 expect(escape.defaultPrevented).toBe(true)
 expect(state.cancel).toHaveBeenCalledOnce()
 expect(state.pending.value).toBe(false)
 // Invalid saved values are locked until a budget override is chosen.
 options.value={alignedChordCheck:{minimumMm:1,toleranceUv:0.05,maxSineSquared:1,controls:1000,cells:10000,spans:100000,domainCells:100000}}
 await nextTick()
 let inspectAgain=all(root).find(n=>n.type==='button'&&n.text==='Inspect')
 await inspectAgain.props.onClick();await nextTick()
 expect(state.pending.value).toBe(false)
 expect(focused.props['aria-label']).toBe('Distance check budget')
 const budget=all(root).find(n=>n.props['aria-label']==='Distance check budget')
 budget.props['onUpdate:modelValue']('standard');await nextTick()
 for(const [field,bad,good] of [['Normal-direction squared sine tolerance',1,0.0001],['Distance threshold, mm',0,1],['UV subdivision tolerance',0,0.05]] as const){
  const input=all(root).find(n=>n.props['aria-label']===field)
  input.props['onUpdate:modelValue'](bad);await nextTick()
  inspectAgain=all(root).find(n=>n.type==='button'&&n.text==='Inspect')
  await inspectAgain.props.onClick();await nextTick()
  expect(state.pending.value).toBe(false)
  expect(focused.props['aria-label']).toBe(field)
  expect(input.props['aria-invalid']).toBe(true)
  input.props['onUpdate:modelValue'](good);await nextTick()
 }
 await inspectAgain.props.onClick();await nextTick();await nextTick()
 expect(state.pending.value).toBe(true)
 state.cancel();await nextTick()
 const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json','utf8')).admitted
 fixture.shell.alignedChordCheck={qualified:false,report:{pending:[{faces:[0,0],pending_reason:'budget'},{faces:[1,1],pending_reason:'budget'}],initial_pairs_checked:0,total_pairs:435,cells:0,spans:0,intrinsic_spans:0}}
 state.result.value=fixture
 await nextTick()
 const region=all(root).find(n=>n.props['aria-label']==='Pending region')
 expect(region).toBeTruthy()
 faces.mockClear()
 region.props['onUpdate:modelValue'](2)
 await nextTick()
 expect(faces).toHaveBeenLastCalledWith([])
 // Recommending extended work is valid only below the binding hard limit.
 budget.props['onUpdate:modelValue']('saved')
 options.value={alignedChordCheck:{minimumMm:1,toleranceUv:0.05,maxSineSquared:0.0001,controls:1000,cells:100000,spans:1000000,domainCells:1000000}}
 const atLimit=structuredClone(fixture)
 atLimit.shell.alignedChordCheck.report.cells=100000
 state.result.value=atLimit;await nextTick()
 expect(all(root).some(n=>n.text?.includes('Maximum cell budget exhausted'))).toBe(true)
 expect(all(root).some(n=>n.text?.includes('Choose extended work and inspect again.'))).toBe(false)
 const atSpanLimit=structuredClone(atLimit)
 atSpanLimit.shell.alignedChordCheck.report.cells=0
 atSpanLimit.shell.alignedChordCheck.report.spans=1000000
 state.result.value=atSpanLimit;await nextTick()
 expect(all(root).some(n=>n.text?.includes('Maximum span budget exhausted'))).toBe(true)
 const atDomainLimit=structuredClone(atLimit)
 atDomainLimit.shell.alignedChordCheck.report.cells=0
 atDomainLimit.shell.alignedChordCheck.report.spans=0
 atDomainLimit.shell.alignedChordCheck.report.domain_cells=1000000
 state.result.value=atDomainLimit;await nextTick()
 expect(all(root).some(n=>n.text?.includes('Maximum trim-domain budget exhausted'))).toBe(true)
 expect(all(root).some(n=>n.text?.includes('Choose extended work and inspect again.'))).toBe(false)
 const contactResult=structuredClone(fixture)
 const product=[[[0.25,0.5],[0.1,0.2]],[[0.75,1],[0.3,0.4]]]
 contactResult.shell.inspection={contacts:{pendingPairs:2,pairs:[
  {faces:[0,28],proof:'pending',pendingProducts:[product,product]},
  {faces:[1,28],proof:'pending',pendingProducts:[product]},
 ]}}
 state.result.value=contactResult;await nextTick()
 const contactRegion=all(root).find(n=>n.props['aria-label']==='Unverified contact region')
 const contactPair=all(root).find(n=>n.props['aria-label']==='Unverified pair')
 contactRegion.props['onUpdate:modelValue'](2);await nextTick()
 expect(contactRegion.props['onUpdate:modelValue']).toBeTruthy()
 contactPair.props['onUpdate:modelValue'](1);await nextTick()
 expect(contactRegion.value).toBe(1)
 contactRegion.props['onUpdate:modelValue'](NaN);await nextTick()
 expect(all(root).some(n=>n.props['data-contact-uv-domains']!==undefined)).toBe(false)
 state.pending.value=false
 state.error.value='test failure'
 await nextTick();await nextTick()
 expect(focused.type).toBe('button')
 expect(focused.text).toBe('Retry')
 active.value=false;await nextTick()
 let retry=all(root).find(n=>n.type==='button'&&n.text==='Retry')
 expect(retry.props.disabled).toBe(true)
 const runCalls=state.run.mock.calls.length
 const cancelCalls=state.cancel.mock.calls.length
 await retry.props.onClick();await nextTick()
 expect(state.run).toHaveBeenCalledTimes(runCalls)
 expect(state.cancel).toHaveBeenCalledTimes(cancelCalls)
 const previousFocus=focused
 state.error.value='late inactive error'
 await nextTick();await nextTick()
 expect(focused).toBe(previousFocus)
 active.value=true;options.value=undefined;await nextTick()
 retry=all(root).find(n=>n.type==='button'&&n.text==='Retry')
 expect(retry.props.disabled).toBe(true)
 await retry.props.onClick();await nextTick()
 expect(state.run).toHaveBeenCalledTimes(runCalls)
 app.unmount()
 const calls=state.cancel.mock.calls.length
 state.pending.value=true
 windowTarget.dispatchEvent(escape)
 expect(state.cancel).toHaveBeenCalledTimes(calls)
 vi.unstubAllGlobals()
})
