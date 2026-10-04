import type { TransparentTriangle } from './transparentTriangleSplit'
import { buildTransparentBsp, type PackedTransparentBsp } from './geometry/transparentBspKernel'
export interface TransparentFragment { readonly owner:string; readonly triangle:TransparentTriangle }
/** Rust builds and cuts once. Camera traversal reuses detached numeric buffers. */
export class TransparentBsp {
  private readonly tree:PackedTransparentBsp
  private readonly owners:string[]
  constructor(fragments:readonly TransparentFragment[],readonly limit=100000,readonly tolerance=0,readonly operationLimit=100000){
    if(!Number.isSafeInteger(operationLimit)||operationLimit<1||operationLimit>0xffffffff||!Number.isSafeInteger(limit)||limit<1||limit>0xffffffff||!Number.isFinite(tolerance)||tolerance<0)throw Error('Invalid transparency BSP limits')
    this.tree=buildTransparentBsp(fragments.map(f=>f.triangle),limit,tolerance,operationLimit)
    this.owners=fragments.map(f=>f.owner)
  }
  get operationCount(){return this.tree.operations}
  get fragmentCount(){return this.tree.count}
  /** Direction toward the eye, in object coordinates. Returned vertices are independent copies. */
  ordered(towardEye:readonly [number,number,number]):TransparentFragment[]{
    const output:TransparentFragment[]=[],{vertices,width,owners}=this.tree
    this.visit(towardEye,index=>{
      const offset=index*3*width
      const triangle=[0,1,2].map(k=>Array.from(vertices.subarray(offset+k*width,offset+(k+1)*width))) as unknown as TransparentTriangle
      output.push({owner:this.owners[owners[index]],triangle})
    })
    return output
  }
  /** Writes into caller-owned storage without allocating per-vertex copies. */
  writeOrdered(towardEye:readonly [number,number,number],target:Float32Array):void{
    const {vertices,width,count}=this.tree,stride=width*3
    if(target.length!==count*stride)throw Error('Invalid transparency output size')
    let offset=0
    this.visit(towardEye,index=>{target.set(vertices.subarray(index*stride,(index+1)*stride),offset);offset+=stride})
  }
  private visit(towardEye:readonly [number,number,number],accept:(fragment:number)=>void):void{
    if(towardEye.some(x=>!Number.isFinite(x))||Math.hypot(...towardEye)===0)throw Error('Invalid transparency view direction')
    const {root,planes,links}=this.tree
    // Positive tokens are node+1; negative tokens are -(fragment+1).
    const stack:number[]=root?[root]:[]
    while(stack.length){
      const item=stack.pop()!
      if(item<0){accept(-item-1);continue}
      const offset=(item-1)*4
      const positive=planes[offset]*towardEye[0]+planes[offset+1]*towardEye[1]+planes[offset+2]*towardEye[2]>=0
      const near=links[offset+(positive?0:1)],far=links[offset+(positive?1:0)]
      if(near)stack.push(near)
      const start=links[offset+2],count=links[offset+3]
      for(let i=count-1;i>=0;i--)stack.push(-(start+i+1))
      if(far)stack.push(far)
    }
  }
}
