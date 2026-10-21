import {readFileSync} from 'node:fs'

// Preserve Vite's raw source imports in real Node Worker test realms.
// Return the repository bytes; do not replace shaders or geometry modules.
export function load(url,context,nextLoad) {
 const source=new URL(url)
 if(source.protocol==='file:'&&source.searchParams.has('raw')) {
  source.search=''
  return {format:'module',shortCircuit:true,source:'export default '+JSON.stringify(readFileSync(source,'utf8'))}
 }
 return nextLoad(url,context)
}
