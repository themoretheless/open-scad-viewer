import {encodeBinary,decodeBinary} from './valueBinaryCodec'
import {sourceBodyExpectation,restoreSourceBody,validSourceBody,type SourceBodyOptions} from './sourceBody'
/** Persist original native inputs. No saved admission or display cache is authority. */
export interface SourceBodyRecord {id:string;name:string;group?:string;source:{encoding:'MGV1/base64';data:string}}
const LIMIT=32*1024*1024
function base64(bytes:Uint8Array):string {
 const chunks:string[]=[]
 for(let at=0;at<bytes.length;at+=16384)chunks.push(String.fromCharCode(...bytes.subarray(at,at+16384)))
 return btoa(chunks.join(''))
}
export function createSourceBodyRecord(id:string,name:string,options:SourceBodyOptions):SourceBodyRecord {
 sourceBodyExpectation(options)
 return {id,name,source:{encoding:'MGV1/base64',data:base64(encodeBinary(options))}}
}
export function isSourceBodyRecordPayload(value:unknown):value is SourceBodyRecord {
 const r=value as SourceBodyRecord
 return !!r&&typeof r.id==='string'&&typeof r.name==='string'&&r.name.length<=100
  &&(r.group===undefined||(typeof r.group==='string'&&r.group.length>0&&r.group.length<=100))
  &&r.source?.encoding==='MGV1/base64'&&typeof r.source.data==='string'
  &&r.source.data.length>0&&r.source.data.length<=4*Math.ceil(LIMIT/3)&&r.source.data.length%4===0
  &&/^[A-Za-z0-9+/]*={0,2}$/.test(r.source.data)
}
export function sourceBodyRecordOptions(record:SourceBodyRecord):SourceBodyOptions {
 const source=record?.source
 if(source?.encoding!=='MGV1/base64'||typeof source.data!=='string'||source.data.length>4*Math.ceil(LIMIT/3)
  ||source.data.length%4!==0||!source.data.length||!/^[A-Za-z0-9+/]*={0,2}$/.test(source.data))throw Error('Invalid source Body archive')
 const text=atob(source.data)
 if(text.length>LIMIT)throw Error('Source Body archive exceeds 32 MiB')
 const bytes=Uint8Array.from(text,c=>c.charCodeAt(0))
 if(base64(bytes)!==source.data)throw Error('Noncanonical source Body archive')
 const options=decodeBinary(bytes) as SourceBodyOptions
 sourceBodyExpectation(options)
 return options
}
export function qualifySourceBodyRecord(record:SourceBodyRecord) {
 let diagnostics:import('./sourceBody').SourceBodyResult['diagnostics']|undefined
 try {
  const options=sourceBodyRecordOptions(record),result=restoreSourceBody(options)
  diagnostics=result.diagnostics
  if(!validSourceBody(sourceBodyExpectation(options),result)||!result.admitted)
   throw Error(result.diagnostics?.reason??'invalid restoration payload')
  return result
 }catch(cause){
  const message=cause instanceof Error?cause.message:String(cause)
  const error=new Error(`Source body ${record.id}: ${message}`,{cause})
  Object.assign(error,{code:'CAD_SOURCE_BODY_RESTORE',sourceBodyId:record.id,diagnostics})
  throw error
 }
}
