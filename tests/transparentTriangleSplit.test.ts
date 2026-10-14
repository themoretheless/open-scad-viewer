import {expect,it} from 'vitest'
import {splitTransparentTriangle, type TransparentTriangle} from '../src/services/transparentTriangleSplit'
const triangle:TransparentTriangle=[[-1,-1,-.6,0],[1,-1,.6,1],[0,1,0,.5]]
function area(t:readonly (readonly number[])[]){const a=t[1].map((v,i)=>v-t[0][i]),b=t[2].map((v,i)=>v-t[0][i]);return Math.hypot(a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])/2}
it('preserves surface area and interpolates attributes at the crossing',()=>{
 const r=splitTransparentTriangle(triangle,[1,0,0,0])
 expect(r.front).toHaveLength(1);expect(r.back).toHaveLength(1);expect(r.coplanar).toEqual([])
 expect([...r.front,...r.back].reduce((sum,t)=>sum+area(t),0)).toBeCloseTo(area(triangle),12)
 for(const t of [...r.front,...r.back])for(const v of t)if(v[0]===0){expect(v[2]).toBe(0);expect(v[3]).toBe(.5)}
 expect(r.front.flat().every(v=>v[0]>=0)).toBe(true);expect(r.back.flat().every(v=>v[0]<=0)).toBe(true)
 r.front[0][0][0]=999;expect(triangle[0][0]).toBe(-1);expect(r.back.flat().some(v=>v[0]===999)).toBe(false)
})
it('triangulates a clipped quad without losing area and normalizes plane scale',()=>{
 const r=splitTransparentTriangle(triangle,[2,0,0,-.5])
 expect(r.front).toHaveLength(1);expect(r.back).toHaveLength(2)
 expect([...r.front,...r.back].reduce((sum,t)=>sum+area(t),0)).toBeCloseTo(area(triangle),12)
 expect(r).toEqual(splitTransparentTriangle(triangle,[1,0,0,-.25]))
})
it('classifies coplanar and one-sided triangles without duplicates',()=>{
 expect(splitTransparentTriangle(triangle,[-.6,0,1,0]).coplanar).toHaveLength(1)
 expect(splitTransparentTriangle(triangle,[1,0,0,2]).front).toHaveLength(1)
 expect(splitTransparentTriangle(triangle,[1,0,0,-2]).back).toHaveLength(1)
 expect(()=>splitTransparentTriangle(triangle,[0,0,0,1])).toThrow()
 expect(()=>splitTransparentTriangle(triangle,[1,0,0,0],NaN)).toThrow()
})

it('matches the frozen clipper over crossing planes and arbitrary linear attributes',async()=>{
 const {referenceSplitTransparentTriangle}=await import('../benchmarks/rush/transparentTriangleSplit-reference')
 for(const scale of [.001,1,1000])for(let i=0;i<20;i++){
  const t=triangle.map(v=>v.map((x,k)=>k<3?x*scale:x)) as unknown as TransparentTriangle
  const p=[Math.sin(i+.2),Math.cos(i+.7),.3,(i-10)*scale*.07] as const
  const actual=splitTransparentTriangle(t,p),expected=referenceSplitTransparentTriangle(t,p)
  for(const key of ['front','back','coplanar'] as const){
   expect(actual[key]).toHaveLength(expected[key].length)
   actual[key].forEach((t,j)=>t.forEach((v,k)=>v.forEach((x,l)=>expect(x).toBeCloseTo(expected[key][j][k][l],10))))
  }
 }
})
