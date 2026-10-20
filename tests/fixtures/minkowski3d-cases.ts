import type {Minkowski3dSource} from '../../src/services/geometry/minkowski3d'
export function box3d(lo:number[],hi:number[]):Minkowski3dSource {
 const [x,y,z]=lo,[X,Y,Z]=hi
 return {positions:[x,y,z,X,y,z,X,Y,z,x,Y,z,x,y,Z,X,y,Z,X,Y,Z,x,Y,Z],indices:[0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7]}
}
export function join3d(a:Minkowski3dSource,b:Minkowski3dSource,reverse=false):Minkowski3dSource {
 const indices=[...b.indices];if(reverse)for(let i=0;i<indices.length;i+=3)[indices[i+1],indices[i+2]]=[indices[i+2],indices[i+1]]
 return {positions:[...a.positions,...b.positions],indices:[...a.indices,...indices.map(i=>i+a.positions.length/3)]}
}
export function lPrism3d():Minkowski3dSource {
 const p=[[0,0],[4,0],[4,1],[1,1],[1,4],[0,4]],top=[[0,1,3],[1,2,3],[0,3,5],[3,4,5]]
 const indices=top.flatMap(t=>[t[0],t[2],t[1],...t.map(i=>i+6)])
 for(let a=0;a<6;a++){const b=(a+1)%6;indices.push(a,b,b+6,a,b+6,a+6)}
 return {positions:[...p.flatMap(([x,y])=>[x,y,0]),...p.flatMap(([x,y])=>[x,y,2])],indices}
}
export const cavity3d=()=>join3d(box3d([0,0,0],[10,10,10]),box3d([3,3,3],[7,7,7]),true)
export const tetra3d=():Minkowski3dSource=>({positions:[0,0,0,1,0,0,0,1,0,0,0,1],indices:[0,2,1,0,1,3,0,3,2,1,2,3]})
