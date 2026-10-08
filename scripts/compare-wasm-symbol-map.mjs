import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'

const [namedPath,publishedPath,...indices]=process.argv.slice(2)
assert.ok(namedPath&&publishedPath,'Supply named diagnostic WASM and published WASM')
const sha=b=>createHash('sha256').update(b).digest('hex')
function parse(bytes){
 assert.equal(bytes.subarray(0,8).toString('hex'),'0061736d01000000')
 let at=8
 const standard=[bytes.subarray(0,8)],names=new Map()
 const uint=()=>{let n=0,shift=0,part;do{assert.ok(at<bytes.length&&shift<=28);part=bytes[at++];n+=(part&127)*2**shift;shift+=7}while(part&128);return n}
 const string=()=>{const size=uint();assert.ok(at+size<=bytes.length);const out=bytes.subarray(at,at+size).toString('utf8');at+=size;return out}
 while(at<bytes.length){
  const start=at,id=bytes[at++],size=uint(),end=at+size
  assert.ok(end<=bytes.length)
  if(id!==0)standard.push(bytes.subarray(start,end))
  else if(string()==='name'){
   while(at<end){
    const sub=bytes[at++],subSize=uint(),subEnd=at+subSize
    assert.ok(subEnd<=end)
    if(sub===1){const count=uint();for(let i=0;i<count;i++)names.set(uint(),string())}
    at=subEnd
   }
  }
  at=end
 }
 return {standard:Buffer.concat(standard),names}
}
const named=parse(readFileSync(namedPath)),published=parse(readFileSync(publishedPath))
const identical=named.standard.equals(published.standard)
console.log(JSON.stringify({identicalStandardSections:identical,
 namedStandardSha256:sha(named.standard),publishedStandardSha256:sha(published.standard),
 functionNameCount:named.names.size,
 mapped:identical?indices.map(index=>({index:Number(index),name:named.names.get(Number(index))??null})):[],
 scope:'Index mapping is valid only when all standard binary sections match byte-for-byte'},null,2))
assert.ok(identical,'Diagnostic function indices cannot be attributed to the published profile')
