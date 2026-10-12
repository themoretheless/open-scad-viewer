import {minify} from 'terser'
export function compactRuntimeChunks(){
 return {name:'compact-runtime-chunks',apply:'build',enforce:'post',renderChunk:{order:'post',async handler(code,chunk){
  if(code.length<1000||chunk.fileName.includes('bytes'))return null
  const result=await minify(code,{ecma:2020,module:true,compress:{passes:3},mangle:true,format:{comments:false}})
  const before=Buffer.byteLength(code),after=Buffer.byteLength(result.code)
  return after<before?{code:result.code,map:null}:null
 }}}
}
