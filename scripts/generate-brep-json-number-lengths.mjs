// Independent browser-number spelling oracle for native UTF-16 JSON accounting.
// Run explicitly when reviewing the fixture; tests do not regenerate expectations.
import {writeFileSync} from 'node:fs'
let seed=0x12345678
const next=()=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return seed>>>0}
const view=new DataView(new ArrayBuffer(8)),rows=[]
while(rows.length<2048){
 view.setUint32(0,next());view.setUint32(4,next())
 const value=view.getFloat64(0)
 if(Number.isFinite(value))rows.push([value,JSON.stringify(value).length])
}
writeFileSync(new URL('../crates/geometry-bridge/tests/fixtures/brep-json-number-lengths.json',import.meta.url),JSON.stringify(rows))
