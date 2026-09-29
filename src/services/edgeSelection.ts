/** Extend through unbranched topological vertices; never guess a turn at a junction. */
export function connectedEdgeChain(edges:readonly {a:number;b:number}[],seeds:readonly number[]):number[] {
 const selected=new Set(seeds.filter(i=>Number.isInteger(i)&&i>=0&&i<edges.length))
 const adjacent=new Map<number,number[]>()
 edges.forEach((e,i)=>{for(const v of [e.a,e.b]){const list=adjacent.get(v)??[];list.push(i);adjacent.set(v,list)}})
 const pending=[...selected]
 while(pending.length){const edge=edges[pending.pop()!];for(const v of [edge.a,edge.b]){const list=adjacent.get(v)!;if(list.length!==2)continue;for(const i of list)if(!selected.has(i)){selected.add(i);pending.push(i)}}}
 return [...selected].sort((a,b)=>a-b)
}
