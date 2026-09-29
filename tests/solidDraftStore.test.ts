import {it,expect,vi,afterEach} from 'vitest'
import {IDBFactory} from 'fake-indexeddb'
import {readSolidDraftSnapshot,writeSolidDraftSnapshot,removeSolidDraftSnapshot,solidDraftSnapshotId} from '../src/services/solidDraftStore'
afterEach(()=>vi.unstubAllGlobals())
it('stores large immutable snapshots and removes only the superseded snapshot',async()=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const first='a'.repeat(4_100_000),second='b'.repeat(4_200_000)
 const a=await writeSolidDraftSnapshot(first),b=await writeSolidDraftSnapshot(second)
 expect(a).not.toBe(b);expect(solidDraftSnapshotId(a)).not.toBeNull()
 expect(await readSolidDraftSnapshot(a)).toBe(first);expect(await readSolidDraftSnapshot(b)).toBe(second)
 await removeSolidDraftSnapshot(a)
 await expect(readSolidDraftSnapshot(a)).rejects.toThrow('missing')
 expect(await readSolidDraftSnapshot(b)).toBe(second)
})
it('rejects invalid references and reports unavailable durable storage',async()=>{
 expect(solidDraftSnapshotId('{"version":1,"bodies":[]}')).toBeNull()
 expect(solidDraftSnapshotId('{"version":1,"solidDraftId":"../bad"}')).toBeNull()
 vi.stubGlobal('indexedDB',undefined)
 await expect(writeSolidDraftSnapshot('{}')).rejects.toThrow('unavailable')
})

it('collects only old unpublished snapshots while retaining all document heads',async()=>{
 const {collectSolidDraftSnapshots}=await import('../src/services/solidDraftStore')
 vi.stubGlobal('indexedDB',new IDBFactory())
 const locks:string[]=[]
 vi.stubGlobal('navigator',{locks:{request:(name:string,_options:unknown,action:()=>unknown)=>{locks.push(name);return Promise.resolve(action())}}})
 const now=Date.now(),old=vi.spyOn(Date,'now').mockReturnValue(now-2*24*60*60*1000)
 let a:string,b:string,legacy:string,orphan:string
 try{a=await writeSolidDraftSnapshot('main');b=await writeSolidDraftSnapshot('solid');legacy=await writeSolidDraftSnapshot('legacy-head');orphan=await writeSolidDraftSnapshot('abandoned')}finally{old.mockRestore()}
 const recent=await writeSolidDraftSnapshot('recent')
 const refs:Record<string,string>={'scad-main-modeler-v1':a!,'scad-solid-modeler-v1':b!,'scad-direct-modeler-v1':legacy!}
 vi.stubGlobal('localStorage',{getItem:(key:string)=>refs[key]??null})
 expect(await collectSolidDraftSnapshots(now)).toBe(1)
 expect(locks).toEqual(['solid-draft:scad-main-modeler-v1','solid-draft:scad-solid-modeler-v1'])
 await expect(readSolidDraftSnapshot(orphan!)).rejects.toThrow('missing')
 expect(await readSolidDraftSnapshot(a!)).toBe('main');expect(await readSolidDraftSnapshot(b!)).toBe('solid')
 expect(await readSolidDraftSnapshot(legacy!)).toBe('legacy-head');expect(await readSolidDraftSnapshot(recent)).toBe('recent')
 vi.stubGlobal('localStorage',{getItem:()=>{throw Error('storage blocked')}})
 await expect(collectSolidDraftSnapshots(now+3*24*60*60*1000)).rejects.toThrow('storage blocked')
 expect(await readSolidDraftSnapshot(recent)).toBe('recent')
 vi.stubGlobal('localStorage',{getItem:()=>'{broken-reference'})
 await expect(collectSolidDraftSnapshots(now+3*24*60*60*1000)).rejects.toThrow()
 expect(await readSolidDraftSnapshot(recent)).toBe('recent')
})
