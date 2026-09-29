import {parseDirectDocument,type DirectDocument} from './directModeling'
import {stringifyMeshJson} from './meshJson'
import {importModelGraphNurbs} from './solidNurbsImport'
import {warmLanguageKernel} from './languages/kernel'
/** Worker entry point: build native definitions and validate the complete replacement. */
export async function importSolidModelGraph(document:DirectDocument,text:string,group?:string):Promise<DirectDocument>{
 await warmLanguageKernel()
 const imported=importModelGraphNurbs(JSON.parse(text))
 for(const item of [...imported.curves,...imported.surfaces])if(group)item.group=group
 return parseDirectDocument(stringifyMeshJson({...document,
  curves:[...(document.curves??[]),...imported.curves],
  surfaces:[...(document.surfaces??[]),...imported.surfaces]}))
}
