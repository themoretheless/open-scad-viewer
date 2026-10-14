import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
const build=(source:string)=>buildOwnNurbs(compileRushFrontend(source).document,{action:'build',display:{segments:2,subdivisionLevels:0}})
it('retains the complete original-profile transport bound for the closed planar guided source',()=>{
 const source=readFileSync(new URL('../examples/rush/closed-planar-guided-frame-c2-progressive-sweep.r',import.meta.url),'utf8')
 const reports=Object.values(build(source).report.construction!)
 const report=reports.find(r=>'continuousErrorUpper' in r)!
 if(!('continuousErrorUpper' in report))throw new Error('Missing original sweep error report')
 expect(report.continuousBound).toBe(true)
 expect(report.continuousErrorUpper).toBeGreaterThan(0)
 expect(report.continuousErrorUpper).toBeLessThanOrEqual(report.budget)
})
it('refuses unproved off-plane antiparallel refinement and restores the exact planar source',()=>{
 const source=readFileSync(new URL('../examples/rush/corrected-frenet-antiparallel-progressive-sweep.r',import.meta.url),'utf8')
 const bad=source.replace('[1mm,-1mm,0]','[1.0000000000000002mm,-1mm,0]')
 expect(bad).not.toBe(source)
 expect(()=>build(bad)).toThrow(/continuous retained-patch error is unproved \(frame-regularity-subdivision-unresolved\)/)
 expect(()=>build(source)).not.toThrow()
})
