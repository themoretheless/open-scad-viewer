import {readFileSync,writeFileSync} from 'node:fs'
import ts from 'typescript'
import {parseOpenSCAD} from '../../src/services/openscadParser'
import {referenceCaptureLegacyOutcome,referenceLegacyMeshBytes,referenceLegacySceneBytes,referenceLegacySha256} from '../../tests/support/referenceLegacyDirectEvaluatorOracle'
const test=readFileSync('tests/legacyDirectEvaluatorOracle.test.ts','utf8')
const literal=test.slice(test.indexOf('Object.freeze([')+'Object.freeze('.length,test.indexOf('\n])')+2)
writeFileSync('benchmarks/own-cad/own-oracle-cases.ts','export default '+literal)
const {default:fixture}=await import('./own-oracle-cases')
const snapshots={}
for(const f of fixture){const r=await referenceCaptureLegacyOutcome(()=>parseOpenSCAD(f.source,{quality:f.quality}));if(r.tag!=='success')throw new Error('Own CAD snapshot failed '+f.id);const mesh=referenceLegacyMeshBytes(r),scene=referenceLegacySceneBytes(r);snapshots[f.id]={meshLength:mesh.length,meshHash:referenceLegacySha256(mesh),sceneLength:scene.length,sceneHash:referenceLegacySha256(scene),volume:r.success.volume,surfaceArea:r.success.surfaceArea}}
writeFileSync('tests/fixtures/own-rust-cad-oracle-v1.json',JSON.stringify(snapshots,null,2)+'\n')
