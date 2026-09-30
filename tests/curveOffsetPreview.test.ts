import {expect,it} from 'vitest'
import {offsetPreviewSegments} from '../src/services/curveOffsetPreview'
import type {CurveOffsetDiagnostics} from '../src/services/curveOffsetDiagnostics'
const line=(points:number[][])=>({degree:1,knots:[],weights:points.map(()=>1),controlPoints:points})
it('maps diagnostics across retained curve chunks without inventing a seam segment',()=>{
 const curves=[line([[0,0],[1,1],[2,0]]),line([[2,0],[3,1],[4,0]])]
 const diagnostics={crossings:[[0,3]],contacts:[],uncertain:[[1,2]],degenerate:[]} as unknown as CurveOffsetDiagnostics
 const s=offsetPreviewSegments(curves,diagnostics)
 expect(s).toHaveLength(4);expect(s.map(x=>x.state)).toEqual(['error','uncertain','uncertain','error'])
 expect(s[2]).toMatchObject({a:[2,0],b:[3,1],index:2})
})
it('leaves an ordinary chain clear and does not reinterpret a zero-offset rational source as chords',()=>{
 expect(offsetPreviewSegments([line([[0,0],[1,0]])],null)[0]!.state).toBe('preview')
 expect(offsetPreviewSegments([{...line([[0,0],[1,1],[2,0]]),degree:2}],null)).toEqual([])
})
