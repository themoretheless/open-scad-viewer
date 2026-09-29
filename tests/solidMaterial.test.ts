import {stringifyMeshJson} from '../src/services/meshJson'
import {expect,it} from 'vitest'
import {assignSolidMaterial} from '../src/services/solidMaterial'
import {DirectHistory,parseDirectDocument} from '../src/services/directModeling'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {transformSelection,splitSolid} from '../src/services/directSolidTools'
it('preserves material through history, reload and geometric editing',()=>{
 const body=cadRoadmapParts()[3].body
 const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
 const material={name:'Copper',color:'#cc7744'}
 history.commit(assignSolidMaterial(history.document,[body.id],material))
 material.color='#000000'
 expect(history.undo().bodies[0].material).toBeUndefined()
 expect(history.redo().bodies[0].material?.color).toBe('#cc7744')
 const restored=parseDirectDocument(stringifyMeshJson(history.document))
 const moved=transformSelection(restored,[body.id],[1,0,0],[0,0,1],0,1)
 expect(moved.bodies[0].material).toEqual({name:'Copper',color:'#cc7744'})
 for(const part of splitSolid(moved.bodies[0],[0,0,1],2))expect(part.material).toEqual(moved.bodies[0].material)
 expect(assignSolidMaterial(restored,[body.id]).bodies[0].material).toBeUndefined()
 expect(restored.bodies[0].material).toBeDefined()
})
it('rejects malformed material at the saved-document boundary',()=>{
 const body=cadRoadmapParts()[3].body
 for(const material of [null,{name:'x',color:'red'},{name:'',color:'#ffffff'},{name:'x',color:'#123456;url(x)'}]){
  expect(()=>parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[{...body,material}]} as never))).toThrow('Invalid body material')
 }
})

it('persists bounded metallic and roughness parameters through undo and reload',()=>{
 const body=cadRoadmapParts()[3].body,history=new DirectHistory({version:1,sketches:[],bodies:[body]})
 history.commit(assignSolidMaterial(history.document,[body.id],{name:'Metal',color:'#cc7744',metallic:1,roughness:.2}))
 expect(parseDirectDocument(stringifyMeshJson(history.document)).bodies[0].material).toMatchObject({metallic:1,roughness:.2})
 expect(history.undo().bodies[0].material).toBeUndefined()
 expect(history.redo().bodies[0].material?.roughness).toBe(.2)
 for(const value of [-1,1.01,NaN,Infinity])expect(()=>assignSolidMaterial(history.document,[body.id],{name:'Bad',color:'#ffffff',metallic:value})).toThrow('Invalid body material')
 expect(()=>parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[{...body,material:{name:'Bad',color:'#ffffff',roughness:2}}]}))).toThrow('Invalid body material')
})
