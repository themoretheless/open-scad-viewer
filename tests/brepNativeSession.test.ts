import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {semanticResultItems} from '../src/core/semanticProgram'
import {lowerOpenSCADToSemanticProgram} from '../src/services/semanticProgramLowerer'
import {analyzeNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
const run=(args:Record<string,unknown>)=>callGeometryRust<Record<string,unknown>>('brep_session',args)
const solidType={space:'d3',geometryKind:'solid',representation:'analytic-brep',evidence:{tag:'representation-preserving'}}
const node={id:0,kind:'box',valueType:solidType,size:[1,1,1],center:false}
it('steps an authored graph through native wire ownership and commits once',()=>{
 const {core}=lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\ntranslate([2,0,0]) cube(2);').program
 const session=run({action:'graph-begin',maxNodes:core.nodes.length,maxBytes:8_000_000,nodes:core.nodes,execution:core.execution,result:core.result,occurrences:core.occurrences}).session
 expect(session).toEqual(expect.any(String))
 let consumed=false
 try{
  for(let completed=1;completed<=core.nodes.length;completed++){
   expect(run({action:'graph-advance',session,limit:1})).toEqual({tag:completed===core.nodes.length?'ready':'progress',completedNodes:completed,totalNodes:core.nodes.length})
  }
  const report=run({action:'graph-commit',session});consumed=true
  expect(report.tag).toBe('committed')
  expect(report.outcomes).toHaveLength(1)
  expect(report.resultItems).toEqual([{reference:semanticResultItems(core.result)[0],outcomeIndex:0}])
  expect(()=>run({action:'graph-advance',session,limit:1})).toThrow(/Unknown/)
 }finally{if(!consumed)run({action:'dispose',session})}
})
it('cancels a native graph between steps without publishing outputs',()=>{
 const session=run({action:'graph-begin',maxNodes:8,maxBytes:1_000_000,nodes:[node,{...node,id:1}],outputs:[0,1]}).session
 run({action:'graph-advance',session,limit:1})
 run({action:'dispose',session})
 expect(()=>run({action:'graph-commit',session})).toThrow(/Unknown/)
})
it('selects authored SemanticResult outputs and metadata inside Rust',()=>{
 const {core}=lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\ncolor("red") cube(2); translate([4,0,0]) cube(1);').program
 const args={maxNodes:core.nodes.length,maxBytes:8_000_000,nodes:core.nodes,execution:core.execution,result:core.result,occurrences:core.occurrences}
 const report=callGeometryRust<{tag:string;outcomes:unknown[];resultItems:{reference:unknown;outcomeIndex:number}[]}>('brep_graph_report',args)
 expect(report.tag).toBe('committed')
 expect(report.resultItems.map(item=>item.reference)).toEqual(semanticResultItems(core.result))
 expect(report.resultItems.map(item=>item.outcomeIndex)).toEqual([0,1])
 expect(report.outcomes).toHaveLength(2)
 const failed=callGeometryRust<Record<string,unknown>>('brep_graph_report',{...args,result:{tag:'multi',items:[...semanticResultItems(core.result)].reverse()}})
 expect(failed).toMatchObject({tag:'failed',completedNodes:0,code:'BREP_SEMANTIC_CONTRACT'})
 expect(failed).not.toHaveProperty('outcomes')
})
it('executes an authored SemanticProgram plan natively with exact geometry',()=>{
 const lowered=lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\ndifference(){cube(3);cube(1);}')
 const core=lowered.program.core
 const outputs=semanticResultItems(core.result).map(item=>item.node)
 const native=callGeometryRust<{tag:string;outcomes:{geometry:{model:NurbsBrep}}[]}>('brep_graph_report',{maxNodes:core.nodes.length,maxBytes:8_000_000,nodes:core.nodes,outputs,execution:core.execution})
 expect(native.tag).toBe('committed')
 expect(native.outcomes[0].geometry.model.bodies).toHaveLength(1)
 expect(analyzeNurbsBrep(native.outcomes[0].geometry.model).signedVolumeMm3).toBeCloseTo(26,6)
 const rejected=callGeometryRust<Record<string,unknown>>('brep_graph_report',{maxNodes:core.nodes.length,maxBytes:8_000_000,nodes:core.nodes,outputs,execution:{...core.execution,evaluationOrder:[...core.execution.evaluationOrder].reverse()}})
 expect(rejected).toMatchObject({tag:'failed',completedNodes:0,node:null,code:'BREP_SEMANTIC_CONTRACT'})
 expect(rejected).not.toHaveProperty('outcomes')
})
it('reports the native graph failure location without publishing its valid prefix',()=>{
 const report=callGeometryRust<Record<string,unknown>>('brep_graph_report',{maxNodes:8,maxBytes:1_000_000,outputs:[0],nodes:[node,{...node,id:1,size:[-1,1,1]}]})
 expect(report).toMatchObject({tag:'failed',node:1,completedNodes:1})
 expect(report.code).toEqual(expect.any(String))
 expect(report).not.toHaveProperty('outcomes')
 const success=callGeometryRust<Record<string,unknown>>('brep_graph_report',{maxNodes:8,maxBytes:1_000_000,outputs:[0],nodes:[node]})
 expect(success).toMatchObject({tag:'committed',completedNodes:1})
 expect(success.outcomes).toHaveLength(1)
})
it('executes a complete geometry DAG in Rust with ordered typed outputs',()=>{
 const valueType={...solidType,geometryKind:'solid-set'}
 const result=callGeometryRust<{outcomes:{tag:string;valueType:unknown;geometry?:{model:{bodies:unknown[]}}}[]}>('brep_graph',{
  maxNodes:8,maxBytes:1_000_000,outputs:[2,1],nodes:[node,
   {id:1,kind:'boolean',operation:'difference',inputs:[0,0],valueType},
   {id:2,kind:'boolean',operation:'union',inputs:[1,0],valueType}],
 })
 expect(result.outcomes.map(value=>value.tag)).toEqual(['value','empty'])
 expect(result.outcomes[0].geometry?.model.bodies).toHaveLength(1)
 expect(result.outcomes[1]).toEqual({tag:'empty',valueType})
 expect(()=>callGeometryRust('brep_graph',{maxNodes:8,maxBytes:1_000_000,outputs:[0],nodes:[node,{...node,id:1,size:[-1,1,1]}]})).toThrow()
})
it('owns empty Boolean and extrusion results without host empty reduction',()=>{
 const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 try{
  const valueType={...solidType,space:'d2',geometryKind:'region'}
  const empty=run({action:'evaluate',session,node:{id:0,kind:'boolean',operation:'union',inputs:[],valueType},inputs:[]}).lease
  expect(run({action:'snapshot',session,lease:empty})).toEqual({tag:'empty',valueType})
  const lease=run({action:'evaluate',session,node:{id:1,kind:'linear-extrude',input:0,valueType:solidType,height:2,center:false,twistDegrees:0,scale:[1,1]},inputs:[empty]}).lease
  run({action:'commit',session,retained:[lease]})
  expect(run({action:'snapshot',session,lease})).toEqual({tag:'empty',valueType:solidType})
 }finally{run({action:'dispose',session})}
})
it('rejects reordered reduced operands against the original native node',()=>{
 const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 try{
  const a=run({action:'evaluate',session,node,inputs:[]}).lease
  const b=run({action:'evaluate',session,node:{...node,id:1},inputs:[]}).lease
  expect(()=>run({action:'evaluate',session,node:{id:2,kind:'boolean',operation:'difference',inputs:[0,1],valueType:solidType},inputNodeIndices:[1,0],inputs:[b,a]})).toThrow(/input order differs/)
  expect(()=>run({action:'commit',session,retained:[a]})).toThrow()
 }finally{run({action:'dispose',session})}
})
it('enforces carrier and evidence admission without the TypeScript backend',()=>{
 for(const valueType of [{space:'d3'},{...solidType,representation:'mesh'},{...solidType,evidence:{tag:'certified'}}]){
  const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
  try{
   expect(()=>run({action:'evaluate',session,node:{...node,valueType},inputs:[]})).toThrow(/Unsupported B-rep semantic carrier or evidence/)
   expect(()=>run({action:'commit',session,retained:[]})).toThrow()
  }finally{run({action:'dispose',session})}
 }
})
it('admits disconnected bodies only for a solid-set in the native owner',()=>{
 for(const geometryKind of ['solid','solid-set']){
  const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
  try{
   const a=run({action:'evaluate',session,node,inputs:[]}).lease
   const b=run({action:'evaluate',session,node:{id:1,kind:'transform',input:0,valueType:solidType,matrix:[1,0,0,0,0,1,0,0,0,0,1,0,3,0,0,1]},inputs:[a]}).lease
   const evaluate=()=>run({action:'evaluate',session,node:{id:2,kind:'boolean',operation:'union',inputs:[0,1],valueType:{...solidType,geometryKind}},inputs:[a,b]})
   if(geometryKind==='solid'){
    expect(evaluate).toThrow(/does not satisfy its solid carrier/)
    expect(()=>run({action:'commit',session,retained:[a]})).toThrow()
   }else{
    const lease=evaluate().lease
    const snapshot=run({action:'snapshot',session,lease}) as {geometry:{model:{bodies:unknown[]}}}
    expect(snapshot.geometry.model.bodies).toHaveLength(2)
    run({action:'commit',session,retained:[lease]})
   }
  }finally{run({action:'dispose',session})}
 }
})
it('retains immutable native results across commit and revokes disposed wire handles',()=>{
 const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 let disposed=false
 try{
  const lease=run({action:'evaluate',session,node,inputs:[]}).lease
  const before=run({action:'snapshot',session,lease})
  expect(before.tag).toBe('value')
  run({action:'commit',session,retained:[lease]})
  expect(run({action:'snapshot',session,lease})).toEqual(before)
  run({action:'dispose',session});disposed=true
  expect(()=>run({action:'snapshot',session,lease})).toThrow(/Unknown native session/)
 }finally{if(!disposed)run({action:'dispose',session})}
})
it('rejects another native session’s lease without harming the owner',()=>{
 const a=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 const b=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 try{
  const lease=run({action:'evaluate',session:a,node,inputs:[]}).lease
  expect(()=>run({action:'evaluate',session:b,node,inputs:[lease]})).toThrow(/Foreign or unknown/)
  expect(run({action:'snapshot',session:a,lease}).tag).toBe('value')
  expect(()=>run({action:'evaluate',session:b,node,inputs:[]})).toThrow(/closed or failed/)
 }finally{run({action:'dispose',session:a});run({action:'dispose',session:b})}
})
it('preserves typed empty outcomes through native wire commit',()=>{
 const session=run({action:'begin',maxNodes:8,maxBytes:1_000_000}).session
 try{
  const a=run({action:'evaluate',session,node,inputs:[]}).lease
  const valueType={...solidType,geometryKind:'solid-set'}
  const lease=run({action:'evaluate',session,node:{id:1,kind:'boolean',valueType,operation:'difference',inputs:[0,0]},inputs:[a,a]}).lease
  expect(run({action:'snapshot',session,lease})).toEqual({tag:'empty',valueType})
  run({action:'commit',session,retained:[lease]})
  expect(run({action:'snapshot',session,lease})).toEqual({tag:'empty',valueType})
  expect(()=>run({action:'snapshot',session,lease:a})).toThrow(/not retained/)
 }finally{run({action:'dispose',session})}
})
