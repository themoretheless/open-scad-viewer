import {readFile} from 'node:fs/promises'
import path from 'node:path'
import {init,parse} from 'es-module-lexer'

/** Follow actual emitted module imports, including statically named dynamic
 * imports. A digest in an unrelated asset cannot establish worker provenance.
 * Runtime compilation still verifies the actual WASM bytes independently. */
export async function traceWorkerWasmFingerprint(dist,entry,sha256){
 await init
 if(!/^[a-f0-9]{64}$/.test(sha256))throw new Error('Invalid expected WASM fingerprint')
 const root=path.resolve(dist),queue=[path.resolve(root,'assets',entry)],visited=new Set(),matched=[]
 let unresolvedDynamicImports=0
 const local=file=>{const relative=path.relative(root,file);if(relative==='..'||relative.startsWith('..'+path.sep)||path.isAbsolute(relative))throw new Error('Worker import escapes the distribution');return relative.replaceAll(path.sep,'/')}
 while(queue.length){
  const file=queue.shift();local(file)
  if(visited.has(file))continue
  if(visited.size>=512)throw new Error('Worker module provenance exceeds its traversal limit')
  visited.add(file)
  const code=await readFile(file,'utf8')
  if(code.includes(sha256))matched.push(local(file))
  for(const imported of parse(code)[0]){
   if(imported.d===-2)continue // import.meta is not a dependency.
   if(imported.n===undefined){if(imported.d>=0)unresolvedDynamicImports++;continue}
   const name=imported.n
   if(!name.startsWith('.')&&!name.startsWith('/'))throw new Error('Worker has an unverifiable external module import: '+name)
   const target=name.startsWith('/')?path.resolve(root,'.'+name):path.resolve(path.dirname(file),name)
   local(target)
   if(path.extname(target)==='.js')queue.push(target)
  }
 }
 if(matched.length===0)throw new Error('Worker module graph does not contain the current WASM fingerprint')
 return {entry,modules:[...visited].map(local),matchedModules:matched,unresolvedDynamicImports}
}
