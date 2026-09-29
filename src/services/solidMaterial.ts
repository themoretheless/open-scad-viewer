import type {DirectDocument,DirectMaterial} from './directModeling'
export const solidMaterialRgb=(color:string):[number,number,number]=>[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)/255) as [number,number,number]
export function assignSolidMaterial(document:DirectDocument,ids:readonly string[],material?:DirectMaterial):DirectDocument {
 if(material&&(!material.name.trim()||material.name.length>100||!/^#[0-9a-f]{6}$/i.test(material.color)))throw Error('Invalid body material.')
 if(material&&[material.metallic,material.roughness].some(v=>v!==undefined&&(!Number.isFinite(v)||v<0||v>1)))throw Error('Invalid body material.')
 const next=structuredClone(document)
 for(const body of next.bodies)if(ids.includes(body.id)){
  if(material)body.material=structuredClone(material)
  else delete body.material
 }
 return next
}
