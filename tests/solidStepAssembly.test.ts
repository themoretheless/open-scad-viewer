import {afterEach,expect,it,vi} from 'vitest'
import {exportSolidStepAssembly} from '../src/services/solidStepAssembly'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import * as kernel from '../src/services/geometry/kernel'
import type {DirectDocument} from '../src/services/directModeling'
afterEach(()=>vi.restoreAllMocks())
const scene=():DirectDocument=>({version:1,sketches:[],bodies:cadRoadmapParts().map(f=>({...f.body,group:'Parts'}))})
it('exports every current body under one root and group without changing the document',async()=>{
 const doc=scene(),before=structuredClone(doc)
 doc.bodies[0].name="Кронштейн 'A' #999 $&"
 before.bodies[0].name=doc.bodies[0].name
 const text=await exportSolidStepAssembly(doc)
 expect(text.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(4)
 expect(text.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(5)
 expect(text).toContain("''A'' #999 $&")
 expect(text).toContain('\\X2\\041A\\X0\\')
 const ids=Array.from(text.matchAll(/^#(\d+)=/gm),m=>m[1])
 expect(new Set(ids).size).toBe(ids.length)
 expect(doc).toEqual(before)
})
it('snapshots the scene before asynchronous kernel readiness',async()=>{
 const doc=scene();let release!:()=>void
 vi.spyOn(kernel,'warmGeometryKernel').mockReturnValue(new Promise<void>(resolve=>{release=resolve}))
 const exporting=exportSolidStepAssembly(doc)
 doc.bodies.length=0;release()
 expect((await exporting).match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(4)
})
it('refuses partial exports and duplicate identities',async()=>{
 const doc=scene();delete doc.bodies[1].brep
 await expect(exportSolidStepAssembly(doc)).rejects.toThrow('flange')
 const duplicate=scene();duplicate.bodies[1].id=duplicate.bodies[0].id
 await expect(exportSolidStepAssembly(duplicate)).rejects.toThrow('unique body IDs')
 await expect(exportSolidStepAssembly({version:1,sketches:[],bodies:[]})).rejects.toThrow('at least one')
 const sketches=scene();sketches.sketches.push({id:'s',name:'s',points:[[0,0],[1,0]],closed:false})
 await expect(exportSolidStepAssembly(sketches)).rejects.toThrow('body-only')
})
