import {describe,it,expect} from 'vitest'
import {runDirectTransaction} from '../src/mcp/directTransaction'
import {emptyDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
describe('disposable MCP action transaction',()=>{
 it('executes outline and native extrusion together through the MCP boundary',async()=>{
  const doc=emptyDirectDocument();doc.sketches.push({id:'profile',name:'Profile',points:[[0,0],[20,0],[20,20],[0,20]],closed:true})
  const actions=[{kind:'outlineStroke',sourceId:'profile',createdId:'stroke',width:1},{kind:'extrusion',options:{sketchIds:['stroke'],height:3,offset:0,operation:'new',targetId:'',id:'solid'}}]
  const result=JSON.parse(await runDirectTransaction(serializeDirectDocument(doc),JSON.stringify({version:1,actions})))
  expect(result.sketches).toHaveLength(2);expect(result.bodies[0].brep).toBeDefined()
 },20000)
 it('returns a validated model and reports a failing action',async()=>{
  const document=serializeDirectDocument(emptyDirectDocument())
  const script=JSON.stringify({version:1,actions:[{kind:'primitive',options:{kind:'box',id:'cube',name:'Cube',size:2,topRadius:1,innerRadius:.5,round:'exact',segments:1}}]})
  expect(JSON.parse(await runDirectTransaction(document,script)).bodies).toHaveLength(1)
  await expect(runDirectTransaction(document,JSON.stringify({version:1,actions:[{kind:'missing'}]}))).rejects.toThrow(/Action 1/)
 },20000)
 it('rejects an already cancelled request without starting execution',async()=>{
  const abort=new AbortController();abort.abort()
  await expect(runDirectTransaction('{}','{}',abort.signal)).rejects.toMatchObject({name:'AbortError'})
 })
})
