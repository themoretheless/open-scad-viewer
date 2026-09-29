import {expect,it,vi} from 'vitest'
import {SolidSketchSnapPreparation} from '../src/services/solidSketchSnapPreparation'
import {sketchSnapGeometry,type SnapGeometry} from '../src/services/modelingSnaps'
import type {DirectSketch} from '../src/services/directModeling'
const sketch=(id:string):DirectSketch=>({id,name:id,closed:false,points:[[0,0],[10,0]]})
it('reuses local targets across placement changes but invalidates edited geometry',async()=>{
 const a=sketch('a'),run=vi.fn(async(job:{sketch:DirectSketch})=>sketchSnapGeometry([job.sketch]))
 const queue=new SolidSketchSnapPreparation({run,cancel:vi.fn()})
 await queue.prepare([a]);expect(run).toHaveBeenCalledTimes(1)
 const moved={...structuredClone(a),name:'Renamed',plane:{origin:[5,6,7],u:[0,1,0],v:[0,0,1]}} as DirectSketch
 await queue.prepare([moved]);expect(run).toHaveBeenCalledTimes(1);expect(queue.get(moved)).toEqual(queue.get(a))
 moved.points[1][0]=20;await queue.prepare([moved]);expect(run).toHaveBeenCalledTimes(2)
 expect(queue.get(moved)?.points.some(p=>p.point[0]===20)).toBe(true)
})
it('discards late responses and reports targets that exceed the cache budget',async()=>{
 let resolve!:(g:SnapGeometry)=>void
 const a=sketch('a'),queue=new SolidSketchSnapPreparation({run:()=>new Promise(r=>{resolve=r}),cancel:vi.fn()})
 const pending=queue.prepare([a]);queue.cancel();resolve(sketchSnapGeometry([a]));await pending
 expect(queue.get(a)).toBeUndefined()
 const limited=new SolidSketchSnapPreparation({run:async()=>sketchSnapGeometry([a]),cancel:vi.fn()},1,1)
 expect((await limited.prepare([a])).errors.map(e=>e.id)).toEqual(['a']);expect(limited.size).toBe(0)
})
