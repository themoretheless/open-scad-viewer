import {MAX_DOCUMENT_CHARACTERS} from '../core/directDocumentLimits'
import {withSolidDraftLease} from './solidDraftLease'
const databaseName='scad-solid-draft-snapshots-v1',storeName='snapshots'
interface Snapshot {id:string;text:string;createdAt?:number;conflictOf?:string}
const CONFLICT_RETENTION_MS=30*24*60*60*1000
const request=<T>(r:IDBRequest<T>)=>new Promise<T>((resolve,reject)=>{r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error??Error('Draft storage request failed.'))})
const complete=(t:IDBTransaction)=>new Promise<void>((resolve,reject)=>{t.oncomplete=()=>resolve();t.onabort=t.onerror=()=>reject(t.error??Error('Draft storage transaction failed.'))})
async function database():Promise<IDBDatabase>{
 if(typeof indexedDB==='undefined')throw Error('IndexedDB is unavailable. Download JSON to preserve this document.')
 const r=indexedDB.open(databaseName,1)
 r.onupgradeneeded=()=>r.result.createObjectStore(storeName,{keyPath:'id'})
 return request(r)
}
export function solidDraftSnapshotId(text:string|null):string|null {
 if(!text||text.length>200)return null
 try{const marker=JSON.parse(text);return marker?.version===1&&typeof marker.solidDraftId==='string'&&/^[a-zA-Z0-9-]{1,100}$/.test(marker.solidDraftId)?marker.solidDraftId:null}catch{return null}
}
export async function readSolidDraftSnapshot(marker:string):Promise<string>{
 const id=solidDraftSnapshotId(marker);if(!id)throw Error('Invalid draft snapshot reference.')
 const db=await database()
 try{
  const tx=db.transaction(storeName,'readonly'),[row]=await Promise.all([request<Snapshot|undefined>(tx.objectStore(storeName).get(id)),complete(tx)])
  if(!row||typeof row.text!=='string'||row.text.length>MAX_DOCUMENT_CHARACTERS)throw Error('Saved draft snapshot is missing or invalid. Import a JSON backup.')
  return row.text
 }finally{db.close()}
}
export async function removeSolidDraftSnapshot(marker:string|null):Promise<void>{
 const id=solidDraftSnapshotId(marker);if(!id)return
 const db=await database()
 try{const tx=db.transaction(storeName,'readwrite');await Promise.all([request(tx.objectStore(storeName).delete(id)),complete(tx)])}finally{db.close()}
}
/** An immutable snapshot is durable before its localStorage reference is published. */
export async function writeSolidDraftSnapshot(text:string):Promise<string>{
 if(text.length>MAX_DOCUMENT_CHARACTERS)throw Error('Document exceeds 64 MB.')
 const id=crypto.randomUUID(),db=await database()
 try{const tx=db.transaction(storeName,'readwrite');await Promise.all([request(tx.objectStore(storeName).add({id,text,createdAt:Date.now()})),complete(tx)])}finally{db.close()}
 return JSON.stringify({version:1,solidDraftId:id})
}

/** Keep a tab's rejected version durable after DRAFT_CONFLICT so both versions survive a crash. */
export async function writeSolidDraftConflict(key:string,text:string,now=Date.now()):Promise<string>{
 if(text.length>MAX_DOCUMENT_CHARACTERS)throw Error('Document exceeds 64 MB.')
 const id=crypto.randomUUID(),db=await database()
 try{const tx=db.transaction(storeName,'readwrite',{durability:'strict'});await Promise.all([request(tx.objectStore(storeName).add({id,text,createdAt:now,conflictOf:key})),complete(tx)])}finally{db.close()}
 return id
}
/** Conflict copies for `key`, newest first. */
export async function listSolidDraftConflicts(key:string):Promise<Array<{id:string;text:string;createdAt:number}>>{
 const db=await database()
 try{
  const tx=db.transaction(storeName,'readonly'),[rows]=await Promise.all([request<Snapshot[]>(tx.objectStore(storeName).getAll()),complete(tx)])
  return rows.filter(r=>r.conflictOf===key&&typeof r.text==='string').map(r=>({id:r.id,text:r.text,createdAt:r.createdAt??0})).sort((a,b)=>b.createdAt-a.createdAt)
 }finally{db.close()}
}
/** Delete one conflict copy; published snapshots and other rows are never touched. */
export async function removeSolidDraftConflict(key:string,id:string):Promise<boolean>{
 const db=await database()
 try{
  const tx=db.transaction(storeName,'readwrite',{durability:'strict'}),store=tx.objectStore(storeName),done=complete(tx)
  const row=await request<Snapshot|undefined>(store.get(id))
  const owned=row?.conflictOf===key
  if(owned)await request(store.delete(id))
  await done;return owned
 }finally{db.close()}
}

/**
 * Serialize reference publication across tabs, including the comparison immediately before writing.
 * Without Web Locks an expiring IndexedDB lease (solidDraftLease.ts) provides the same exclusion;
 * without IndexedDB either, saving is refused and JSON download remains available.
 */
export async function withSolidDraftLock<T>(key:string,action:()=>T|Promise<T>):Promise<T>{
 const locks=globalThis.navigator?.locks
 if(!locks)return withSolidDraftLease(`solid-draft:${key}`,action)
 return locks.request(`solid-draft:${key}`,{mode:'exclusive'},action)
}


const draftKeys=['scad-main-modeler-v1','scad-solid-modeler-v1'] as const
/** Keep published heads, unknown-age legacy rows and a 24-hour grace period for abandoned snapshots. */
export async function collectSolidDraftSnapshots(now=Date.now()):Promise<number>{
 if(!Number.isFinite(now))throw Error('Invalid draft cleanup time.')
 return withSolidDraftLock(draftKeys[0],()=>withSolidDraftLock(draftKeys[1],async()=>{
  // A throwing storage read must abort cleanup rather than appear to have no references.
  const retained=new Set<string>()
  for(const key of [...draftKeys,'scad-direct-modeler-v1']){
   const text=localStorage.getItem(key);if(!text)continue
   const value=JSON.parse(text),id=solidDraftSnapshotId(text)
   if(value&&typeof value==='object'&&'solidDraftId' in value&&!id)throw Error('Draft reference cannot be inspected safely.')
   if(id)retained.add(id)
  }
  const db=await database()
  try{
   const tx=db.transaction(storeName,'readwrite'),done=complete(tx)
   let removed=0
   const scanned=new Promise<void>((resolve,reject)=>{
    const cursor=tx.objectStore(storeName).openCursor()
    cursor.onerror=()=>reject(cursor.error??Error('Draft cleanup failed.'))
    cursor.onsuccess=()=>{
     const entry=cursor.result;if(!entry){resolve();return}
     const row=entry.value as Snapshot
     const age=typeof row.conflictOf==='string'?CONFLICT_RETENTION_MS:24*60*60*1000
     if(!retained.has(row.id)&&typeof row.createdAt==='number'&&Number.isFinite(row.createdAt)&&row.createdAt<=now-age){entry.delete();removed++}
     entry.continue()
    }
   })
   await Promise.all([scanned,done]);return removed
  }finally{db.close()}
 }))
}
