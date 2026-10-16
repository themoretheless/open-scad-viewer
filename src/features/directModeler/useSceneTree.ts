import { type DirectBody, type DirectDocument, type DirectSketch, type DirectHistory } from '../../services/directModeling'
import { type Pane, DEFAULT_GROUP_SOURCE } from './helpers'
import { type SolidNurbsCurve, type SolidNurbsSurface } from '../../services/solidNurbs'
import { computed, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface SceneTreeOptions {
  cancelCommandState: () => void
  cancelGesture: () => void
  commit: (next: DirectDocument) => void
  document: ShallowRef<DirectDocument>
  documentObjects: (d: DirectDocument) => (DirectSketch | DirectBody | SolidNurbsCurve | SolidNurbsSurface)[]
  emit: (event: 'edit-group', request: { name: string; source: string; replaces: string | null }) => void
  extraSelection: Ref<string[]>
  fit: (pane: Pane, selectedOnly?: boolean) => void
  history: DirectHistory
  hovered: Ref<string>
  isolatedBodyIds: Ref<string[]>
  label: (a: string, b: string) => string
  lockedIds: Ref<string[]>
  objectSelectable: (id: string) => boolean
  run: (action: () => void) => void
  selectedIds: ComputedRef<string[]>
  selection: Ref<string>
  sync: (next?: DirectDocument) => void
}

export function useSceneTree(options: SceneTreeOptions) {
  const { cancelCommandState, cancelGesture, commit, document, documentObjects, emit, extraSelection, fit, history, hovered, isolatedBodyIds, label, lockedIds, objectSelectable, run, selectedIds, selection, sync } = options
/** Bodies listed by group, ungrouped first, each group keeping its insertion order. */
const bodySections = computed(() => {
  const ungrouped = document.value.bodies.filter(body => body.group === undefined)
  const sections: { key: string; name: string | null; bodies: DirectBody[] }[] = []
  if (ungrouped.length) sections.push({ key: '', name: null, bodies: ungrouped })
  const named = new Map<string, DirectBody[]>((document.value.groups??[]).map(g=>[g.name,[]]))
  for (const object of documentObjects(document.value))if(object.group&&!named.has(object.group))named.set(object.group,[])
  for (const body of document.value.bodies) {
    if (body.group === undefined) continue
    const existing = named.get(body.group)
    if (existing) existing.push(body)
    else named.set(body.group, [body])
  }
  for (const [name, bodies] of named) sections.push({ key: name, name, bodies })
  return sections
})


interface SceneRow {key:string;height:number;kind:'object'|'source'|'group'|'label'|'empty';pane?:Pane;dot?:string;title?:string;section?:{name:string|null};item?:{id:string;name:string;group?:string;instance?:unknown;brep?:unknown;material?:{color:string}}}
const sceneRows=computed(()=>{
 const rows:SceneRow[]=[]
 const objects=(items:SceneRow['item'][],pane:Pane,dot:string)=>{for(const item of items)if(item)rows.push({key:'object:'+item.id,height:32,kind:'object',item,pane,dot})}
 if(document.value.sketches.length){rows.push({key:'label:sketch',height:28,kind:'label',title:label('Эскизы','Sketches')});objects(document.value.sketches,'2d','sketch')}
 for(const section of bodySections.value){rows.push({key:'group:'+section.key,height:44,kind:'group',section});objects(section.bodies,'3d','body')}
 if(document.value.sourceBodies?.length){
  rows.push({key:'label:source',height:28,kind:'label',title:label('Исходные тела · preview','Native source bodies · preview')})
  for(const item of document.value.sourceBodies)rows.push({key:'source:'+item.id,height:32,kind:'source',item,pane:'3d',dot:'nurbs'})
 }
 const nurbs=[...document.value.curves??[],...document.value.surfaces??[]]
 if(nurbs.length){rows.push({key:'label:nurbs',height:28,kind:'label',title:'NURBS'});objects(nurbs,'3d','nurbs')}
 if(!rows.length)rows.push({key:'empty',height:80,kind:'empty'})
 return rows
})

function isolateGroup(name:string) {
  cancelCommandState()
  isolatedBodyIds.value=documentObjects(document.value).filter(o=>o.group===name).map(o=>o.id)
  const selected=selectedIds.value.filter(id=>isolatedBodyIds.value.includes(id)&&objectSelectable(id))
  selection.value=selected[0]??'';extraSelection.value=selected.slice(1);hovered.value='';fit('3d')
}

function removeGroup(name: string) {
  run(() => {
    if(documentObjects(document.value).some(b=>b.group===name&&lockedIds.value.includes(b.id)))throw Error(label('Сначала разблокируйте объекты группы','Unlock the group objects first'))
    const next = { ...history.document }
    next.bodies = next.bodies.filter(body => body.group !== name)
    next.sketches = next.sketches.filter(item=>item.group!==name)
    if(next.curves)next.curves=next.curves.filter(item=>item.group!==name)
    if(next.surfaces)next.surfaces=next.surfaces.filter(item=>item.group!==name)
    const remaining = (next.groups ?? []).filter(group => group.name !== name)
    next.groups = remaining.length ? remaining : undefined
    cancelGesture()
    commit(next)
    if (!next.bodies.some(body => body.id === selection.value)) selection.value = ''
    sync()
  })
}

/** The source lives in the host's left panel; this only chooses what to open there. */
function openGroupDialog(name: string | null) {
  if (name === null) {
    const taken = new Set(document.value.groups?.map(group => group.name) ?? [])
    let index = 1
    while (taken.has(label('Группа ', 'Group ') + index)) index++
    emit('edit-group', { name: label('Группа ', 'Group ') + index, source: DEFAULT_GROUP_SOURCE, replaces: null })
    return
  }
  const source = document.value.groups?.find(group => group.name === name)?.source ?? ''
  emit('edit-group', { name, source, replaces: name })
}
function openNewGroupFromMenu(event: MouseEvent) {
  const menu = (event.currentTarget as HTMLElement).closest('details')
  if (menu) menu.open = false
  openGroupDialog(null)
}
  return { openNewGroupFromMenu, bodySections, sceneRows, isolateGroup, removeGroup, openGroupDialog }
}
