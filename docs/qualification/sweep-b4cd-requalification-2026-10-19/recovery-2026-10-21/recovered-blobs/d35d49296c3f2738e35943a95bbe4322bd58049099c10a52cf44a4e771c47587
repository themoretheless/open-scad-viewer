export interface CpuPaintTriangle {vertices:readonly (readonly number[])[];color:string;opacity:number;stroke:boolean}
/** Uses the already ordered/projected scene, including its transparency splits. */
export function paintCpuOrbit(ctx:CanvasRenderingContext2D,triangles:readonly CpuPaintTriangle[],width:number,height:number,size:number,left:number,top:number,cssWidth:number){
 ctx.setTransform(1,0,0,1,0,0);ctx.clearRect(0,0,width,height)
 ctx.setTransform(width/size,0,0,height/size,-left*width/size,-top*height/size)
 ctx.lineWidth=size/cssWidth;ctx.lineJoin='round'
 for(const triangle of triangles){
  ctx.globalAlpha=triangle.opacity;ctx.fillStyle=ctx.strokeStyle=triangle.color
  ctx.beginPath();ctx.moveTo(triangle.vertices[0][0],triangle.vertices[0][1])
  for(let i=1;i<triangle.vertices.length;i++)ctx.lineTo(triangle.vertices[i][0],triangle.vertices[i][1])
  ctx.closePath();ctx.fill();if(triangle.stroke)ctx.stroke()
 }
 ctx.globalAlpha=1
}
