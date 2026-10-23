import {inspectNurbsBrep,type NurbsBrep} from './geometry/brep'
import type {StepCapabilityReport} from './cadStepRouting'
import type {DirectStepV10Document} from './cadNurbsStep'
import {MAX_RETAINED_STEP_STORE_BYTES,retainedStepStoreAcceptsEncodedBytes} from './cadStepRouting'
import {WORKSPACE_DATABASE_NAME,WORKSPACE_DATABASE_VERSION,WORKSPACE_OBJECT_STORE,WORKSPACE_STEP_MODEL_STORE} from './workspaceIndexedDb'

interface StepModelRow {slot:'active';version:1|2;model:NurbsBrep;document?:DirectStepV10Document;report:StepCapabilityReport}

const request=<T>(value:IDBRequest<T>)=>new Promise<T>((resolve,reject)=>{
  value.onsuccess=()=>resolve(value.result)
  value.onerror=()=>reject(value.error??Error('IndexedDB STEP model request failed'))
})
const complete=(transaction:IDBTransaction)=>new Promise<void>((resolve,reject)=>{
  transaction.oncomplete=()=>resolve()
  transaction.onabort=transaction.onerror=()=>reject(transaction.error??Error('IndexedDB STEP model transaction failed'))
})
const database=()=>new Promise<IDBDatabase>((resolve,reject)=>{
  const open=indexedDB.open(WORKSPACE_DATABASE_NAME,WORKSPACE_DATABASE_VERSION)
  open.onupgradeneeded=()=>{
    if(!open.result.objectStoreNames.contains(WORKSPACE_OBJECT_STORE))open.result.createObjectStore(WORKSPACE_OBJECT_STORE,{keyPath:'slot'})
    if(!open.result.objectStoreNames.contains(WORKSPACE_STEP_MODEL_STORE))open.result.createObjectStore(WORKSPACE_STEP_MODEL_STORE,{keyPath:'slot'})
  }
  open.onsuccess=()=>resolve(open.result)
  open.onerror=()=>reject(open.error??Error('Could not open IndexedDB STEP model store'))
})
const validate=(row:StepModelRow)=>{
  inspectNurbsBrep(row.model)
  const encoded=JSON.stringify(row)
  if(!retainedStepStoreAcceptsEncodedBytes(encoded))throw RangeError(`Retained STEP model exceeds ${MAX_RETAINED_STEP_STORE_BYTES} UTF-8 bytes`)
  return row
}

export async function saveProjectStepModel(model:NurbsBrep,report:StepCapabilityReport,document?:DirectStepV10Document):Promise<void>{
  const db=await database()
  try{
    const transaction=db.transaction(WORKSPACE_STEP_MODEL_STORE,'readwrite')
    await Promise.all([request(transaction.objectStore(WORKSPACE_STEP_MODEL_STORE).put(validate({slot:'active',version:2,model,document,report}))),complete(transaction)])
  }finally{db.close()}
}

export async function loadProjectStepModel():Promise<StepModelRow|undefined>{
  const db=await database()
  try{
    const transaction=db.transaction(WORKSPACE_STEP_MODEL_STORE,'readonly')
    const [row]=await Promise.all([request<StepModelRow|undefined>(transaction.objectStore(WORKSPACE_STEP_MODEL_STORE).get('active')),complete(transaction)])
    return row?validate(row):undefined
  }finally{db.close()}
}
