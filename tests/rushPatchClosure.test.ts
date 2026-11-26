import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function audit(panelCount:number){
  const inputs=Array.from({length:panelCount},(_,index)=>'panel'+index)
  const nodes=inputs.map((id,index)=>{
    const path='tests/fixtures/rush-iges/closure/panel-'+index+'.igs'
    return id+' = open_iges_file(iges_text:'+JSON.stringify(readFileSync(path,'utf8'))+')'
  })
  const source=['// @rush/1',...nodes,
    'audit = panel0.inspect_patch_closure(inputs:['+inputs.slice(1).join(',')+'],max_intervals:100,max_cells:10000)',
    'show audit.brep_tessellate(4)',
  ].join('\n')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row=>row.operation==='inspect_patch_closure')!.report)
}

it('pairs the twelve cube boundaries of six independently loaded IGES patches',async()=>{
  const report=await audit(6)
  expect(report).toMatchObject({
    allBoundariesPaired:true,sourceAgreementComplete:true,openIntervals:0,ambiguousIntervals:0,unsupported:[],
  })
  expect(report.boundaries).toHaveLength(24)
  expect(report.matches).toHaveLength(12)
  expect(report.matches.every((match:{rangeMm:number[]})=>match.rangeMm[0]===0&&match.rangeMm[1]===1)).toBe(true)
},180000)

it('leaves four open intervals when one of the patches is absent',async()=>{
  expect(await audit(5)).toMatchObject({allBoundariesPaired:false,openIntervals:4})
},180000)
