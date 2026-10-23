import {describe, expect, it} from 'vitest'
import {withEditableSketchPath, editablePathAnchors} from '../src/services/editableSketchPath'
import {emptyDirectDocument, parseDirectDocument, DirectHistory, extrudeDirectSketch} from '../src/services/directModeling'
import {pathFromPolygon, smoothPath, setPathAnchorPosition, setAnchorHandle, dragAlignGuides} from '../src/services/geometry/path2d'
import {outlineSketchStroke,simplifySketchPath} from '../src/services/editableSketchOperations'
import {transformSketch} from '../src/services/directSketchGeometry'

const sketch = {id:'profile', name:'Bézier', closed:true, points:[]}
const create = () => withEditableSketchPath(sketch, smoothPath(pathFromPolygon([[0,0],[20,0],[20,20],[0,20]],true)))

describe('authored Bézier sketch profiles', () => {
  it('retains authored controls through save, undo and extrusion while rebuilding samples on load', () => {
    const original=create(), d=emptyDirectDocument();d.sketches.push(original)
    const history=new DirectHistory(d)
    const edited=withEditableSketchPath(original,setPathAnchorPosition(original.editablePath!,1,[30,0]))
    history.commit({...d,sketches:[edited]})
    expect(editablePathAnchors(history.document.sketches[0].editablePath!)[1]).toEqual([30,0])
    expect(history.undo().sketches[0].editablePath).toEqual(original.editablePath)
    expect(history.redo().sketches[0].editablePath).toEqual(edited.editablePath)
    const parsed=parseDirectDocument(JSON.stringify({...d,sketches:[{...edited,points:[[999,999],[998,999],[999,998]]}]}))
    expect(parsed.sketches[0].points).toEqual(edited.points)
    expect(extrudeDirectSketch(parsed.sketches[0],5,'body').mesh.indices.length).toBeGreaterThan(0)
    expect(original.editablePath).not.toEqual(edited.editablePath)
  })
  it('edits tangent controls independently of display samples and refuses competing definitions', () => {
    const original=create(), path=setAnchorHandle(original.editablePath!,1,'out',[25,5])
    const edited=withEditableSketchPath(original,path)
    expect(edited.points).not.toEqual(original.points)
    expect(() => withEditableSketchPath({...original,analytic:{kind:'circle',center:[0,0],radius:2,start:0,sweep:360}},path)).toThrow()
    expect(() => transformSketch(original,[1,0],0,1)).toThrow(/Bézier/)
    expect(() => withEditableSketchPath(original,{...path,start:[Infinity,0]})).toThrow()
  })
  it('outlines all loops as one region and simplifies only on explicit request', () => {
    const original=create(), before=structuredClone(original)
    const outline=outlineSketchStroke({id:'box',name:'Box',points:[[0,0],[20,0],[20,20],[0,20]],closed:true},1,'outline')
    expect(() => outlineSketchStroke(original,1,'dense')).toThrow(/256/)
    expect(outline.retainedProfile!.loops.length).toBe(2)
    expect(outline.editablePath).toBeUndefined()
    expect(parseDirectDocument(JSON.stringify({version:1,sketches:[outline],bodies:[]})).sketches[0].retainedProfile).toBeDefined()
    const simplified=simplifySketchPath(original,.1)
    expect(simplified.editablePath).toBeDefined()
    expect(original).toEqual(before)
    expect(() => simplifySketchPath(original,NaN)).toThrow()
    expect(() => outlineSketchStroke(original,0,'outline')).toThrow()
  })
  it('supports open curves and reports both screen guide axes', () => {
    const open=withEditableSketchPath({...sketch,closed:false},smoothPath(pathFromPolygon([[0,0],[10,5],[20,0]],false)))
    expect(open.closed).toBe(false)
    expect(editablePathAnchors(open.editablePath!)).toHaveLength(3)
    expect(() => extrudeDirectSketch(open,5,'body')).toThrow(/Close/)
    const guides=dragAlignGuides({min:[99,49],max:[99,49]},[{min:[100,50],max:[100,50]}],8)
    expect(guides.delta).toEqual([1,1])
    expect(guides.guides).toHaveLength(2)
  })
})
