import type {DirectStepLod0Import,DirectStepLod0Shell} from './cadNurbsStep'
import {storageGet,storageKeys,storageRemove,storageSet} from './safeStorage'

const INDEX_KEY='open-scad-viewer.step-lod0-index-v1'
const ENTRY_PREFIX='open-scad-viewer.step-lod0-v1.'
export const MAX_LOD0_MEMORY_BYTES=32*1024*1024
export const MAX_LOD0_DURABLE_BYTES=4*1024*1024
export const MAX_LOD0_SOURCE_INDEX_BYTES=512*1024
export const MAX_LOD0_ENTRIES=64
const MAX_LOD0_MESH_COMPONENTS=300_000

interface Lod0Mesh {positions:Float64Array;indices:Uint32Array}
export interface CachedStepLod0Shell extends Omit<DirectStepLod0Shell,'mesh'> {mesh:Lod0Mesh}
export interface CachedStepLod0Import extends Omit<DirectStepLod0Import,'lod0Shells'> {lod0Shells:CachedStepLod0Shell[]}
interface DocumentIndex {
  source:string
  document:Omit<DirectStepLod0Import,'lod0Shells'>
  shellKeys:string[]
}
interface DurableIndex {version:1;documents:DocumentIndex[]}

const memory=new Map<string,CachedStepLod0Shell>()
let memoryBytes=0
let documents:DocumentIndex[]=[]
let clock=0
const accessed=new Map<string,number>()

const meshBytes=(mesh:Lod0Mesh)=>mesh.positions.byteLength+mesh.indices.byteLength
const encodedBytes=(text:string)=>new TextEncoder().encode(text).byteLength
const finite=(value:unknown)=>typeof value==='number'&&Number.isFinite(value)
const validMesh=(value:unknown):value is {positions:number[];indices:number[]}=>{
  if(!value||typeof value!=='object')return false
  const mesh=value as {positions?:unknown;indices?:unknown}
  if(!Array.isArray(mesh.positions)||!Array.isArray(mesh.indices)
    ||mesh.positions.length<9||mesh.positions.length>MAX_LOD0_MESH_COMPONENTS||mesh.positions.length%3
    ||mesh.indices.length<3||mesh.indices.length>MAX_LOD0_MESH_COMPONENTS||mesh.indices.length%3)return false
  const positions=mesh.positions,indices=mesh.indices
  return positions.every(finite)&&indices.every(index=>Number.isInteger(index)&&index>=0&&index<positions.length/3)
}
const validShell=(value:unknown):value is DirectStepLod0Shell=>{
  if(!value||typeof value!=='object')return false
  const shell=value as DirectStepLod0Shell
  return typeof shell.cacheKey==='string'&&/^[0-9a-f]{64}$/.test(shell.cacheKey)
    &&Number.isInteger(shell.shellEntityId)&&typeof shell.name==='string'
    &&validMesh(shell.mesh)&&Array.isArray(shell.exactBrepEntityIds)
}
const validDocumentIndex=(value:unknown):value is DocumentIndex=>{
  if(!value||typeof value!=='object')return false
  const row=value as DocumentIndex
  return typeof row.source==='string'&&row.source.length<=MAX_LOD0_SOURCE_INDEX_BYTES
    &&!!row.document&&typeof row.document.cacheKey==='string'&&/^[0-9a-f]{64}$/.test(row.document.cacheKey)
    &&Array.isArray(row.shellKeys)&&row.shellKeys.length>0&&row.shellKeys.length<=MAX_LOD0_ENTRIES
    &&row.shellKeys.every(key=>typeof key==='string'&&/^[0-9a-f]{64}$/.test(key))
    &&Array.isArray(row.document.diagnostics)&&Array.isArray(row.document.schemaDiagnostics)
    &&Array.isArray(row.document.exactBrepEntityIds)
}
const decodeIndex=()=>{
  const raw=storageGet(INDEX_KEY)
  if(!raw||encodedBytes(raw)>MAX_LOD0_SOURCE_INDEX_BYTES*2)return []
  try{
    const value=JSON.parse(raw) as DurableIndex
    return value.version===1&&Array.isArray(value.documents)
      ?value.documents.filter(validDocumentIndex).slice(0,8):[]
  }catch{return []}
}
documents=decodeIndex()

const touch=(key:string)=>{
  const value=memory.get(key)
  if(!value)return
  memory.delete(key);memory.set(key,value);accessed.set(key,++clock)
}
const evictMemory=()=>{
  while(memory.size>MAX_LOD0_ENTRIES||memoryBytes>MAX_LOD0_MEMORY_BYTES){
    const oldest=memory.keys().next().value as string|undefined
    if(!oldest)break
    const entry=memory.get(oldest)!
    memory.delete(oldest);accessed.delete(oldest);memoryBytes-=meshBytes(entry.mesh)
  }
}
const putMemory=(shell:CachedStepLod0Shell)=>{
  const previous=memory.get(shell.cacheKey)
  if(previous)memoryBytes-=meshBytes(previous.mesh)
  memory.delete(shell.cacheKey)
  memory.set(shell.cacheKey,shell);accessed.set(shell.cacheKey,++clock)
  memoryBytes+=meshBytes(shell.mesh);evictMemory()
}
const getMemory=(key:string)=>{
  const shell=memory.get(key)
  if(shell)touch(key)
  return shell
}
const decodeShell=(key:string)=>{
  const raw=storageGet(ENTRY_PREFIX+key)
  if(!raw||encodedBytes(raw)>MAX_LOD0_DURABLE_BYTES)return
  try{
    const shell=JSON.parse(raw) as unknown
    if(!validShell(shell)||shell.cacheKey!==key)return
    return {...shell,mesh:{positions:Float64Array.from(shell.mesh.positions),indices:Uint32Array.from(shell.mesh.indices)}} as CachedStepLod0Shell
  }catch{return}
}
const durableBytes=()=>storageKeys(ENTRY_PREFIX).reduce((sum,key)=>sum+encodedBytes(storageGet(key)??''),0)
const persistShell=(shell:CachedStepLod0Shell)=>{
  const encoded=JSON.stringify({...shell,mesh:{positions:Array.from(shell.mesh.positions),indices:Array.from(shell.mesh.indices)}})
  if(encodedBytes(encoded)>MAX_LOD0_DURABLE_BYTES)return
  const key=ENTRY_PREFIX+shell.cacheKey
  const previous=encodedBytes(storageGet(key)??'')
  if(durableBytes()-previous+encodedBytes(encoded)<=MAX_LOD0_DURABLE_BYTES)storageSet(key,encoded)
}
const persistIndex=()=>{
  // Durable source lookup is deliberately bounded; shell entries remain independently addressable.
  let retained=documents.slice(0,8)
  while(retained.length&&encodedBytes(JSON.stringify({version:1,documents:retained}))>MAX_LOD0_SOURCE_INDEX_BYTES)retained=retained.slice(1)
  storageSet(INDEX_KEY,JSON.stringify({version:1,documents:retained} satisfies DurableIndex))
}

export const stepLod0Cache={
  get(cacheKey:string):CachedStepLod0Shell|undefined{
    const cached=getMemory(cacheKey)??decodeShell(cacheKey)
    if(cached&&!memory.has(cacheKey))putMemory(cached)
    return cached
  },
  getDocument(text:string):CachedStepLod0Import|undefined{
    const index=documents.find(row=>row.source===text)
    if(!index)return
    const shells=index.shellKeys.map(key=>this.get(key))
    if(shells.some(shell=>!shell))return
    documents=[index,...documents.filter(row=>row!==index)]
    return {...index.document,lod0Shells:shells as CachedStepLod0Shell[]}
  },
  putDocument(text:string,imported:DirectStepLod0Import):CachedStepLod0Import{
    const shells=imported.lod0Shells.map(shell=>{
      if(!validShell(shell))throw Error('Invalid AP242 LOD-0 shell payload.')
      const cached:CachedStepLod0Shell={...shell,mesh:{positions:Float64Array.from(shell.mesh.positions),indices:Uint32Array.from(shell.mesh.indices)}}
      putMemory(cached);persistShell(cached);return cached
    })
    const {lod0Shells:_shells,...document}=imported
    documents=[{source:text,document,shellKeys:shells.map(shell=>shell.cacheKey)},
      ...documents.filter(row=>row.document.cacheKey!==imported.cacheKey&&row.source!==text)].slice(0,8)
    persistIndex()
    return {...document,lod0Shells:shells}
  },
  reload(){
    memory.clear();memoryBytes=0;accessed.clear();documents=decodeIndex()
  },
  clear(){
    memory.clear();memoryBytes=0;accessed.clear();documents=[]
    for(const key of storageKeys(ENTRY_PREFIX))storageRemove(key)
    storageRemove(INDEX_KEY)
  },
  stats(){return {entries:memory.size,bytes:memoryBytes,documents:documents.length}},
}
