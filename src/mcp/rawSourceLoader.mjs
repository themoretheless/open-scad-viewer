import {readFileSync} from 'node:fs'
/** Match Vite's explicit raw imports in disposable Node geometry processes. */
export function load(url,context,nextLoad) {
 const source=new URL(url)
 if(source.protocol==='file:'&&source.searchParams.has('raw')) {
  source.search=''
  return {format:'module',shortCircuit:true,source:'export default '+JSON.stringify(readFileSync(source,'utf8'))}
 }
 return nextLoad(url,context)
}
