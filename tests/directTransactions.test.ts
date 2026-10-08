import {describe,it,expect} from 'vitest'
import {executeDirectTransaction,appendDirectCheckpoint,appendDirectDocumentEdit} from '../src/services/directTransactions'
import {emptyDirectDocument,DirectHistory,serializeDirectDocument} from '../src/services/directModeling'
const box={kind:'primitive',options:{kind:'box',id:'seed',name:'Seed',size:2,topRadius:1,innerRadius:.5,round:'exact',segments:1}}
const move={kind:'sceneEdit',options:{operation:'transform',ids:['seed'],id:'seed',createdId:'',x:10,y:0,z:0,axis:'z',angle:0,scale:1}}
const batch=(actions:unknown[])=>JSON.stringify({version:1,actions})
describe('atomic direct actions',()=>{
 it('outlines and extrudes as one transaction, and rolls back invalid extrusion',()=>{
  const before=emptyDirectDocument();before.sketches.push({id:'profile',name:'Profile',points:[[0,0],[20,0],[20,20],[0,20]],closed:true})
  const actions=[{kind:'outlineStroke',sourceId:'profile',createdId:'stroke',width:1},{kind:'extrusion',options:{sketchIds:['stroke'],height:3,offset:0,operation:'new',targetId:'',id:'solid'}}]
  const next=executeDirectTransaction(before,batch(actions)),history=new DirectHistory(before)
  expect(next.bodies[0].brep).toBeDefined();expect(next.sketches).toHaveLength(2)
  history.commit(next);expect(history.undo().bodies).toHaveLength(0);expect(history.document.sketches).toHaveLength(1)
  expect(()=>executeDirectTransaction(before,batch([actions[0],{...actions[1],options:{...actions[1].options,height:0}}]))).toThrow(/Action 2/)
  expect(before.sketches).toHaveLength(1);expect(before.bodies).toHaveLength(0)
 })
 it('publishes a whole batch as one undo step',()=>{
  const before=emptyDirectDocument(),next=executeDirectTransaction(before,batch([box,move]))
  expect(before.bodies).toHaveLength(0);expect(next.bodies).toHaveLength(1)
  expect(next.bodies[0].mesh.positions[0]).toBeGreaterThan(8)
  const history=new DirectHistory();history.commit(next);expect(history.undo().bodies).toHaveLength(0);expect(history.redo().bodies).toHaveLength(1)
 })
 it('rolls back an earlier successful operation when a later operation fails',()=>{
  const before=emptyDirectDocument(),snapshot=serializeDirectDocument(before)
  expect(()=>executeDirectTransaction(before,batch([box,{kind:'unsupported'}]))).toThrow(/Action 2/)
  expect(serializeDirectDocument(before)).toBe(snapshot)
 })
 it('records changes as checkpoints, skips duplicate snapshots and replays them',()=>{
  const model=executeDirectTransaction(emptyDirectDocument(),batch([box]))
  const recording=appendDirectCheckpoint({version:1,actions:[]},model)
  expect(appendDirectCheckpoint(recording,model)).toBe(recording)
  expect(serializeDirectDocument(executeDirectTransaction(emptyDirectDocument(),JSON.stringify(recording)))).toBe(serializeDirectDocument(model))
  expect(()=>executeDirectTransaction(model,batch(Array(81).fill(box)))).toThrow(/80/)
 })
})

describe('recorded entity commands',()=>{
 it('replays creation, changes, deletion and reordering without repeated checkpoints',()=>{
  const base=emptyDirectDocument();let script=appendDirectCheckpoint({version:1,actions:[]},base)
  const a=executeDirectTransaction(base,batch([box]));script=appendDirectDocumentEdit(script,base,a)
  const b=executeDirectTransaction(a,batch([move]));script=appendDirectDocumentEdit(script,a,b)
  const c=structuredClone(b);c.sketches=[{id:'second',name:'B',closed:false,points:[[0,0],[1,0]]},{id:'first',name:'A',closed:false,points:[[0,0],[2,0]]}];script=appendDirectDocumentEdit(script,b,c)
  const d=structuredClone(c);d.sketches.reverse();d.bodies=[];script=appendDirectDocumentEdit(script,c,d)
  expect(script.actions.slice(1).every(a=>a.kind==='documentEdit')).toBe(true)
  expect(JSON.parse(serializeDirectDocument(executeDirectTransaction(base,JSON.stringify(script))))).toEqual(JSON.parse(serializeDirectDocument(d)))
  expect(appendDirectDocumentEdit(script,d,d)).toBe(script)
  const bad=structuredClone(a);bad.bodies[0].name='changed';const one=appendDirectDocumentEdit({version:1,actions:[]},a,b)
  expect(()=>executeDirectTransaction(bad,JSON.stringify(one))).toThrow(/no longer matches/);expect(bad.bodies[0].name).toBe('changed')
 })
})
