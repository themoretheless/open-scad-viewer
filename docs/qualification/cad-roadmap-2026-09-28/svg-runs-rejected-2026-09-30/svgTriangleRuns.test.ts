import {expect,it} from 'vitest'
import {svgTriangleRuns} from '../src/services/svgTriangleRuns'
it('combines adjacent same-paint triangles without changing painter order or input',()=>{
 const items=[{id:'a',points:'0,0 1,0 0,1'},{id:'a',points:'1,0 1,1 0,1'},{id:'b',points:'2,0 3,0 2,1'},{id:'a',points:'4,0 5,0 4,1'}]
 const before=JSON.stringify(items),runs=svgTriangleRuns(items,p=>p.id)
 expect(runs.map(p=>p.id)).toEqual(['a','b','a'])
 expect(runs[0].path).toBe('M0,0L1,0L0,1ZM1,0L1,1L0,1Z')
 expect(runs[1].path).toBeUndefined();expect(JSON.stringify(items)).toBe(before)
})
it('preserves separate transparent, preview, differently shaded and reversed-winding triangles',()=>{
 const items=[{points:'0,0 1,0 0,1',paint:'a'},{points:'1,0 1,1 0,1',paint:null},{points:'1,0 1,1 0,1',paint:'a'},{points:'1,0 1,1 0,1',paint:'b'}]
 expect(svgTriangleRuns(items,p=>p.paint)).toEqual(items)
 expect(svgTriangleRuns([],()=>null)).toEqual([])
})
