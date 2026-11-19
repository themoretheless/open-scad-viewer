import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/trim-exact-tangents.r','utf8')
const rational=(n:number,d=1)=>({numerator:String(n),denominator:String(d)})
async function run(program:string){
 const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
 return scene.measurements!.map(row=>JSON.parse(row.report))
}
it('retains the original contour clock on both sides of a seam corner',async()=>{
 const [point,tangents,join,vertex]=await run(source)
 expect(point.point).toEqual([rational(0),rational(0),rational(1,4)])
 expect(tangents.tangents.before).toEqual([rational(0),rational(-1),rational(0)])
 expect(tangents.tangents.after).toEqual([rational(1),rational(0),rational(0)])
 expect(join.join).toBe('Corner')
 expect(vertex.join).toBe('Corner')
 const smooth=await run(source.replaceAll('exact_parameter:"1/2"','exact_parameter:"3/4"'))
 expect(smooth[2].join).toBe('C1')
 await expect(run(source.replace('exact_parameter:"1/2"','exact_parameter:"1/0"'))).rejects.toThrow()
})

it('preserves a displacement smaller than binary64 resolution on the seam parameter',async()=>{
  const denominator=2n**81n
  const numerator=denominator/2n+1n
  const [point]=await run(source.replace('exact_parameter:"1/2"',`exact_parameter:"${numerator}/${denominator}"`))
  expect(point.point).toEqual([{numerator:'1',denominator:String(denominator)},rational(0),rational(1,4)])
})
