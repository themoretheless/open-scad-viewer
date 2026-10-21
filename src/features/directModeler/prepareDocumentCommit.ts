import type {DirectDocument} from '../../services/directModeling'
import {stringifyMeshJson} from '../../services/meshJson'
const documentObjects=(d:DirectDocument)=>[...d.bodies,...d.sketches,...d.curves??[],...d.surfaces??[]]
export function prepareDocumentCommit(next:DirectDocument,previous:DirectDocument|undefined,fallbackIds:Iterable<string>,lockedIds:string[],activeGroup:string,origin:'edit'|'file',label:(ru:string,en:string)=>string) {
  const existing=new Set(previous?documentObjects(previous).map(b=>b.id):fallbackIds)
  const sourceChanges=new Map<string,boolean>()
  const previousBodies=new Map(previous?.bodies.map(body=>[body.id,body])??[])
  const nextBodies=new Map(next.bodies.map(body=>[body.id,body]))
  if(origin==='edit')for(const source of previous?.bodies??[])if(!nextBodies.has(source.id)) {
    const linked=next.bodies.filter(body=>body.instance?.sourceId===source.id)
    if(linked.length)throw Error(label('Нельзя удалить источник «','Cannot delete source “')+source.name+label('»: остаются связанные экземпляры — ','”: linked instances remain — ')+linked.length+'. '+label('Сделайте их независимыми или удалите вместе с источником.','Make them independent or delete them together with the source.'))
  }
  // Imported caches are checked and rebuilt by the authoritative history parser.
  // The edit-only guard detects attempts to modify a live linked body directly.
  for(const body of previous?.bodies??[])if(origin==='edit'&&body.instance) {
    const sourceId=body.instance.sourceId
    if(!sourceChanges.has(sourceId)) {
      const oldSource=previousBodies.get(sourceId),newSource=nextBodies.get(sourceId)
      sourceChanges.set(sourceId,stringifyMeshJson({mesh:oldSource?.mesh,brep:oldSource?.brep})!==stringifyMeshJson({mesh:newSource?.mesh,brep:newSource?.brep}))
    }
    const candidate=nextBodies.get(body.id)
    if(!sourceChanges.get(sourceId)&&candidate?.instance&&stringifyMeshJson(candidate.instance)===stringifyMeshJson(body.instance)
      &&stringifyMeshJson({mesh:candidate.mesh,brep:candidate.brep})!==stringifyMeshJson({mesh:body.mesh,brep:body.brep}))
      throw Error(label('Измените источник или отсоедините экземпляр: ','Edit the source or detach the instance: ')+body.name)
  }
  const lockedObjects=(previous?documentObjects(previous):[]).filter(object=>lockedIds.includes(object.id))
  const validateLocked=lockedObjects.length?(resolved:DirectDocument)=>{
    const nextObjects=new Map(documentObjects(resolved).map(o=>[o.id,o]))
    for(const object of lockedObjects){
      const replacement=nextObjects.get(object.id)
      if(!replacement||stringifyMeshJson(object)!==stringifyMeshJson(replacement))throw Error(label('Объект заблокирован: ','Object is locked: ')+object.name)
    }
  }:undefined
  for(const body of documentObjects(next))if(!existing.has(body.id)&&(origin==='file'?body.group===undefined:!body.group)&&activeGroup)body.group=activeGroup
  const added=documentObjects(next).filter(b=>!existing.has(b.id)).map(b=>b.id)
  return {validateLocked,added}

}
