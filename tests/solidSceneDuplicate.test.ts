import {expect,it} from 'vitest'
import {applySolidSceneEdit,type SolidSceneEditOptions} from '../src/services/solidSceneEdit'
import {emptyDirectDocument} from '../src/services/directModeling'

it('rejects duplicate requests with missing, repeated or occupied identities before geometry work',()=>{
 const document=emptyDirectDocument()
 document.sketches.push({id:'profile',name:'Profile',closed:true,points:[[0,0],[1,0],[0,1]]})
 const options:SolidSceneEditOptions={operation:'duplicate',id:'profile',ids:['profile'],createdId:'',x:10,y:10,z:0,axis:'z',angle:0,scale:1}
 for(const override of [{},{copyIds:[]},{copyIds:['profile']},{ids:['missing'],copyIds:['new']},{ids:['profile','profile'],copyIds:['new','other']}]){
  expect(()=>applySolidSceneEdit(document,{...options,...override})).toThrow('Invalid duplicate identities.')
 }
 expect(document.sketches).toHaveLength(1)
 expect(document.sketches[0].id).toBe('profile')
})
