import type {DirectDocument} from './directModeling'
import {certifyMinkowski3d,certifyNurbsMinkowski,verifyMinkowskiProof,minkowskiSource,type MinkowskiProof,type MinkowskiAuthoredSource,type ApproximationTopology} from './geometry/minkowski3d'
export interface SolidMinkowskiOptions {ids:[string,string];createdId:string;ru:boolean;maximumSourceDeviationMm?:number}
export interface SolidMinkowskiResult {document:DirectDocument;resultId:string|null;proof:MinkowskiProof;volumeMm3:number;maximumCoordinateRoundingMm:number;sourceHadBrep:boolean;curvedSourceErrorCertified:boolean;maximumSetDeviationMm:number|null;roundingTopologyCertified:true;approximationTopology:ApproximationTopology|null;sourceApproximationTopology:[boolean,boolean]}
function hasDocumentId(source:DirectDocument,id:string):boolean {
 for(const b of source.bodies)if(b.id===id)return true
 for(const s of source.sketches)if(s.id===id)return true
 if(source.curves)for(const c of source.curves)if(c.id===id)return true
 if(source.surfaces)for(const s of source.surfaces)if(s.id===id)return true
 return false
}
/** A new body and one Undo transaction. Sources remain editable and untouched. */
export function applySolidMinkowski(source:DirectDocument,options:SolidMinkowskiOptions):SolidMinkowskiResult {
 const label=(r:string,e:string)=>options.ru?r:e
 if(options.ids.length!==2||options.ids[0]===options.ids[1])throw Error(label('Выберите ровно два разных тела.','Select exactly two different bodies.'))
 if(!options.createdId||hasDocumentId(source,options.createdId))throw Error('Minkowski result identity is empty or already used.')
 if(source.sketches.length+source.bodies.reduce((n,b)=>n+(b.instance?0:1),0)>=200)throw Error(label('В проекте уже 200 независимых объектов.','The project already contains 200 independent objects.'))
 const bodies=options.ids.map(id=>{const b=source.bodies.find(b=>b.id===id);if(!b)throw Error(label('Выбранное тело больше не существует.','A selected body no longer exists.'));return b})
 const authored=bodies.some(b=>!!b.brep)
 const asSource=(body:typeof bodies[number]):MinkowskiAuthoredSource=>body.brep?{kind:'brep',model:body.brep}:{kind:'mesh',mesh:minkowskiSource(body.mesh)}
 const inputs=authored?bodies.map(asSource):bodies.map(b=>minkowskiSource(b.mesh))
 const precision='f32' as const
 const result=authored?certifyNurbsMinkowski(inputs[0] as MinkowskiAuthoredSource,inputs[1] as MinkowskiAuthoredSource,options.maximumSourceDeviationMm??1,precision):certifyMinkowski3d(inputs[0] as ReturnType<typeof minkowskiSource>,inputs[1] as ReturnType<typeof minkowskiSource>,precision)
 const pos=result.mesh.positions
 for(let i=0;i<pos.length;i++){const x=pos[i];if(!Number.isFinite(x)||Math.abs(x)>1e6)throw Error(label('Результат выходит за пределы координат проекта.','The result exceeds the project coordinate range.'))}
 const proof=(authored?{version:2,a:inputs[0],b:inputs[1],certificate:result.certificate,meshPrecision:precision}:{version:1,a:inputs[0],b:inputs[1],certificate:result.certificate,meshPrecision:precision}) as MinkowskiProof
 // Recheck the exported witness before publishing a document transaction.
 verifyMinkowskiProof(proof)
 const d=structuredClone(source),resultId=result.mesh.indices.length?options.createdId:null
 if(resultId)d.bodies.push({id:resultId,name:label('Сумма Минковского','Minkowski sum'),mesh:result.mesh,...bodies[0].group?{group:bodies[0].group}:{}})
 return {document:d,resultId,proof,volumeMm3:result.volumeMm3,maximumCoordinateRoundingMm:result.maximumCoordinateRoundingMm,sourceHadBrep:authored,curvedSourceErrorCertified:result.curvedSourceErrorCertified,maximumSetDeviationMm:result.maximumSetDeviationMm,roundingTopologyCertified:true,approximationTopology:'approximationTopology' in result?result.approximationTopology:null,sourceApproximationTopology:'sourceApproximationTopology' in result&&result.sourceApproximationTopology?result.sourceApproximationTopology:authored?[0,1].map(i=>{const c=result.certificate as import('./geometry/minkowski3d').NurbsMinkowskiCertificate;return c.sourceHomotopyBounds?.[i]?.[0]==='0'||!!c.sourceTopology?.[i]}) as [boolean,boolean]:[true,true]}
}
