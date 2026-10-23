import {afterEach,expect,it} from 'vitest'
import {createHash} from 'node:crypto'
import {mkdtempSync,rmSync,writeFileSync,symlinkSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {verifyBrowserPayloadTree} from '../scripts/browserPayloadTree.mjs'
const roots:string[]=[]
const ignored=[{path:'DEPENDENCIES_VALIDATED',byteLength:0,sha256:createHash('sha256').update('').digest('hex')}]
const fixture=()=>{const root=mkdtempSync(join(tmpdir(),'browser-payload-'));roots.push(root);writeFileSync(join(root,'browser'),'executable payload');return root}
afterEach(()=>{for(const root of roots.splice(0))rmSync(root,{recursive:true,force:true})})
it('preserves payload identity with and without the empty installer marker',()=>{
 const root=fixture(),before=verifyBrowserPayloadTree(root,false,ignored)
 writeFileSync(join(root,'DEPENDENCIES_VALIDATED'),'')
 expect(verifyBrowserPayloadTree(root,false,ignored)).toEqual(before)
 writeFileSync(join(root,'browser'),'changed executable')
 expect(verifyBrowserPayloadTree(root,false,ignored).value).not.toBe(before.value)
})
it('rejects nonempty, linked, and broader marker exclusions',()=>{
 const root=fixture(),marker=join(root,'DEPENDENCIES_VALIDATED')
 writeFileSync(marker,'unexpected content')
 expect(()=>verifyBrowserPayloadTree(root,false,ignored)).toThrow(/empty regular/)
 rmSync(marker);symlinkSync('browser',marker)
 expect(()=>verifyBrowserPayloadTree(root,false,ignored)).toThrow(/empty regular/)
 expect(()=>verifyBrowserPayloadTree(root,false,[{...ignored[0]!,path:'browser'}])).toThrow(/Unsupported/)
})
it('rejects links outside the recorded browser payload',()=>{
 const root=fixture();symlinkSync('../unrecorded',join(root,'escape'))
 expect(()=>verifyBrowserPayloadTree(root,false,ignored)).toThrow(/escapes/)
})
