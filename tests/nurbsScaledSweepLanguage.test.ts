import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'

it('lowers scale records through packaged Rush frontend and enforces physical dimensions',()=>{
 const source=readFileSync('examples/rush/scaled-sweep.r','utf8')
 const graph=compileRushFrontend(source)
 expect(graph.execution_target).toBe('own-nurbs')
 expect(graph.document.nodes.find(n=>n.op==='scaled_sweep')).toMatchObject({
  origin:[0,0,0],scale:{degree:1,knots:[0,0,1,1],values:[1,2],weights:[1,1]},
 })
 expect(()=>compileRushFrontend(source.replace('values: [1,2]','values: [1,2mm]'))).toThrow(/scale\/values\/1/)
 expect(()=>compileRushFrontend(source.replace('origin: [0mm,0mm,0mm]','origin: [0mm,0mm,1deg]'))).toThrow()
 expect(()=>compileRushFrontend(source.replace('weights: [1,1]','weights: [1,1],typo: 2'))).toThrow()
})
