import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string,alter=(s:string)=>s){
 const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
 const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
 return JSON.parse(scene.measurements![0]!.report)
}
function encloses(bounds:string[],expected:number){
 const [lo,hi]=bounds.map(s=>{const [n,d='1']=s.split('/');return Number(n)/Number(d)})
 expect(lo).toBeLessThanOrEqual(expected);expect(hi).toBeGreaterThanOrEqual(expected)
 expect(hi!-lo!).toBeLessThanOrEqual(0.00001)
}
it('locates a boundary crossing between path vertices with a retained nearest face',async()=>{
 for(const n of ['measure-swept-ball-gap','measure-swept-path-gap','classify-swept-path-material']){
   const r=await measure(n)
   expect(r.nearest).toMatchObject({comparison:'Less',distanceSquaredMm2:'0',centerMm:['1','7/2','5'],boundaryMm:['1','7/2','5']})
   if(n==='classify-swept-path-material') expect(r.relation).toBe('CrossesBoundary')
 }
})
it('encloses first contact and the interval of contact on the traversed face',async()=>{
 const first=await measure('bound-swept-first-contact')
 expect(first.contact.span).toBe(0);encloses(first.contact.parameterBounds,5/8)
 const faces=await measure('bound-swept-face-ranges')
 expect(faces.contacts).toHaveLength(1)
 encloses(faces.contacts[0].firstBounds,5/8);encloses(faces.contacts[0].lastBounds,7/8)
 const clear=await measure('bound-swept-first-contact',s=>s.replace('[2,"7/2",5]','[0,"7/2",5]'))
 expect(clear.contact).toBeNull()
})
