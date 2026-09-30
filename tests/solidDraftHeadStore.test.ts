import {it,expect,vi,afterEach} from 'vitest'
import {IDBFactory} from 'fake-indexeddb'
import {readSolidDraftHead,writeSolidDraftHead} from '../src/services/solidDraftHeadStore'
afterEach(()=>vi.unstubAllGlobals())
it('atomically publishes one revision and rejects a competing writer without replacing geometry',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 expect(await readSolidDraftHead('model')).toBeNull()
 const initial=await writeSolidDraftHead('model',null,'initial')
 const results=await Promise.allSettled([writeSolidDraftHead('model',initial.revision,'first'),writeSolidDraftHead('model',initial.revision,'second')])
 expect(results.filter(r=>r.status==='fulfilled')).toHaveLength(1)
 expect(results.find(r=>r.status==='rejected')).toMatchObject({reason:{message:'DRAFT_CONFLICT'}})
 const saved=await readSolidDraftHead('model')
 expect(['first','second']).toContain(saved!.text)
 await expect(writeSolidDraftHead('model',saved!.revision,'obsolete',()=>false)).rejects.toThrow('DRAFT_SUPERSEDED')
 expect(await readSolidDraftHead('model')).toEqual(saved)
})
it('retains a committed head after transaction abort and requests strict durability',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const initial=await writeSolidDraftHead('model',null,'retained')
 const request=indexedDB.open('scad-solid-draft-heads-v1')
 const db=await new Promise<IDBDatabase>((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)})
 const prototype=Object.getPrototypeOf(db),original=prototype.transaction,durabilities:unknown[]=[]
 const spy=vi.spyOn(prototype,'transaction').mockImplementation(function(this:IDBDatabase,...args:any[]){
  const tx=original.apply(this,args);durabilities.push(args[2]?.durability)
  if(args[1]==='readwrite'){const store=tx.objectStore('heads'),put=store.put.bind(store);store.put=(...values:Parameters<IDBObjectStore['put']>)=>{const r=put(...values);tx.abort();return r}}
  return tx
 })
 try{await expect(writeSolidDraftHead('model',initial.revision,'lost')).rejects.toThrow();expect(durabilities).toContain('strict');expect(await readSolidDraftHead('model')).toEqual(initial)}finally{spy.mockRestore();db.close()}
})
it('refuses to overwrite a corrupt head even when its revision still matches',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const initial=await writeSolidDraftHead('model',null,'retained')
 const request=indexedDB.open('scad-solid-draft-heads-v1')
 const db=await new Promise<IDBDatabase>((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)})
 const put=(row:unknown)=>new Promise<void>((resolve,reject)=>{const tx=db.transaction('heads','readwrite');tx.objectStore('heads').put(row);tx.oncomplete=()=>resolve();tx.onabort=()=>reject(tx.error)})
 const raw=()=>new Promise<unknown>((resolve,reject)=>{const r=db.transaction('heads','readonly').objectStore('heads').get('model');r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error)})
 try{
  for(const row of [{...initial,text:null},{...initial,text:42},{...initial,revision:''}]){
   await put(row)
   await expect(readSolidDraftHead('model')).rejects.toThrow()
   await expect(writeSolidDraftHead('model',initial.revision,'new geometry')).rejects.toThrow('DRAFT_CORRUPT')
   expect(await raw()).toEqual(row)
  }
  await put(initial)
  const next=await writeSolidDraftHead('model',initial.revision,'recovered')
  expect(await readSolidDraftHead('model')).toEqual(next)
 }finally{db.close()}
})
it('preserves quota errors and the old head when put throws, then retries successfully',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const initial=await writeSolidDraftHead('model',null,'retained')
 const request=indexedDB.open('scad-solid-draft-heads-v1')
 const db=await new Promise<IDBDatabase>((resolve,reject)=>{request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)})
 const store=db.transaction('heads','readonly').objectStore('heads'),prototype=Object.getPrototypeOf(store)
 const spy=vi.spyOn(prototype,'put').mockImplementation(()=>{throw new DOMException('Quota exhausted','QuotaExceededError')})
 try{
  await expect(writeSolidDraftHead('model',initial.revision,'new geometry')).rejects.toMatchObject({name:'QuotaExceededError'})
  expect(await readSolidDraftHead('model')).toEqual(initial)
 }finally{spy.mockRestore();db.close()}
 const next=await writeSolidDraftHead('model',initial.revision,'new geometry')
 expect(await readSolidDraftHead('model')).toEqual(next)
})
