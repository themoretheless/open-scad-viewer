import {it,expect} from 'vitest'
import {emptyDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {createSolidInstance} from '../src/services/solidInstances'
import {exportSolidBlenderSnapshot} from '../src/services/solidBlenderExchange'
it('exports current instance geometry and stable project/body identities',()=>{
 const document=emptyDirectDocument(),brep=createBrepBox([0,0,0],[2,3,4])
 document.bodies.push({id:'source',name:'Part',material:{name:'Copper',color:'#b87333',metallic:1,roughness:.2},brep,mesh:tessellateNurbsBrep(brep,1)})
 const linked=createSolidInstance(document,'source','instance',[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const output=JSON.parse(exportSolidBlenderSnapshot(linked,'project-a'))
 expect(output.bodies[0].material).toEqual({name:'Copper',color:'#b87333',metallic:1,roughness:.2})
 expect(output.schema).toBe('openscad-viewer/blender-1');expect(output.projectId).toBe('project-a')
 expect(output.bodies.map((body:any)=>body.id)).toEqual(['source','instance'])
 expect(Math.max(...output.bodies[1].mesh.positions.filter((_:number,i:number)=>i%3===0))).toBe(12)
 expect(exportSolidBlenderSnapshot(linked,'project-a')).toBe(JSON.stringify(output))
})
it('refuses partial curve/sketch export and missing project identity',()=>{
 const document=emptyDirectDocument();document.sketches.push({id:'s',name:'Sketch',closed:false,points:[[0,0],[1,0]]})
 expect(()=>exportSolidBlenderSnapshot(document,'project-a')).toThrow('body-only')
 expect(()=>exportSolidBlenderSnapshot(emptyDirectDocument(),'')).toThrow('stable project ID')
})

it('keeps export identity across existing undo/redo, edits and reload without adding an undo step',async()=>{
 const {DirectHistory,serializeDirectDocument,parseDirectDocument}=await import('../src/services/directModeling')
 const history=new DirectHistory()
 const changed=history.document;changed.groups=[{name:'Group',source:''}];history.commit(changed)
 history.undo()
 const id=history.ensureBlenderProjectId()
 expect(history.canUndo).toBe(false);expect(history.canRedo).toBe(true)
 expect(history.redo().blenderProjectId).toBe(id)
 expect(history.undo().blenderProjectId).toBe(id)
 const reloaded=new DirectHistory(parseDirectDocument(serializeDirectDocument(history.document)))
 expect(reloaded.ensureBlenderProjectId()).toBe(id)
 const edit=reloaded.document;delete edit.blenderProjectId;edit.groups=[{name:'Next',source:''}];reloaded.commit(edit)
 expect(reloaded.document.blenderProjectId).toBe(id)
 expect(()=>parseDirectDocument(JSON.stringify({...emptyDirectDocument(),blenderProjectId:42}))).toThrow('identity')
})
