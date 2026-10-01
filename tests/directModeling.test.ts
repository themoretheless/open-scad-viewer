import { describe, expect, it } from 'vitest'
import { bodyPoints, DirectHistory, directBodiesScad, emptyDirectDocument, extrudeDirectSketch, parseDirectDocument, transformDirectPoints } from '../src/services/directModeling'
import { inspectPolygonMesh } from '../src/services/geometry/polygon'
import { parseOpenSCAD } from '../src/services/openscadParser'
const sketch = () => ({ id: 's', name: 'L', closed: true, points: [[0,0],[20,0],[20,10],[10,10],[10,20],[0,20]] as [number,number][] })
describe('direct modeling', () => {
  it('creates a closed concave solid, independent of edits and deletion of its sketch', () => {
    const d = emptyDirectDocument(); d.sketches.push(sketch()); d.bodies.push(extrudeDirectSketch(d.sketches[0], 5, 'body'))
    const mesh = JSON.stringify(d.bodies[0].mesh)
    d.sketches[0].points[0][0] = -100; d.sketches = []
    expect(JSON.stringify(d.bodies[0].mesh)).toBe(mesh)
    const report = inspectPolygonMesh(d.bodies[0].mesh)
    expect(report.closed).toBe(true); expect(report.signedVolumeMm3).toBeCloseTo(1500)
  })
  it('undoes and redoes complete snapshots, drops redo after a new edit and isolates callers', () => {
    const h = new DirectHistory(), d = h.document; d.sketches.push(sketch()); h.commit(d)
    d.sketches[0].points[0][0] = 99
    expect(h.document.sketches[0].points[0][0]).toBe(0)
    const e = h.document; e.bodies.push(extrudeDirectSketch(e.sketches[0], 5, 'b')); h.commit(e)
    expect(h.undo().bodies).toHaveLength(0); expect(h.redo().bodies).toHaveLength(1)
    h.undo(); const f = h.document; f.sketches[0].name = 'New'; h.commit(f)
    expect(h.canRedo).toBe(false); expect(h.document.bodies).toHaveLength(0)
  })
  it('transforms body vertices without modifying the 2D source', () => {
    const s = sketch(), b = extrudeDirectSketch(s, 5, 'b'), before = JSON.stringify(s)
    b.mesh.positions = transformDirectPoints(bodyPoints(b), [10,20,30], 90, 2).flat()
    expect(inspectPolygonMesh(b.mesh).signedVolumeMm3).toBeCloseTo(12000)
    expect(JSON.stringify(s)).toBe(before)
    expect(Math.min(...bodyPoints(b).map(p=>p[2]))).toBeCloseTo(27.5)
  })
  it('round trips JSON and rejects invalid geometry atomically', () => {
    const d = emptyDirectDocument(); d.sketches.push(sketch()); const h = new DirectHistory(d)
    expect(parseDirectDocument(JSON.stringify(d))).toEqual(d)
    const invalid = h.document; invalid.sketches[0].points[0][0] = NaN
    expect(()=>h.commit(invalid)).toThrow(); expect(h.canUndo).toBe(false)
    expect(()=>extrudeDirectSketch({...sketch(), closed:false},5,'b')).toThrow()
    expect(()=>extrudeDirectSketch(sketch(),-1,'b')).toThrow()
    expect(()=>transformDirectPoints([[0,0]], [0,0], 0, 0)).toThrow()
  })
  it('exports baked solids that compile into the main scene with correct volume and topology', async () => {
    const d = emptyDirectDocument(); d.bodies.push(extrudeDirectSketch(sketch(),5,'b'))
    const result = await parseOpenSCAD(directBodiesScad(d))
    expect(result.volume).toBeCloseTo(1500)
    expect(result.meshes.length).toBeGreaterThan(0)
    for (const m of result.meshes) { expect(m.topology.boundary).toBe(0); expect(m.topology.nonManifold).toBe(0) }
  })
})
it('transforms mixed-dimensional point batches natively without losing dimensions',()=>{
 const points=[[1,0],[3,0,4]],before=JSON.stringify(points)
 const result=transformDirectPoints(points,[10,20,30],90,2)
 expect(result[0]).toHaveLength(2);expect(result[1]).toHaveLength(3)
 ;[[12,18],[12,22,36]].forEach((p,i)=>p.forEach((x,k)=>expect(result[i][k]).toBeCloseTo(x,12)))
 expect(JSON.stringify(points)).toBe(before)
})
it('keeps representable large centroids finite and refuses malformed point transforms',()=>{
 const points=[[1e308,0],[1e308,0]]
 expect(transformDirectPoints(points,[0,0],0,1)).toEqual(points)
 for(const p of [[],[[1]],[[1,2,3,4]],[[0,0],[Infinity,0]]])expect(()=>transformDirectPoints(p,[0,0],0,1)).toThrow()
 expect(()=>transformDirectPoints([[0,0]],[0],0,1)).toThrow()
 expect(()=>transformDirectPoints([[1e308,0]],[1e308,0],0,1)).toThrow('finite')
 expect(transformDirectPoints([[0,0]],[1,2],0,1)).toEqual([[1,2]])
})

it('restores history asynchronously without moving stacks on failure, cancellation or stale results',async()=>{
 const history=new DirectHistory(),doc=history.document;doc.sketches.push(sketch());history.commit(doc)
 const stats=history.storageStats
 let finish!:(document:ReturnType<typeof emptyDirectDocument>)=>void
 let requested=''
 const pending=history.restoreAsync('undo',text=>{requested=text;return new Promise(resolve=>{finish=resolve})})
 expect(history.storageStats).toEqual(stats)
 history.cancelRestore();finish(parseDirectDocument(requested));expect(await pending).toBe(false)
 expect(history.storageStats).toEqual(stats)
 await expect(history.restoreAsync('undo',async()=>{throw Error('worker stopped')})).rejects.toThrow('worker stopped')
 expect(history.storageStats).toEqual(stats)
 const stale=history.restoreAsync('undo',text=>{requested=text;return new Promise(resolve=>{finish=resolve})})
 const edit=history.document;edit.sketches[0].name='New edit';history.commit(edit)
 finish(parseDirectDocument(requested));expect(await stale).toBe(false)
 expect(history.document.sketches[0].name).toBe('New edit');expect(history.canRedo).toBe(false)
})

it('accepts only the latest asynchronous history request and owns the restored document',async()=>{
 const history=new DirectHistory(),doc=history.document;doc.sketches.push(sketch());history.commit(doc)
 let finish!:(document:ReturnType<typeof emptyDirectDocument>)=>void
 let requested=''
 const old=history.restoreAsync('undo',text=>{requested=text;return new Promise(resolve=>{finish=resolve})})
 expect(await history.restoreAsync('undo',async text=>parseDirectDocument(text))).toBe(true)
 finish(parseDirectDocument(requested));expect(await old).toBe(false)
 expect(history.document.sketches).toHaveLength(0)
 let loaded!:ReturnType<typeof emptyDirectDocument>
 expect(await history.restoreAsync('redo',async text=>loaded=parseDirectDocument(text))).toBe(true)
 expect(history.storageStats.materializedStates).toBe(0)
 loaded.sketches[0].name='borrowed mutation'
 expect(history.document.sketches[0].name).toBe('L')
 const stats=history.storageStats
 await expect(history.restoreAsync('undo',async()=>history.document)).rejects.toThrow('different document')
 expect(history.storageStats).toEqual(stats)
 expect(history.storageStats.materializedStates).toBe(1)
})

it('commits a parsed async result atomically and ignores cancelled or superseded preparation',async()=>{
 const history=new DirectHistory(),candidate=emptyDirectDocument();candidate.sketches.push(sketch())
 let finish!:(value:typeof candidate)=>void
 const pending=history.commitAsync(()=>new Promise(resolve=>{finish=resolve}))
 expect(history.canUndo).toBe(false);history.cancelRestore();finish(candidate)
 expect(await pending).toBe(false);expect(history.document.sketches).toHaveLength(0)
 await expect(history.commitAsync(async()=>candidate,()=>{throw Error('locked')})).rejects.toThrow('locked')
 expect(history.canUndo).toBe(false)
 const old=history.commitAsync(()=>new Promise(resolve=>{finish=resolve}))
 expect(await history.commitAsync(async()=>candidate)).toBe(true)
 finish(emptyDirectDocument());expect(await old).toBe(false)
 candidate.sketches[0].name='external mutation'
 expect(history.document.sketches[0].name).toBe('L')
 expect(history.undo().sketches).toHaveLength(0)
 expect(history.redo().sketches[0].name).toBe('L')
})

it('resets a recovered baseline atomically without adding undo or sharing caller data',async()=>{
 const h=new DirectHistory(),next=emptyDirectDocument();next.sketches.push(sketch())
 h.commit(next)
 const recovered=emptyDirectDocument();recovered.sketches.push({...sketch(),name:'Recovered'})
 expect(await h.resetAsync(async()=>parseDirectDocument(JSON.stringify(recovered)))).toBe(true)
 expect(h.canUndo).toBe(false);expect(h.canRedo).toBe(false)
 recovered.sketches[0].name='Outside';expect(h.document.sketches[0].name).toBe('Recovered')
 const before=h.document
 await expect(h.resetAsync(async()=>{throw Error('corrupt recovery')})).rejects.toThrow('corrupt recovery')
 expect(h.document).toEqual(before)
})
it('ignores baseline recovery after cancellation or an intervening edit',async()=>{
 const h=new DirectHistory(),recovered=emptyDirectDocument();recovered.sketches.push(sketch())
 let complete!:(value:ReturnType<typeof emptyDirectDocument>)=>void
 const first=h.resetAsync(()=>new Promise(resolve=>complete=resolve));h.cancelRestore();complete(recovered)
 expect(await first).toBe(false);expect(h.document.sketches).toHaveLength(0)
 const second=h.resetAsync(()=>new Promise(resolve=>complete=resolve))
 const edited=h.document;edited.sketches.push({...sketch(),name:'New edit'});h.commit(edited);complete(recovered)
 expect(await second).toBe(false);expect(h.document.sketches[0].name).toBe('New edit');expect(h.canUndo).toBe(true)
})

it('preserves project identity after async restore without materializing old geometry during commit',async()=>{
 const base=emptyDirectDocument();base.blenderProjectId='project-identity';base.sketches.push(sketch())
 const history=new DirectHistory(base),next=history.document;next.sketches[0].name='changed';history.commit(next)
 expect(await history.restoreAsync('undo',async text=>parseDirectDocument(text))).toBe(true)
 expect(history.storageStats.materializedStates).toBe(0)
 const candidate=structuredClone(base);delete candidate.blenderProjectId;candidate.sketches[0].name='new edit'
 expect(await history.commitAsync(async()=>candidate,resolved=>{
  expect(history.storageStats.materializedStates).toBe(0)
  expect(resolved.blenderProjectId).toBe('project-identity')
 })).toBe(true)
 expect(history.document.blenderProjectId).toBe('project-identity')
 expect(history.undo().sketches[0].name).toBe('L')
})
