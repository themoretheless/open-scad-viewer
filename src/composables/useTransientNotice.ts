import {onUnmounted,type Ref} from 'vue'
export function useTransientNotice(notice:Ref<string>,duration=2200) {
 let timer:ReturnType<typeof setTimeout>|null=null
 onUnmounted(()=>{if(timer)clearTimeout(timer)})
 return (message:string)=>{
  notice.value=message
  if(timer)clearTimeout(timer)
  timer=setTimeout(()=>{notice.value=''},duration)
 }
}
