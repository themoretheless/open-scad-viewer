import {splitTransparentTriangle, type TransparentTriangle} from './transparentTriangleSplit'
export interface TransparentFragment { readonly owner:string; readonly triangle:TransparentTriangle }
interface Node {plane:[number,number,number,number]; fragments:TransparentFragment[]; front?:Node; back?:Node}
/** Orthographic painter ordering. Geometry is split once; camera changes only traverse the tree. */
export class TransparentBsp {
  private root?:Node
  private count=0
  private operations=0
  private width=0
  constructor(fragments:readonly TransparentFragment[],readonly limit=100000,readonly tolerance=0,readonly operationLimit=100000){
    if(!Number.isSafeInteger(operationLimit)||operationLimit<1||!Number.isSafeInteger(limit)||limit<1||!Number.isFinite(tolerance)||tolerance<0)throw Error('Invalid transparency BSP limits')
    this.width=fragments[0]?.triangle[0].length??0
    if(fragments.some(f=>f.triangle.some(v=>v.length!==this.width)))throw Error('Inconsistent transparency vertex width')
    if(fragments.length>limit||fragments.length>operationLimit)throw Error('Transparency input limit exceeded')
    this.partition(fragments,0)
  }
  private partition(fragments:readonly TransparentFragment[],depth:number,parent?:Node,side?:'front'|'back'){
    if(fragments.length>16){
      const t=fragments[0].triangle,a=t[1].map((v,i)=>v-t[0][i]),b=t[2].map((v,i)=>v-t[0][i])
      const n=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],length=Math.hypot(...n)
      const plane:[number,number,number,number]=[...n.map(v=>v/length),-n.reduce((sum,v,i)=>sum+v*t[0][i],0)/length] as [number,number,number,number]
      if(length>0&&fragments.every(f=>{
        if(++this.operations>this.operationLimit)throw Error('Transparency operation limit exceeded')
        return splitTransparentTriangle(f.triangle,plane,this.tolerance).coplanar.length>0
      })){
        for(const fragment of fragments)this.insert(fragment,parent,side)
        return
      }
    }
    const entries=fragments.map(fragment=>({fragment,center:[0,1,2].map(k=>fragment.triangle.reduce((sum,v)=>sum+v[k]/3,0))}))
    const spans=[0,1,2].map(k=>{let min=Infinity,max=-Infinity;for(const e of entries){min=Math.min(min,e.center[k]);max=Math.max(max,e.center[k])}return max-min})
    const axis=spans.indexOf(Math.max(...spans))
    entries.sort((a,b)=>a.center[axis]-b.center[axis])
    // Spatial planes avoid extending every curved face across the entire scene.
    // Small leaves still use geometric planes for exact painter ordering.
    if(entries.length>16&&depth<8&&spans[axis]>this.tolerance){
      const plane:[number,number,number,number]=[0,0,0,-entries[entries.length>>>1].center[axis]]
      plane[axis]=1
      const node:Node={plane,fragments:[]},front:TransparentFragment[]=[],back:TransparentFragment[]=[]
      for(const {fragment} of entries){
        if(++this.operations>this.operationLimit)throw Error('Transparency operation limit exceeded')
        const parts=splitTransparentTriangle(fragment.triangle,plane,this.tolerance)
        for(const [key,target] of [['front',front],['back',back],['coplanar',node.fragments]] as const)
          for(const triangle of parts[key])target.push({owner:fragment.owner,triangle:triangle as unknown as TransparentTriangle})
        if(this.count+front.length+back.length+node.fragments.length>this.limit)throw Error('Transparency fragment limit exceeded')
      }
      if(front.length+back.length>entries.length*1.5){
        for(const {fragment} of entries)this.insert(fragment,parent,side)
        return
      }
      this.count+=node.fragments.length
      if(parent)parent[side!]=node;else this.root=node
      this.partition(front,depth+1,node,'front')
      this.partition(back,depth+1,node,'back')
    }else{
      const pending:Array<[number,number]>=[[0,entries.length]]
      while(pending.length){const [start,end]=pending.pop()!;if(start>=end)continue;const middle=(start+end)>>>1;this.insert(entries[middle].fragment,parent,side);pending.push([start,middle],[middle+1,end])}
    }
  }
  get operationCount(){return this.operations}
  get fragmentCount(){return this.count}
  private insert(fragment:TransparentFragment,parent?:Node,side?:'front'|'back'){
    const pending:Array<{fragment:TransparentFragment;parent?:Node;side?:'front'|'back'}>=[{fragment,parent,side}]
    while(pending.length){
      if(++this.operations>this.operationLimit)throw Error('Transparency operation limit exceeded')
      const task=pending.pop()!,t=task.fragment.triangle
      if(t.some(v=>v.length!==t[0].length||v.length<3||v.some(x=>!Number.isFinite(x))))throw Error('Invalid transparency BSP vertex')
      const a=t[1].map((v,i)=>v-t[0][i]),b=t[2].map((v,i)=>v-t[0][i])
      const n=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]],length=Math.hypot(...n)
      if(!Number.isFinite(length))throw Error('Unrepresentable transparency plane')
      if(length===0)continue // Zero-area primitives produce no raster fragments.
      const node=task.parent?task.parent[task.side!]:this.root
      if(!node){
        if(++this.count>this.limit)throw Error('Transparency fragment limit exceeded')
        const normal=n.map(v=>v/length)
        const created:Node={plane:[normal[0],normal[1],normal[2],-normal.reduce((sum,v,i)=>sum+v*t[0][i],0)],fragments:[{owner:task.fragment.owner,triangle:t.map(v=>[...v]) as unknown as TransparentTriangle}]}
        if(task.parent)task.parent[task.side!]=created;else this.root=created
      }else{
        const parts=splitTransparentTriangle(t,node.plane,this.tolerance)
        for(const part of parts.coplanar){if(++this.count>this.limit)throw Error('Transparency fragment limit exceeded');node.fragments.push({owner:task.fragment.owner,triangle:part as unknown as TransparentTriangle})}
        for(const side of ['front','back'] as const)for(const part of parts[side])pending.push({fragment:{owner:task.fragment.owner,triangle:part as unknown as TransparentTriangle},parent:node,side})
        if(this.count+pending.length>this.limit)throw Error('Transparency fragment limit exceeded')
      }
    }
  }
  /** Direction toward the eye, in object coordinates. Returned vertices are independent copies. */
  ordered(towardEye:readonly [number,number,number]):TransparentFragment[]{
    const output:TransparentFragment[]=[]
    this.visit(towardEye,item=>output.push({owner:item.owner,triangle:item.triangle.map(v=>[...v]) as unknown as TransparentTriangle}))
    return output
  }
  /** Writes into caller-owned storage without allocating per-vertex copies. */
  writeOrdered(towardEye:readonly [number,number,number],target:Float32Array):void{
    if(target.length!==this.count*3*this.width)throw Error('Invalid transparency output size')
    let offset=0
    this.visit(towardEye,item=>{for(const vertex of item.triangle)for(const value of vertex)target[offset++]=value})
  }
  private visit(towardEye:readonly [number,number,number],accept:(fragment:TransparentFragment)=>void):void{
    if(towardEye.some(x=>!Number.isFinite(x))||Math.hypot(...towardEye)===0)throw Error('Invalid transparency view direction')
    const stack:Array<Node|TransparentFragment>=this.root?[this.root]:[]
    while(stack.length){
      const item=stack.pop()!
      if('triangle' in item){accept(item);continue}
      const positive=item.plane[0]*towardEye[0]+item.plane[1]*towardEye[1]+item.plane[2]*towardEye[2]>=0
      const near=positive?item.front:item.back,far=positive?item.back:item.front
      if(near)stack.push(near)
      for(let i=item.fragments.length-1;i>=0;i--)stack.push(item.fragments[i])
      if(far)stack.push(far)
    }
  }
}
