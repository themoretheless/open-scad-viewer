import {readFileSync} from 'node:fs'
import {createSSRApp,shallowRef,ref} from 'vue'
import {renderToString} from '@vue/server-renderer'
import {it,expect,vi} from 'vitest'
const state=vi.hoisted(()=>({result:undefined as any,error:'',errorCode:''}))
vi.mock('../src/services/useExpressionInspection',()=>({useExpressionInspection:()=>({
 result:shallowRef(state.result),pending:ref(false),error:ref(state.error),errorCode:ref(state.errorCode),run:vi.fn(),retry:vi.fn(),cancel:vi.fn(),
})}))
import ExpressionShellCheck from '../src/components/ExpressionShellCheck.vue'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-radius-dispatch-2026-10-09.json',import.meta.url),'utf8'))
for(const ru of [true,false])it(`shows unverified contact domains without calling them self-intersections: ${ru}`,async()=>{
 state.result=structuredClone(fixture.admitted)
 const transition=state.result.shell.faces.findIndex((f:any)=>f.kind==='transition')
 const product=[[[0.25,0.5],[0.1,0.2]],[[0.75,1],[0.3,0.4]]]
 state.result.shell.inspection={contacts:{pendingPairs:1,pairs:[{faces:[0,transition],proof:'pending',pendingProducts:Array(10000).fill(product)}]}}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(html).toContain(ru?'Самопересечение для них не подтверждено.':'They are not confirmed self-intersections.')
 expect(html).toContain('data-contact-transition-limitation')
 expect(html).toContain('data-contact-uv-domains')
 expect(html).toContain('U [0.25, 0.5]')
 expect(html).toContain('V [0.3, 0.4]')
 expect(html).toContain('max="10000"')
 expect(html.match(/aria-label="(?:Непроверенная область контакта|Unverified contact region)"/g)).toHaveLength(1)
 expect(html.length).toBeLessThan(15000)
 const inactive=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:false,ru}))
 expect(inactive).toMatch(/<button[^>]*disabled[^>]*>[^<]*(?:Показать исходные грани|Show source faces)/)
})
for(const [name,label] of [['admitted','допуск подтверждён'],['violation','нарушение допуска подтверждено'],['unproven','не проверен']]){
 it(`renders the native ${name} radius state without whole-body qualification`,async()=>{
  state.result=fixture[name]
  const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru:true}))
  expect(html).toContain(label)
  expect(html).toContain('Касательность и толщина требуют отдельных проверок.')
  expect(html).toContain(`data-radius-state="${name==='admitted'?'qualified':name==='violation'?'violation':'unproven'}"`)
  if(name==='violation')expect(html).toContain('Проверьте закон радиуса и форму перехода перед применением.')
 })
}
const supportFixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json',import.meta.url),'utf8'))
for(const name of ['admitted','unproven'])it(`renders bounded support tangency ${name} without neighbor body admission`,async()=>{
 state.result=supportFixture[name]
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru:true}))
 expect(html).toContain(`data-support-tangency-state="${name==='admitted'?'qualified':'unproven'}"`)
 expect(html).toContain('Соседние грани тела и толщина требуют отдельных проверок.')
 if(name==='admitted')expect(html).toContain('четыре границы подтверждены')
 else expect(html).toContain('Проверка неполная.')
})
const neighborFixture=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/expression-neighbor-tangency-dispatch-2026-10-10.json',import.meta.url),'utf8'))
for(const name of ['admitted','unproven'])it(`renders native actual-neighbor tangency ${name} with remaining whole-body checks`,async()=>{
 state.result=neighborFixture[name]
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru:true}))
 expect(html).toContain(`data-neighbor-tangency-state="${name==='admitted'?'qualified':'unproven'}"`)
 expect(html).toContain('Толщина, самопересечения и остальные швы требуют отдельных проверок.')
 expect(html).not.toContain('Соседние грани тела и толщина требуют отдельных проверок.')
 if(name==='admitted')expect(html).toContain('четыре соединения подтверждены')
 else expect(html).toContain('Повторите с большим бюджетом или исправьте соединения перехода.')
})

for(const ru of [true,false])for(const qualified of [true,false])it(`renders aligned distance coverage with explicit whole-thickness limit: ${ru}/${qualified}`,async()=>{
 state.result=structuredClone(neighborFixture.admitted)
 state.result.request.alignedChordCheck={minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,cells:10000,spans:100000,domainCells:100000}
 state.result.shell.alignedChordCheck={qualified,report:{pending:qualified?[]:[{faces:[0,1]}],initial_pairs_checked:435,total_pairs:435,cells:100,spans:200,intrinsic_spans:50}}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(html).toContain(`data-aligned-chord-state="${qualified?'qualified':'unproven'}"`)
 expect(html).toContain(ru?'Минимальная толщина всего тела не подтверждена.':'Minimum thickness of the whole body is unqualified.')
 expect(html).toContain(ru?'Бюджет проверки расстояний':'Distance check budget')
 if(!qualified)expect(html).toContain(ru?'Показать исходные грани кандидатов':'Show candidate source faces')
})

it('renders one pending-region selector without enumerating a large frontier',async()=>{
 state.result=structuredClone(neighborFixture.admitted)
 state.result.request.alignedChordCheck={minimumMm:1,maxSineSquared:1e-4,toleranceUv:0.05,controls:1000,cells:10000,spans:100000,domainCells:100000}
 const region={faces:[0,1],domains:[[[0,1],[0,1]],[[0,1],[0,1]]],pending_reason:'budget'}
 state.result.shell.alignedChordCheck={qualified:false,report:{pending:Array(435).fill(region),initial_pairs_checked:435,total_pairs:435,cells:100,spans:200,intrinsic_spans:50}}
 const render=()=>renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru:false}))
 const initial=await render()
 state.result.shell.alignedChordCheck.report.pending=Array(30557).fill(region)
 const large=await render()
 expect(large).toContain('max="30557"')
 expect(large.match(/aria-label="Pending region"/g)).toHaveLength(1)
 expect(Math.abs(large.length-initial.length)).toBeLessThan(100)
})

for(const ru of [true,false])for(const [reason,en,translated] of [
 ['budget','Calculation budget exhausted. Choose extended work and inspect again.','Исчерпан бюджет расчёта. Выберите расширенный бюджет и повторите проверку.'],
 ['enclosure','Surface bounds are unproven. Check the source geometry.','Не подтверждены границы поверхности. Проверьте исходную геометрию.'],
 ['resolution','Region subdivision limit reached. A more precise check is required.','Достигнут предел разбиения области. Требуется более точная проверка.'],
])it(`localizes the pending distance region reason: ${ru}/${reason}`,async()=>{
 state.result=structuredClone(neighborFixture.admitted)
 state.result.shell.alignedChordCheck={qualified:false,report:{pending:[{faces:[0,1],pending_reason:reason}],initial_pairs_checked:1,total_pairs:435,cells:1,spans:1,intrinsic_spans:0}}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(html).toContain(ru?translated:en)
})

it('shows saved distance precision and normal tolerance without replacing them with defaults',async()=>{
 state.result.request.alignedChordCheck={minimumMm:1.4,maxSineSquared:0.002,toleranceUv:0.012,controls:1000,cells:0,spans:0,domainCells:0}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru:false}))
 expect(html).toMatch(/<input(?=[^>]*aria-label="UV subdivision tolerance")(?=[^>]*value="0.012")[^>]*>/)
 expect(html).toMatch(/<input(?=[^>]*aria-label="Normal-direction squared sine tolerance")(?=[^>]*value="0.002")[^>]*>/)
})

for(const ru of [true,false])it(`explains unavailable transition-face highlighting and preserves UV intervals: ${ru}`,async()=>{
 state.result=structuredClone(neighborFixture.admitted)
 const transition=state.result.shell.faces.findIndex((f:any)=>f.kind==='transition')
 expect(transition).toBeGreaterThanOrEqual(0)
 state.result.shell.alignedChordCheck={qualified:false,report:{pending:[{faces:[transition,transition],domains:[[[0.25,0.5],[0.1,0.2]],[[0.75,1],[0.3,0.4]]],pending_reason:'budget'}],initial_pairs_checked:0,total_pairs:435,cells:0,spans:0,intrinsic_spans:0}}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(html).toContain('data-chord-transition-limitation')
 expect(html).toContain(ru?'переход в сцене пока не отображается.':'the transition is not yet displayed in the scene.')
 expect(html).toMatch(/<button[^>]*disabled[^>]*>[^<]*(?:Показать исходные грани кандидатов|Show candidate source faces)/)
 expect(html).toContain('U [0.25, 0.5]')
 expect(html).toContain('V [0.3, 0.4]')
})

for(const ru of [true,false])it(`renders optional native span accounting as work rather than timing: ${ru}`,async()=>{
 state.result=structuredClone(neighborFixture.admitted)
 state.result.shell.alignedChordCheck={qualified:false,report:{pending:[],initial_pairs_checked:0,total_pairs:435,cells:0,spans:36,intrinsic_spans:7,profile_transition_spans:8,span_stages:{position:1,normal:2,shared_corner:3,original_pair:4,retained_pair:5,corrected_pair:6}}}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(html).toContain('data-chord-span-stages')
 expect(html).toContain(ru?'Это число обработанных участков, а не время расчёта.':'These counts measure processed spans, not elapsed time.')
 expect(html).toContain((ru?'Координаты':'Positions')+': 1')
 expect(html).toContain((ru?'Нормали':'Normals')+': 2')
 delete state.result.shell.alignedChordCheck.report.span_stages
 const legacy=await renderToString(createSSRApp(ExpressionShellCheck,{options:state.result.request,active:true,ru}))
 expect(legacy).not.toContain('data-chord-span-stages')
})

for(const ru of [true,false])for(const [code,ruHint,enHint] of [
 ['CAD_REQUEST','Некорректные параметры проверки.','Invalid inspection parameters.'],
 ['CAD_TIMEOUT','Уменьшите бюджет и повторите проверку.','Reduce the work budget and retry.'],
 ['CAD_CRASH','Вычислитель остановлен или не запустился.','The computation worker stopped or could not start.'],
 ['CAD_PROTOCOL','Ответ расчёта не прошёл проверку.','The calculation response failed validation.'],
 ['CAD_TRANSPORT','Не удалось передать запрос или получить результат.','The request or result could not be transferred.'],
 ['CAD_DISPOSED','Сохраните проект и перезагрузите страницу.','Save the project and reload the page.'],
 ['CAD_OPERATION','Проверьте исходную геометрию и параметры.','Check the source geometry and parameters.'],
])it(`renders actionable inspection failure ${code}/${ru}`,async()=>{
 state.result=undefined;state.error='technical fixture error';state.errorCode=code
 try{
  const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:neighborFixture.admitted.request,active:true,ru}))
  expect(html).toContain('role="alert"');expect(html).toContain(ru?ruHint:enHint)
  expect(html).toContain(ru?'Повторить':'Retry')
  expect(html).toMatch(/<details[^>]*><summary[^>]*>(Подробности|Details)<\/summary>/);expect(html).toContain('technical fixture error')
 }finally{state.error='';state.errorCode=''}
})

for(const ru of [true,false])for(const [field,value,ruLabel,enLabel,ruHint,enHint] of [
 ['minimumMm',0,'Порог расстояния, мм','Distance threshold, mm','Введите конечное число больше 0.','Enter a finite number greater than 0.'],
 ['toleranceUv',0,'Точность разбиения UV','UV subdivision tolerance','Введите конечное число больше 0 и не больше 1.','Enter a finite number greater than 0 and at most 1.'],
 ['maxSineSquared',1,'Допуск sin² направления нормалей','Normal-direction squared sine tolerance','Введите конечное число от 0 до 1, не включая 1.','Enter a finite number from 0 up to, but excluding, 1.'],
] as const)it(`locates the invalid ${field} parameter with an accessible hint: ${ru}`,async()=>{
 state.result=undefined
 const request=structuredClone(neighborFixture.admitted.request)
 request.alignedChordCheck={minimumMm:1,toleranceUv:0.05,maxSineSquared:1e-4,controls:1000,cells:10000,spans:100000,domainCells:100000,[field]:value}
 const html=await renderToString(createSSRApp(ExpressionShellCheck,{options:request,active:true,ru}))
 const inputs=html.match(/<input\b[^>]*>/g)??[]
 const invalid=inputs.filter(input=>input.includes('aria-invalid="true"'))
 expect(invalid).toHaveLength(1)
 expect(invalid[0]).toContain(`aria-label="${ru?ruLabel:enLabel}"`)
 const description=invalid[0].match(/aria-describedby="([^"]+)"/)?.[1]
 expect(description).toBeTruthy();expect(html).toContain(`id="${description}"`)
 expect(html).toContain(`data-chord-parameter-error="${field}"`)
 expect(html).toContain(ru?ruHint:enHint)
})
