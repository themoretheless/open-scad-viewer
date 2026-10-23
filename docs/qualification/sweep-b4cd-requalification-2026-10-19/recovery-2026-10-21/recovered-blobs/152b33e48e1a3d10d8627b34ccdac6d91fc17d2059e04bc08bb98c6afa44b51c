import {importDirectStepV9,type DirectStepV9Import} from './cadNurbsStep'
import {composeStepV9Occurrences,type NurbsBrep} from './geometry/brep'

export interface StepExternalResolvedDocument {text:string;sha256:string}
export interface StepExternalResolver {
  resolve(uri:string,options:{signal:AbortSignal}):Promise<StepExternalResolvedDocument>
}
export interface StepExternalPolicy {
  allowedUris:ReadonlySet<string>
  expectedSha256:Readonly<Record<string,string>>
  maxDocuments:number
  maxDepth:number
  maxDocumentBytes:number
  maxTotalBytes:number
}
export interface ResolvedStepDocument {uri:string;digest:string;imported:DirectStepV9Import}
export interface AuthoritativeStepProductGraph {
  model:NurbsBrep
  documents:{uri:string;digest:string;definitionIds:string[]}[]
  occurrences:{uri:string;topoId:string;definitionId:string;digest:string}[]
}
export interface ResolvedStepBundle {
  root:DirectStepV9Import
  documents:ResolvedStepDocument[]
  authoritative:AuthoritativeStepProductGraph
  report:{resolved:string[];refused:string[];totalBytes:number}
}

const DEFAULT_POLICY:StepExternalPolicy={
  allowedUris:new Set(),expectedSha256:{},maxDocuments:16,maxDepth:4,
  maxDocumentBytes:16*1024*1024,maxTotalBytes:32*1024*1024,
}
const externalUris=(imported:DirectStepV9Import):string[]=>[...new Set(imported.externalReferences.map(reference=>{
  const separator=reference.indexOf(':')
  if(separator<0)throw Error('Malformed STEP external-reference report')
  return reference.slice(separator+1)
}))].sort()
const digest=async(text:string)=>{
  const bytes=new TextEncoder().encode(text)
  const hash=await crypto.subtle.digest('SHA-256',bytes)
  return [...new Uint8Array(hash)].map(value=>value.toString(16).padStart(2,'0')).join('')
}
const cancelled=(signal:AbortSignal)=>{if(signal.aborted)throw new DOMException('STEP external resolution cancelled','AbortError')}
const positiveInteger=(value:number,name:string)=>{
  if(!Number.isSafeInteger(value)||value<0)throw Error(`STEP external ${name} must be a non-negative safe integer`)
}

/**
 * Resolves AP242 DOCUMENT_FILE references only through the supplied resolver.
 * This module never reads ambient files and never performs network requests.
 */
export async function importStepV6WithResolver(
  text:string,resolver:StepExternalResolver,signal:AbortSignal,
  policy:Partial<StepExternalPolicy>={},
):Promise<ResolvedStepBundle>{
  const limits={...DEFAULT_POLICY,...policy}
  positiveInteger(limits.maxDocuments,'document limit')
  positiveInteger(limits.maxDepth,'depth limit')
  positiveInteger(limits.maxDocumentBytes,'document byte limit')
  positiveInteger(limits.maxTotalBytes,'total byte limit')
  const root=importDirectStepV9(text),documents:ResolvedStepDocument[]=[]
  const refused:string[]=[],active=new Set<string>(),seen=new Set<string>()
  let totalBytes=new TextEncoder().encode(text).byteLength
  if(totalBytes>limits.maxTotalBytes)throw Error('STEP external bundle exceeds total byte limit')
  const visit=async(uri:string,depth:number):Promise<void>=>{
    cancelled(signal)
    if(depth>limits.maxDepth)throw Error(`STEP external reference depth exceeds ${limits.maxDepth}`)
    if(active.has(uri))throw Error(`STEP external reference cycle at ${uri}`)
    if(seen.has(uri))return
    if(!limits.allowedUris.has(uri)){refused.push(`${uri}:not-allowlisted`);throw Error(`Untrusted STEP external reference: ${uri}`)}
    if(seen.size>=limits.maxDocuments)throw Error(`STEP external document count exceeds ${limits.maxDocuments}`)
    active.add(uri)
    try{
      const resolved=await resolver.resolve(uri,{signal});cancelled(signal)
      const bytes=new TextEncoder().encode(resolved.text).byteLength
      if(bytes>limits.maxDocumentBytes)throw Error(`STEP external document ${uri} exceeds byte limit`)
      totalBytes+=bytes
      if(totalBytes>limits.maxTotalBytes)throw Error('STEP external bundle exceeds total byte limit')
      const actual=await digest(resolved.text)
      if(actual!==resolved.sha256||actual!==limits.expectedSha256[uri]){
        refused.push(`${uri}:digest-mismatch`);throw Error(`STEP external digest mismatch: ${uri}`)
      }
      const imported=importDirectStepV9(resolved.text)
      seen.add(uri);documents.push({uri,digest:actual,imported})
      for(const child of externalUris(imported))await visit(child,depth+1)
    }finally{active.delete(uri)}
  }
  for(const uri of externalUris(root))await visit(uri,1)
  cancelled(signal)
  const sources=[{uri:'urn:step:root',digest:await digest(text),imported:root},...documents]
  cancelled(signal)
  const model=composeStepV9Occurrences(sources.map(source=>source.imported.model))
  const bodyIds=model.topologyIds?.bodies
  if(!bodyIds||bodyIds.length!==model.bodies.length)throw Error('STEP /9 composition did not assign occurrence TopoIds')
  const occurrences:{uri:string;topoId:string;definitionId:string;digest:string}[]=[]
  let bodyOffset=0
  for(const source of sources){
    const definitions=source.imported.definitionIdentities
    if(definitions.length===0)throw Error(`STEP /9 document ${source.uri} has no definition identity`)
    for(let index=0;index<source.imported.model.bodies.length;index++){
      occurrences.push({uri:source.uri,topoId:bodyIds[bodyOffset++],
        definitionId:definitions[Math.min(index,definitions.length-1)],digest:source.digest})
    }
  }
  return {
    root,documents,
    authoritative:{
      model,
      documents:sources.map(source=>({uri:source.uri,digest:source.digest,definitionIds:source.imported.definitionIdentities})),
      occurrences,
    },
    report:{resolved:documents.map(document=>document.uri),refused,totalBytes},
  }
}
