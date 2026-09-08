import {callGeometryRust} from './geometryRustKernel'
import type {GeometryDeformation} from './geometryEditing'
import type {PolygonBuild,PolygonMesh} from './polygonKernel'
export interface SdfProfile {outer:number[][];holes:number[][][]}
export type SdfField =
 | {kind:'extrude';profile:SdfProfile;half_height:number}
 | {kind:'revolve';profile:SdfProfile}
 | {kind:'deform';input:SdfField;deformation:GeometryDeformation}
 | {kind:'mesh_distance';mesh:PolygonMesh;signed:boolean}
 | {kind:'sphere';center:number[];radius:number}
 | {kind:'box';center:number[];half_size:number[]}
 | {kind:'torus';center:number[];major_radius:number;minor_radius:number}
 | {kind:'union'|'intersection'|'difference';a:SdfField;b:SdfField}
 | {kind:'smooth_union';a:SdfField;b:SdfField;radius:number}
 | {kind:'offset';input:SdfField;distance:number}
 | {kind:'translate';input:SdfField;vector:number[]}
export interface SdfGrid {min:number[];max:number[];cells:number[]}
export function validateSdfBudget(field:SdfField):void {
 const pending:{field:SdfField;depth:number}[]=[{field,depth:0}];let count=0;let triangles=0;
 while(pending.length){const {field:f,depth}=pending.pop()!;if(++count>256||depth>32)throw new Error('SDF tree exceeds 256 nodes or depth 32');
 if(f.kind==='mesh_distance'&&(triangles+=f.mesh.indices.length/3)>4096)throw new Error('SDF mesh sources exceed 4096 triangles');
 if('a' in f)pending.push({field:f.a,depth:depth+1},{field:f.b,depth:depth+1});
 if('input' in f)pending.push({field:f.input,depth:depth+1});}
}
export const evaluateSdf=(field:SdfField,point:number[]):number=>{validateSdfBudget(field);return callGeometryRust('sdf_evaluate',{field,point})}
export const tessellateSdf=(field:SdfField,grid:SdfGrid):PolygonBuild=>{validateSdfBudget(field);return callGeometryRust('sdf_tessellate',{field,grid})}

/** Twist preserves the zero set through inverse mapping; distance is not certified. */
export const deformSdf=(field:SdfField,deformation:GeometryDeformation):SdfField=>{validateSdfBudget(field);return callGeometryRust('sdf_deform',{field,deformation})}
export const sculptSdfSphere=(field:SdfField,center:number[],radius:number,remove=false):SdfField=>{validateSdfBudget(field);return callGeometryRust('sdf_sculpt_sphere',{field,center,radius,remove})}
