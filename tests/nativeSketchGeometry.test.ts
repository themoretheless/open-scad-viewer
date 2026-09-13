import {it,expect} from 'vitest'
import {sampleCurve,transformSketch,type AnalyticCurve} from '../src/services/directSketchGeometry'
import type {DirectSketch} from '../src/services/directModeling'
it('samples signed arcs including endpoints and circles without duplicate closure',()=>{
 const curve:AnalyticCurve={kind:'arc',center:[2,3],radius:5,start:90,sweep:-90}
 const arc=sampleCurve(curve)
 expect(arc).toHaveLength(31)
 expect(arc[0][0]).toBeCloseTo(2,12);expect(arc[0][1]).toBeCloseTo(8,12)
 expect(arc.at(-1)![0]).toBeCloseTo(7,12);expect(arc.at(-1)![1]).toBeCloseTo(3,12)
 const circle=sampleCurve({...curve,kind:'circle'})
 expect(circle).toHaveLength(120);expect(circle[0]).not.toEqual(circle.at(-1))
 for(const p of circle)expect(Math.hypot(p[0]-2,p[1]-3)).toBeCloseTo(5,11)
})
it('transforms polyline vertices about an explicit pivot and keeps metadata',()=>{
 const sketch:DirectSketch={id:'s',name:'Polyline',closed:false,points:[[1,0],[3,0],[3,2]],plane:{origin:[4,5,6],u:[1,0,0],v:[0,1,0]}},before=JSON.stringify(sketch)
 const moved=transformSketch(sketch,[10,20],90,2,[1,1])
 const expected=[[13,21],[13,25],[9,25]]
 moved.points.forEach((p,i)=>p.forEach((x,k)=>expect(x).toBeCloseTo(expected[i][k],12)))
 expect(moved.plane).toEqual(sketch.plane);expect(moved.id).toBe('s');expect(JSON.stringify(sketch)).toBe(before)
})
it('keeps arc parameters analytic and rebuilds display points after placement',()=>{
 const analytic:AnalyticCurve={kind:'arc',center:[2,3],radius:4,start:10,sweep:120},sketch:DirectSketch={id:'a',name:'Arc',closed:false,analytic,points:sampleCurve(analytic)}
 const result=transformSketch(sketch,[5,-2],30,2)
 expect(result.analytic).toEqual({...analytic,center:[7,1],radius:8,start:40})
 expect(result.points).toEqual(sampleCurve(result.analytic!))
 expect(sketch.analytic).toEqual(analytic)
})
it('rejects invalid sampling and transformation before returning corrupt geometry',()=>{
 const curve:AnalyticCurve={kind:'circle',center:[0,0],radius:1,start:0,sweep:360}
 for(const patch of [{radius:0},{radius:1e7},{sweep:0},{sweep:361},{center:[Infinity,0] as [number,number]}])expect(()=>sampleCurve({...curve,...patch})).toThrow()
 const sketch:DirectSketch={id:'s',name:'S',closed:false,points:[[0,0],[1,1]]},before=JSON.stringify(sketch)
 expect(()=>transformSketch(sketch,[0,0],0,0)).toThrow()
 expect(()=>transformSketch(sketch,[0,0],0,1,[NaN,0])).toThrow()
 expect(()=>transformSketch({...sketch,points:[]},[0,0],0,1)).toThrow('points')
 expect(JSON.stringify(sketch)).toBe(before)
})
it('offsets polygon contours consistently in either winding and refuses collapsed insets',async()=>{
 const {offsetSketch}=await import('../src/services/directSketchGeometry')
 const sketch:DirectSketch={id:'box',name:'Box',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},before=JSON.stringify(sketch)
 for(const points of [sketch.points,[...sketch.points].reverse()]){
  const outward=offsetSketch({...sketch,points},1),inward=offsetSketch({...sketch,points},-1)
  expect(outward.points.map(p=>p.join(',')).sort()).toEqual(['-1,-1','11,-1','11,11','-1,11'].sort())
  expect(inward.points.map(p=>p.join(',')).sort()).toEqual(['1,1','9,1','9,9','1,9'].sort())
  expect(()=>offsetSketch({...sketch,points},-6)).toThrow('collapses')
 }
 expect(JSON.stringify(sketch)).toBe(before)
})
it('offsets analytic arcs by radius without losing their parameters',async()=>{
 const {offsetSketch}=await import('../src/services/directSketchGeometry')
 const analytic:AnalyticCurve={kind:'arc',center:[2,3],radius:4,start:10,sweep:-120},sketch:DirectSketch={id:'a',name:'Arc',closed:false,analytic,points:sampleCurve(analytic)}
 const moved=offsetSketch(sketch,2)
 expect(moved.analytic).toEqual({...analytic,radius:6});expect(moved.points).toEqual(sampleCurve(moved.analytic!))
 expect(()=>offsetSketch(sketch,-4)).toThrow()
})
it('validates open and closed contours natively and rejects invalid offset requests',async()=>{
 const {offsetSketch,validateSimpleSketch}=await import('../src/services/directSketchGeometry')
 expect(()=>validateSimpleSketch([[0,0],[2,2],[0,2],[2,0]],true)).toThrow('self-intersect')
 expect(()=>validateSimpleSketch([[0,0],[2,0],[2,2]],false)).not.toThrow()
 expect(()=>validateSimpleSketch([[0,0]],false)).toThrow()
 expect(()=>validateSimpleSketch(Array.from({length:513},(_,i)=>[i,0]),false)).toThrow()
 const open:DirectSketch={id:'p',name:'Open',closed:false,points:[[0,0],[1,0]]}
 expect(()=>offsetSketch(open,1)).toThrow('closed contour')
 expect(()=>offsetSketch(open,0)).toThrow('nonzero')
})
it('trims an open segment into ordered chains and opens a closed contour',async()=>{
 const {trimSketch}=await import('../src/services/directSketchGeometry')
 const line:DirectSketch={id:'line',name:'Line',closed:false,points:[[0,0],[10,0]]}
 const boundaries=[3,7].map((x,i):DirectSketch=>({id:`cut${i}`,name:'Cut',closed:false,points:[[x,-1],[x,1]]}))
 const before=JSON.stringify(line),parts=trimSketch(line,0,.5,boundaries)
 expect(parts.map(p=>p.points)).toEqual([[[0,0],[3,0]],[[7,0],[10,0]]])
 expect(parts.map(p=>p.id)).toEqual(['line','line-trim']);expect(JSON.stringify(line)).toBe(before)
 const square={...line,closed:true,points:[[0,0],[10,0],[10,10],[0,10]] as [number,number][]}
 const opened=trimSketch(square,0,.5,boundaries)
 expect(opened).toHaveLength(1);expect(opened[0].closed).toBe(false)
 expect(opened[0].points).toEqual([[7,0],[10,0],[10,10],[0,10],[0,0],[3,0]])
 expect(trimSketch(line,0,.5,[])).toEqual([])
})
it('extends either endpoint to the nearest forward boundary and preserves metadata',async()=>{
 const {extendSketch}=await import('../src/services/directSketchGeometry')
 const line:DirectSketch={id:'line',name:'Line',closed:false,points:[[0,0],[2,0]]}
 const boundaries=[-3,5,8].map((x,i):DirectSketch=>({id:`b${i}`,name:'Boundary',closed:false,points:[[x,-1],[x,1]]}))
 expect(extendSketch(line,'end',boundaries).points).toEqual([[0,0],[5,0]])
 expect(extendSketch(line,'start',boundaries).points).toEqual([[-3,0],[2,0]])
 expect(extendSketch(line,'start',boundaries).name).toBe(line.name)
 expect(()=>extendSketch(line,'end',[{...boundaries[0],id:'line'}])).toThrow('No boundary')
})
it('rejects malformed trim or extension without changing source geometry',async()=>{
 const {trimSketch,extendSketch}=await import('../src/services/directSketchGeometry')
 const line:DirectSketch={id:'s',name:'S',closed:false,points:[[0,0],[2,0]]},before=JSON.stringify(line)
 expect(()=>trimSketch(line,1,.5,[])).toThrow()
 expect(()=>trimSketch(line,0,NaN,[])).toThrow()
 expect(()=>trimSketch(line,0,2,[])).toThrow()
 expect(()=>extendSketch({...line,points:[]},'end',[])).toThrow()
 expect(()=>extendSketch({...line,closed:true},'end',[])).toThrow('open polyline')
 expect(JSON.stringify(line)).toBe(before)
})

it('refuses a trim result above the document point limit and recovers for a valid request',async()=>{
 const {trimSketch}=await import('../src/services/directSketchGeometry')
 const points:[number,number][]=[[0,0],[10,0],...Array.from({length:510},(_,i):[number,number]=>[10-i/51,1])]
 const sketch:DirectSketch={id:'limit',name:'Limit',closed:true,points},before=JSON.stringify(sketch)
 const boundaries=[3,7].map((x,i):DirectSketch=>({id:`cut${i}`,name:'Cut',closed:false,points:[[x,-1],[x,1]]}))
 expect(()=>trimSketch(sketch,0,.5,boundaries)).toThrow('512 points')
 expect(JSON.stringify(sketch)).toBe(before)
 expect(trimSketch({...sketch,closed:false,points:[[0,0],[10,0]]},0,.5,boundaries)).toHaveLength(2)
})
it('refuses an extension that would cross the original contour',async()=>{
 const {extendSketch}=await import('../src/services/directSketchGeometry')
 const sketch:DirectSketch={id:'s',name:'S',closed:false,points:[[0,0],[4,0],[4,4],[2,4],[2,2]]}
 const boundary:DirectSketch={id:'b',name:'Boundary',closed:false,points:[[0,-2],[4,-2]]}
 const before=JSON.stringify(sketch)
 expect(()=>extendSketch(sketch,'end',[boundary])).toThrow('self-intersect')
 expect(JSON.stringify(sketch)).toBe(before)
})
it('places batches of 2D and 3D points in a scaled oblique basis natively',async()=>{
 const {worldPoints,worldPoint}=await import('../src/services/directSketchGeometry')
 const plane={origin:[10,20,30] as [number,number,number],u:[2,0,0] as [number,number,number],v:[1,3,0] as [number,number,number]}
 const points=[[1,2],[1,2,4]],before=JSON.stringify(points)
 expect(worldPoints(points,plane)).toEqual([[14,26,30],[14,26,54]])
 expect(worldPoint(points[1],plane)).toEqual([14,26,54])
 expect(worldPoints([[1,2],[3,4,5]])).toEqual([[1,2,0],[3,4,5]])
 expect(worldPoints([])).toEqual([]);expect(JSON.stringify(points)).toBe(before)
})
it('refuses invalid or overflowing placement before returning a partial point batch',async()=>{
 const {worldPoints}=await import('../src/services/directSketchGeometry')
 for(const points of [[[0,0],[1]],[[0,0],[Infinity,0]],[[0,0],[1,2,3,4]]])expect(()=>worldPoints(points)).toThrow()
 expect(()=>worldPoints([[2,0]],{origin:[0,0,0],u:[1e308,0,0],v:[0,1,0]})).toThrow('finite')
 expect(worldPoints([[2,3]])).toEqual([[2,3,0]])
})
