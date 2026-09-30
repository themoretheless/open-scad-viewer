import {expect,it} from 'vitest'
import {validCurveOffsetConstruction} from '../src/services/curveOffsetConstruction'
import {DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
const evidence={version:1 as const,scope:'at-construction' as const,sourceId:'removed-source',distanceMm:2,toleranceMm:.01,errorUpperMm:.001,method:'outward-source-offset-bevel-wire/1' as const,segmentCount:4,crossings:1,contacts:0,uncertain:0,complete:true,regionTopologyCertified:false as const}
it('preserves historical construction evidence through serialization, edits, undo and redo',async()=>{
 await warmGeometryKernel()
 const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'offset',name:'Offset',curve:{degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,2],[10,2]]},offsetConstruction:evidence}]}
 const restored=parseDirectDocument(serializeDirectDocument(document))
 expect(restored.curves![0]!.offsetConstruction).toEqual(evidence)
 const malformed=structuredClone(document);malformed.curves[0]!.offsetConstruction.errorUpperMm=.02
 expect(()=>parseDirectDocument(serializeDirectDocument(malformed))).toThrow('Invalid offset construction evidence')
 const history=new DirectHistory(restored),edited=structuredClone(restored)
 edited.curves![0]!.curve.controlPoints[1]![0]=12
 history.commit(edited);history.undo();expect(history.document.curves![0]!.offsetConstruction).toEqual(evidence)
 history.redo();expect(history.document.curves![0]!.offsetConstruction?.scope).toBe('at-construction')
 expect(history.document.curves![0]!.curve.controlPoints[1]![0]).toBe(12)
})
it('rejects malformed or falsely certified evidence',()=>{
 expect(validCurveOffsetConstruction(evidence)).toBe(true)
 for(const change of [{scope:'current-geometry'},{regionTopologyCertified:true},{errorUpperMm:.02},{crossings:7},{complete:true,uncertain:1},{sourceId:''},{distanceMm:Infinity}])expect(validCurveOffsetConstruction({...evidence,...change})).toBe(false)
})
