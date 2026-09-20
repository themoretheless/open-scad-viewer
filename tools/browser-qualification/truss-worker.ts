import {MainSolidWorkerClient} from '../../src/services/mainSolidWorkerClient'
import type {TrussModel, TrussResponse} from '../../src/services/trussAnalysis'

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

export async function run(workerUrl:string) {
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
      for(let i=0;i<20;i++)verify(model,await client.run({kind:'truss',model}))
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
    const controller=new AbortController()
    const aborted=client.run({kind:'truss',model:fixture(122)},{signal:controller.signal}).then(
      ()=>{throw Error('Cancelled call succeeded')},error=>{if(error.name!=='AbortError')throw error},
    )
    controller.abort();await aborted
    verify(small,await client.run({kind:'truss',model:small}))
    if(Number(created)!==2)throw Error('Cancellation did not replace the worker')
    return {scope:'production CAD worker round trip; includes clone, WASM and reply validation, not GPU upload or full UI',
      coldMs,warmups:20,samples:31,reports,animationFramesDuringWarmWork:frames,
      maxAnimationGapMs:Math.max(0,...frameTimes.slice(1).map((t,i)=>t-frameTimes[i])),
      workersCreated:created,singularCode:'TRUSS_SINGULAR',abortRecovery:true}
  } finally {cancelAnimationFrame(frameHandle);client.dispose()}
}
