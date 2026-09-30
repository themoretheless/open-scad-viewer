<script setup lang="ts" generic="T extends {key:string;height:number}">
import {computed,nextTick,onMounted,onBeforeUnmount,ref,watch} from 'vue'
import {sceneListWindow,sceneRowOffsets} from '../services/sceneListWindow'
const props=defineProps<{items:readonly T[]}>()
defineSlots<{default(props:{item:T;position:number;total:number;style:{height:string;marginBottom:string;flexShrink:number;boxSizing:'border-box'}}):unknown}>()
const list=ref<HTMLUListElement|null>(null),top=ref(0),viewport=ref(600),focused=ref<string|null>(null)
let container:HTMLElement|null=null,observer:ResizeObserver|undefined
const offsets=computed(()=>sceneRowOffsets(props.items.map(row=>row.height)))
const range=computed(()=>props.items.length<=100?{start:0,end:props.items.length}:sceneListWindow(offsets.value,top.value,viewport.value))
const pieces=computed(()=>{
 const indices=new Set<number>()
 for(let i=range.value.start;i<range.value.end;i++)indices.add(i)
 const pin=props.items.findIndex(row=>row.key===focused.value)
 if(pin>=0)indices.add(pin)
 const rows:Array<{key:string;item?:T;height:number;position?:number}>=[]
 let previous=0
 for(const index of [...indices].sort((a,b)=>a-b)){
  if(index>previous)rows.push({key:'gap:'+previous,height:offsets.value[index]-offsets.value[previous]})
  rows.push({key:'row:'+props.items[index].key,item:props.items[index],height:props.items[index].height,position:index+1})
  previous=index+1
 }
 if(previous<props.items.length)rows.push({key:'gap:'+previous,height:offsets.value[props.items.length]-offsets.value[previous]})
 return rows
})
function update(){
 if(!container||!list.value)return
 top.value=container.getBoundingClientRect().top-list.value.getBoundingClientRect().top
 viewport.value=container.clientHeight
}
async function reveal(key:string,focus=false,last=false){
 const index=props.items.findIndex(row=>row.key===key)
 if(index<0||!container||!list.value)return false
 const start=offsets.value[index],end=offsets.value[index+1]
 update()
 const delta=start<top.value?start-top.value:end>top.value+viewport.value?end-top.value-viewport.value:0
 if(delta)container.scrollTop+=delta
 update();await nextTick()
 const row=Array.from(list.value.querySelectorAll<HTMLElement>('[data-scene-key]')).find(row=>row.dataset.sceneKey===key)
 if(focus){const buttons=Array.from(row?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')??[]);const button=last?buttons.at(-1):buttons[0];button?.focus();return !!button}
 return !!row
}
function rowFor(event:Event){return (event.target as HTMLElement)?.closest<HTMLElement>('[data-scene-key]')}
function focusIn(event:FocusEvent){focused.value=rowFor(event)?.dataset.sceneKey??null}
function focusOut(event:FocusEvent){if(!list.value?.contains(event.relatedTarget as Node|null))focused.value=null}
async function keydown(event:KeyboardEvent){
 const row=rowFor(event),index=props.items.findIndex(item=>item.key===row?.dataset.sceneKey)
 const root=event.target===list.value
 if(index<0&&!root)return
 let next=-1,last=false
 if(event.key==='ArrowDown')next=root?0:index+1
 else if(event.key==='ArrowUp')next=root?props.items.length-1:index-1
 else if(event.key==='Home')next=0
 else if(event.key==='End')next=props.items.length-1
 else if(event.key==='Tab'&&!root){
  const buttons=Array.from(row!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'))
  if(event.shiftKey&&event.target===buttons[0]){next=index-1;last=true}
  else if(!event.shiftKey&&event.target===buttons.at(-1))next=index+1
 }
 if(next<0||next>=props.items.length)return
 event.preventDefault();event.stopPropagation()
 const direction=event.key==='End'||event.key==='ArrowUp'||event.shiftKey?-1:1
 while(next>=0&&next<props.items.length){if(await reveal(props.items[next].key,true,last))break;next+=direction}
}
watch(()=>props.items,async()=>{
 if(focused.value&&!props.items.some(row=>row.key===focused.value)){
  focused.value=null;list.value?.focus?.()
 }
 await nextTick();update()
})
onMounted(()=>{
 if(!list.value?.closest)return
 container=list.value?.closest<HTMLElement>('.dock-body')??list.value?.parentElement??null
 container?.addEventListener('scroll',update,{passive:true})
 if(typeof ResizeObserver!=='undefined'){observer=new ResizeObserver(update);if(container)observer.observe(container)}
 update()
})
onBeforeUnmount(()=>{container?.removeEventListener('scroll',update);observer?.disconnect()})
defineExpose({reveal})
</script>
<template>
 <ul ref="list" class="scene-list" tabindex="-1" style="gap:0" @focusin="focusIn" @focusout="focusOut" @keydown="keydown">
  <template v-for="piece in pieces" :key="piece.key">
   <slot v-if="piece.item" :item="piece.item" :position="piece.position!" :total="items.length" :style="{height:piece.height+'px',marginBottom:'1px',flexShrink:0,boxSizing:'border-box'}" />
   <li v-else aria-hidden="true" role="presentation" :style="{height:piece.height+'px',flexShrink:0,pointerEvents:'none'}" />
  </template>
 </ul>
</template>
