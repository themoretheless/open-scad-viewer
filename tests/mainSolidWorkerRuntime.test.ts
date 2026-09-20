import {expect,it} from 'vitest'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import type {TrussModel} from '../src/services/trussAnalysis'

it('rejects overlapping requests during initialization and recovers after typed errors',async()=>{
  const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
  const model:TrussModel={nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],
    restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]}
  const first=handle({version:1,id:1,job:{kind:'truss',model}})
  await handle({version:1,id:2,job:{kind:'truss',model}})
  expect(messages[0]).toMatchObject({version:1,id:2,kind:'truss',ok:false,error:{code:'CAD_BUSY'}})
  await first
  expect(messages[1]).toMatchObject({version:1,id:1,kind:'truss',ok:true,result:{freeDofs:1}})
  const unstable={...model,restrained:[[true,true,true],[false,false,false]]}
  await handle({version:1,id:3,job:{kind:'truss',model:unstable}})
  expect(messages[2]).toMatchObject({id:3,ok:false,error:{name:'GeometryKernelError',code:'TRUSS_SINGULAR'}})
  await handle({version:1,id:4,job:{kind:'truss',model}})
  expect(messages[3]).toMatchObject({id:4,ok:true,result:{freeDofs:1}})
  for(const malformed of [null,{}, {version:2,id:5,job:{kind:'truss',model}},
    {version:1,id:-1,job:{kind:'truss',model}}, {version:1,id:5,job:{kind:'unknown'}}])await handle(malformed)
  expect(messages).toHaveLength(4)
})
