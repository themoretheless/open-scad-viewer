import {MainSolidWorkerClient} from '../../src/services/mainSolidWorkerClient'
import type {TrussModel, TrussResponse} from '../../src/services/trussAnalysis'
import {resolveTrussScenario, type TrussScenario} from '../../src/services/trussScenario'

function fixture(freeNodes:number):TrussModel {
  const model:TrussModel={nodesMm:[[10,0,0],[0,10,0],[0,0,0]],members:[],
    restrained:[[true,true,true],[true,true,true],[true,true,true]],forcesN:[[0,0,0],[0,0,0],[0,0,0]]}
  for(let i=0;i<freeNodes;i++) {
    model.nodesMm.push([1,2,10+i/10]);model.restrained.push([false,false,false]);model.forcesN.push([1,-2,-3])
    for(let anchor=0;anchor<3;anchor++)model.members.push({nodes:[anchor,i+3],youngMpa:2000,areaMm2:2})
  }
  return model
}
function verify(model:TrussModel,result:TrussResponse) {
  if(result.freeDofs!==(model.nodesMm.length-3)*3)throw Error('Wrong free DOFs')
  for(let i=0;i<model.nodesMm.length-3;i++) {
    const z=10+i/10,total=-3/z,q0=(total-1)/10,q1=(2*total+2)/10
    const expected=[q0*Math.hypot(-9,2,z),q1*Math.hypot(1,-8,z),(total-q0-q1)*Math.hypot(1,2,z)]
    for(let k=0;k<3;k++)if(Math.abs(result.axialForcesN[i*3+k]-expected[k])>1e-9)throw Error('Analytical force mismatch')
  }
}

export async function run(workerUrl:string,warmups=200) {
  if(!Number.isSafeInteger(warmups)||warmups<0||warmups>1000)throw Error('warmups must be 0-1000')
  let created=0,frames=0,frameHandle=0
  const frameTimes:number[]=[]
  const tick=(now:number)=>{frames++;frameTimes.push(now);frameHandle=requestAnimationFrame(tick)}
  const client=new MainSolidWorkerClient(()=>{created++;return new Worker(workerUrl,{type:'module'})})
  try {
    const coldStart=performance.now(),small=fixture(1)
    verify(small,await client.run({kind:'truss',model:small}))
    const coldMs=performance.now()-coldStart
    // Measure main-thread animation progress only after cold initialization.
    frameHandle=requestAnimationFrame(tick)
    const reports=[]
    for(const freeNodes of [1,40,122]) {
      const model=fixture(freeNodes)
      for(let i=0;i<warmups;i++)verify(model,await client.run({kind:'truss',model}))
      const samplesMs=[]
      for(let i=0;i<31;i++) {
        const start=performance.now(),result=await client.run({kind:'truss',model})
        samplesMs.push(performance.now()-start);verify(model,result)
      }
      const ordered=[...samplesMs].sort((a,b)=>a-b)
      reports.push({nodes:model.nodesMm.length,members:model.members.length,freeDofs:freeNodes*3,
        p50Ms:ordered[15],p95Ms:ordered[29],samplesMs})
    }
    cancelAnimationFrame(frameHandle)
    if(frames<2)throw Error('Animation did not progress during warm worker calculations')
    if(created!==1)throw Error('Worker was not reused')
    const unstable=fixture(1);unstable.members.splice(1)
    try {await client.run({kind:'truss',model:unstable});throw Error('Expected singular refusal')}
    catch(error){if((error as {code?:string}).code!=='TRUSS_SINGULAR')throw error}
    if(created!==1)throw Error('Recoverable refusal discarded the worker')
    const {forcesN:_,...structure}=small
    const loads=[{nodes:[3],originMm:[0,0,0] as [number,number,number],forceN:[1,-2,-3] as [number,number,number],momentNmm:[14,13,-4] as [number,number,number]}]
    verify(small,await client.run({kind:'truss',model:{...structure,loads}}))
    loads[0].momentNmm[2]+=1
    try {await client.run({kind:'truss',model:{...structure,loads}});throw Error('Expected unrealizable-load refusal')}
    catch(error){if((error as {code?:string}).code!=='TRUSS_LOAD_UNREALIZABLE')throw error}
    loads[0].momentNmm[2]-=1
    verify(small,await client.run({kind:'truss',model:{...structure,loads}}))
    if(created!==1)throw Error('Recoverable wrench refusal discarded the worker')
    const scenario:TrussScenario={nodesMm:structure.nodesMm,members:structure.members,
      cases:['first','second'].map(id=>({id,restrained:structuredClone(structure.restrained),loads:structuredClone(loads)})),
      combinations:[{id:'combined',terms:[{caseId:'first',factor:1.5},{caseId:'second',factor:-0.5}]}],activeId:'combined'}
    const resolved=resolveTrussScenario(scenario)
    scenario.cases[0].loads[0].forceN[0]=99
    verify(small,await client.run({kind:'truss',model:resolved.model}))
    scenario.cases[1].restrained[3][0]=true
    try {resolveTrussScenario(scenario);throw Error('Expected incompatible-support refusal')}
    catch(error){if((error as {code?:string}).code!=='TRUSS_SCENARIO_SUPPORTS')throw error}
    if(created!==1)throw Error('Scenario computation did not reuse the worker')
    const controller=new AbortController()
    const aborted=client.run({kind:'truss',model:fixture(122)},{signal:controller.signal}).then(
      ()=>{throw Error('Cancelled call succeeded')},error=>{if(error.name!=='AbortError')throw error},
    )
    controller.abort();await aborted
    verify(small,await client.run({kind:'truss',model:small}))
    if(Number(created)!==2)throw Error('Cancellation did not replace the worker')
    return {scope:'production CAD worker round trip; includes clone, WASM and reply validation, not GPU upload or full UI',
      coldMs,warmups,samples:31,reports,animationFramesDuringWarmWork:frames,
      maxAnimationGapMs:Math.max(0,...frameTimes.slice(1).map((t,i)=>t-frameTimes[i])),
      workersCreated:created,singularCode:'TRUSS_SINGULAR',wrenchRecovery:true,scenarioSnapshotAndSupportRefusal:true,abortRecovery:true}
  } finally {cancelAnimationFrame(frameHandle);client.dispose()}
}
