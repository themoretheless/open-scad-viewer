import {it,expect,vi} from 'vitest'
import {readFileSync} from 'node:fs'
import {SourceBodyDisplay,SourceBodyDisplayError,sourceBodyDisplayMessage,sourceBodyDisplayLocation} from '../src/services/sourceBodyDisplay'
import {createSourceBodyRecord} from '../src/services/sourceBodyArchive'
import type {SourceBodyResult} from '../src/services/sourceBody'
const {op:_,...options}=JSON.parse(readFileSync(new URL('../docs/qualification/mapped-retained-carrier-2026-10-08/multispan-root-body-step-request-2026-10-10.json',import.meta.url),'utf8'))
const denied=(uncertainPair:unknown):SourceBodyResult=>({admitted:false,sourceBody:null,edges:[],diagnostics:{reason:'mapped-source-endpoints-unproven',incidence:{uncertainPair},embedding:null,volume:null}})
it('locates a native refusal in the bound original recipe and preserves both locales',async()=>{
 const refusal=denied(5),run=vi.fn().mockResolvedValue(refusal),publish=vi.fn(),fail=vi.fn()
 const display=new SourceBodyDisplay({run,cancel:vi.fn()})
 await display.prepare([createSourceBodyRecord('original','Original body',options)],publish,fail)
 expect(publish).not.toHaveBeenCalled();expect(fail).toHaveBeenCalledOnce()
 const [id,error]=fail.mock.calls[0]!
 expect(id).toBe('original');expect(error).toBeInstanceOf(SourceBodyDisplayError)
 expect(error).toMatchObject({sourceBodyId:'original',pair:5,code:'CAD_SOURCE_BODY_RESTORE'})
 const faces=[...new Set(options.definition.shell.pairs[5].uses.map((u:number[])=>u[0]))]
 expect(error.faces).toEqual(faces)
 expect(sourceBodyDisplayLocation(error,'ru')).toContain('Ребро 6')
 expect(sourceBodyDisplayLocation(error,'en')).toContain('Edge 6')
 expect(sourceBodyDisplayMessage(error,'ru')).toContain('точки обрезки')
 expect(sourceBodyDisplayMessage(error,'en')).toContain('trim points')
 expect(error.diagnostics).toBe(refusal.diagnostics)
})
it('never invents a scene location for missing or invalid native pair indices',()=>{
 for(const pair of [null,undefined,-1,0.5,NaN,Infinity,'5',options.definition.shell.pairs.length]){
  const error=new SourceBodyDisplayError(denied(pair).diagnostics,options.definition,'original')
  expect(error.pair).toBeNull();expect(error.faces).toEqual([])
  expect(sourceBodyDisplayLocation(error,'ru')).toBe('')
 }
 expect(sourceBodyDisplayLocation(new Error('CAD_PROTOCOL'),'en')).toBe('')
})
it('does not publish a refusal or its address after source display cancellation',async()=>{
 let resolve!:(result:SourceBodyResult)=>void
 const run=vi.fn(()=>new Promise<SourceBodyResult>(r=>{resolve=r})),publish=vi.fn(),fail=vi.fn()
 const display=new SourceBodyDisplay({run,cancel:vi.fn()})
 const pending=display.prepare([createSourceBodyRecord('original','Original body',options)],publish,fail)
 display.cancel();resolve(denied(5));await pending
 expect(publish).not.toHaveBeenCalled();expect(fail).not.toHaveBeenCalled()
})
it('provides distinct localized next actions for each supported native refusal',()=>{
 for(const [reason,ru,en] of [
  ['mapped-source-endpoints-unproven','точки обрезки','trim points'],
  ['mapped-source-composition-unproven','исходные кривые','original curves'],
  ['mapped-source-uses-not-opposite','ориентацию граней','face orientation'],
  ['source-shell-work-limit','Увеличьте лимиты','Increase limits'],
 ]){
  const diagnostics={...denied(5).diagnostics,reason:reason!}
  const error=new SourceBodyDisplayError(diagnostics,options.definition,'original')
  expect(sourceBodyDisplayMessage(error,'ru')).toContain(ru)
  expect(sourceBodyDisplayMessage(error,'en')).toContain(en)
  expect(sourceBodyDisplayLocation(error,'ru')).toContain('Ребро 6')
 }
})
