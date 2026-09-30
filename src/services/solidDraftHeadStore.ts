import {MAX_DOCUMENT_CHARACTERS} from './directDocumentLimits'
export interface SolidDraftHead {key:string;revision:string;text:string}
const databaseName='scad-solid-draft-heads-v1',store='heads'
async function database():Promise<IDBDatabase>{
 if(typeof indexedDB==='undefined')throw Error('Durable draft storage is unavailable. Download JSON.')
 return new Promise((resolve,reject)=>{
  const r=indexedDB.open(databaseName,1)
  r.onupgradeneeded=()=>r.result.createObjectStore(store,{keyPath:'key'})
  r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error??Error('Draft database failed.'))
 })
}
function valid(value:unknown):value is SolidDraftHead{
 const v=value as SolidDraftHead
 return !!v&&typeof v.key==='string'&&typeof v.revision==='string'&&v.revision.length>0&&typeof v.text==='string'&&v.text.length<=MAX_DOCUMENT_CHARACTERS
}
export async function readSolidDraftHead(key:string):Promise<SolidDraftHead|null>{
 const db=await database()
 try{return await new Promise((resolve,reject)=>{
  const tx=db.transaction(store,'readonly'),r=tx.objectStore(store).get(key)
  let row:SolidDraftHead|null=null
  r.onsuccess=()=>{if(r.result!==undefined&&!valid(r.result)){tx.abort();return}row=r.result??null}
  tx.oncomplete=()=>resolve(row);tx.onabort=tx.onerror=()=>reject(tx.error??Error('Saved draft head is invalid. Import a JSON backup.'))
 })}finally{db.close()}
}
/** Compare and replace the whole canonical document in one strict-durability transaction. */
export async function writeSolidDraftHead(key:string,expected:string|null,text:string,current=()=>true):Promise<SolidDraftHead>{
 if(text.length>MAX_DOCUMENT_CHARACTERS)throw Error('Document exceeds 64 MB.')
 const db=await database(),head={key,revision:crypto.randomUUID(),text}
 try{return await new Promise((resolve,reject)=>{
  const tx=db.transaction(store,'readwrite',{durability:'strict'}),r=tx.objectStore(store).get(key)
  let failure:Error|undefined
  r.onsuccess=()=>{
   if(r.result!==undefined&&(!valid(r.result)||r.result.key!==key)){failure=Error('DRAFT_CORRUPT');tx.abort();return}
   if((r.result?.revision??null)!==expected){failure=Error('DRAFT_CONFLICT');tx.abort();return}
   try{
    if(!current()){failure=Error('DRAFT_SUPERSEDED');tx.abort();return}
    const write=tx.objectStore(store).put(head)
    write.onerror=()=>{failure=write.error??Error('Durable draft write failed.')}
   }catch(e){failure=e instanceof Error?e:Error(String(e));tx.abort()}
  }
  tx.oncomplete=()=>resolve(head)
  tx.onabort=tx.onerror=()=>reject(failure??tx.error??Error('Durable draft transaction failed.'))
 })}finally{db.close()}
}
