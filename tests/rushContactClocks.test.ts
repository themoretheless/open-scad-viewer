import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(transform=(s:string)=>s){
  const program=transform(readFileSync('examples/rush/order-boundary-contacts.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  return JSON.parse(scene.measurements!.find(row=>row.operation==='order_boundary_contacts')!.report)
}

function less(a:string,b:string){
  const [an,ad='1']=a.split('/'),[bn,bd='1']=b.split('/')
  return BigInt(an!)*BigInt(bd)<BigInt(bn!)*BigInt(ad)
}

it('orders distinct contact clocks and retains both contributors at the exact quadratic root',async()=>{
  expect(await run(s=>s.replace('isolate:true','isolate:false'))).toMatchObject({complete:true,groups:[[2],[0,3],[1]]})
},180000)

it('separates the exact contact groups with rational witnesses',async()=>{
  const report=await run()
  expect(report.complete).toBe(true)
  expect(report.intervals.map((entry:{indices:number[]})=>entry.indices)).toEqual([[2],[0,3],[1]])
  expect(report.witnesses).toHaveLength(2)
  for(const [i,value] of report.witnesses.entries()){
    expect(less(report.intervals[i].bounds[1],value)).toBe(true)
    expect(less(value,report.intervals[i+1].bounds[0])).toBe(true)
  }
  expect(await run(s=>s.replace('max_work:100000','max_work:1'))).toMatchObject({complete:false,reason:'ResourceLimit'})
},180000)
