import {readFile} from 'node:fs/promises'

// Preserve Vite's ?raw text import semantics in the standalone Node oracle.
// This supplies actual shader source; all geometry still uses the real WASM.
export async function load(url,context,nextLoad) {
 const source=new URL(url)
 if(source.protocol==='file:'&&source.searchParams.has('raw')) {
  source.search=''
  return {format:'module',shortCircuit:true,source:'export default '+JSON.stringify(await readFile(source,'utf8'))}
 }
 return nextLoad(url,context)
}
