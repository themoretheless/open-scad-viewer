// Read-only reproduction of scenario defects in PR7; never a product solver.
import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import ts from 'typescript'

const commit='7b3afccf379aead6151c7ceaf9b633e40eb62924'
const paths=['src/services/latticeStrengthScenario.ts','src/services/latticeTrussFea.ts']
const sources=paths.map(path=>execFileSync('git',['show',`${commit}:${path}`],{encoding:'utf8'}))
function evaluate(source,require) {
  const exports={}
  const output=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText
  new Function('exports','require',output)(exports,require)
  return exports
}
const scenario=evaluate(sources[0],()=>{throw Error('Unexpected dependency')})
// Expose private helpers only for observation; their bodies remain unchanged.
const legacy=evaluate(sources[1]+'\nexport {assembleLoads, restrain};\n',name=>{
  assert.equal(name,'./latticeStrengthScenario');return scenario
})
const nodes=[[0,0,0],[10,0,0],[10,10,0],[0,10,0],[0,0,10],[10,0,10],[10,10,10],[0,10,10]]
function resultants(forces) {
  const force=[0,0,0],moment=[0,0,0]
  for(let i=0;i<nodes.length;i++) {
    const p=nodes[i],f=Array.from(forces.slice(i*3,i*3+3))
    for(let k=0;k<3;k++)force[k]+=f[k]
    moment[0]+=p[1]*f[2]-p[2]*f[1]
    moment[1]+=p[2]*f[0]-p[0]*f[2]
    moment[2]+=p[0]*f[1]-p[1]*f[0]
  }
  return {forceN:force,momentNmm:moment}
}
const moments=[]
for(const direction of [[1,0,0],[0,1,0],[0,0,1]]) {
  const action={id:'moment',kind:'moment',direction,magnitude:100,faceNormal:[0,0,1],ru:'',en:''}
  const observed=resultants(legacy.assembleLoads(nodes,[action],[10,10,10]))
  moments.push({requestedMomentNmm:direction.map(v=>v*100),observed})
}
assert.deepEqual(moments.map(x=>x.observed.momentNmm),[[0,0,0],[0,0,100],[0,0,0]])
assert.ok(moments.every(x=>x.observed.forceN.every(v=>v===0)))

const pointMoment=resultants(legacy.assembleLoads(nodes,[{id:'point-moment',kind:'moment',
  direction:[0,0,1],magnitude:100,at:[10,0,10],ru:'',en:''}],[10,10,10]))
assert.deepEqual(pointMoment,{forceN:[0,0,40],momentNmm:[0,-400,0]})

const withInterior=[...nodes,[5,5,2]]
const fixed=legacy.restrain(withInterior,[{id:'pin',kind:'pinned',normal:[0,0,-1],ru:'',en:''}])
const fullyFixed=withInterior.map((_,i)=>i).filter(i=>fixed.slice(i*3,i*3+3).every(Boolean))
assert.deepEqual(fullyFixed,[8])
const implicit=legacy.restrain(nodes,[])
assert.equal(implicit.filter(Boolean).length,12)

console.log(JSON.stringify({commit,sourceSha256:Object.fromEntries(paths.map((path,i)=>[
  path,createHash('sha256').update(sources[i]).digest('hex'),
])),scope:'read-only legacy scenario audit; no current solver or material-safety claims',
  moments,pointMoment,pinned:{selectedFace:'-Z',fullyFixedNodes:fullyFixed,coordinates:fullyFixed.map(i=>withInterior[i])},
  emptySupports:{implicitlyRestrainedDofs:implicit.filter(Boolean).length},
},null,2))
