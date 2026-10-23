import {describe,it,expect} from 'vitest'
import {emitPolygonMeshGcodeJob,inspectGcode} from '../src/services/geometry/polygon'
import {checkGcodePreviewJob} from '../src/services/gcodePreviewProtocol'
const positions=Float64Array.from([0,0,0,10,0,0,10,10,0,0,10,0,0,0,2,10,0,2,10,10,2,0,10,2]);
const indices=Uint32Array.from([0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]);
describe('configured print jobs with current WASM',()=>{
 it('preserves geometry across eight coordinate modes',()=>{
  let baseline:number|undefined;
  for(const inches of [false,true])for(const relativeXyz of [false,true])for(const relativeE of [false,true]){
   const result=emitPolygonMeshGcodeJob({positions,indices},0,2,{inches,relativeXyz,relativeE,zHopMm:0.5,startTemplate:'G91\nG20\nM83'});
   expect(result.gcode).toContain('configured-job 1');expect(result.gcode3mfBase64.length).toBeGreaterThan(100);
   if(baseline===undefined)baseline=result.preview.printDistanceMm;
   expect(result.preview.printDistanceMm).toBeCloseTo(baseline,3);
   expect(result.preview.moves.filter(m=>m.extruded).every(m=>m.z<=2.00001)).toBe(true);
  }
 });
 it('transports per-tool targets and explicit unverified lines',()=>{
  const result=inspectGcode(';FLAVOR:Marlin\nM104 T1 S210\nT0\nM109 R180\nM207 S0.8\nG10\nG11\nG1 X10 Y0 Z0 E1 F600');
  expect(result.firmware?.tools?.find(t=>t.id===1)?.targetC).toBe(210);
  expect(result.firmware?.tools?.find(t=>t.id===0)?.targetC).toBe(180);
  expect(result.firmware?.events?.some(e=>e.wait==='HeatingOrCooling')).toBe(true);
 });
 it('validates template and coordinate settings before worker dispatch',()=>{
  const mesh={vertices:new Float32Array(18),indices:Uint32Array.from([0,1,2]),transform:Float32Array.from([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])};
  const job={kind:'job' as const,mesh,zMin:0,zMax:2,settings:{relativeXyz:true,startTemplate:'G91',zHopMm:0.5,chamberTempC:0}};
  expect(()=>checkGcodePreviewJob(job)).not.toThrow();
  expect(()=>checkGcodePreviewJob({...job,settings:{startTemplate:'\0'}})).toThrow();
  expect(()=>checkGcodePreviewJob({...job,settings:{zHopMm:101}})).toThrow();
  expect(()=>checkGcodePreviewJob({...job,settings:{chamberTempC:151}})).toThrow();
 });
});
