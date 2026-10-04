<script setup lang="ts">
import {worldPoint,type SketchPlane} from '../services/directSketchGeometry'
import type {Point2} from '../services/directModeling'
import type {ProfilePreparationReport} from '../services/solidProfilePreparation'
import {computed} from 'vue'
import {profileIntersectionDiagnostics} from '../services/profileIntersectionDiagnostics'
const props=defineProps<{mode:'preview'|'status';report:ProfilePreparationReport;locale?:string;project?:(point:number[],pane:'2d'|'3d')=>number[];pane?:'2d'|'3d';plane?:SketchPlane;view?:number}>()
const diagnostics=computed(()=>props.report.diagnosticLoops&&props.report.intersections?profileIntersectionDiagnostics(props.report.diagnosticLoops,props.report.intersections):null)
const screen=(point:Point2)=>props.project!(props.pane==='2d'?point:worldPoint(point,props.plane),props.pane!)
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
</script>
<template>
              <g v-if="mode==='preview' && diagnostics" data-diagnostic="profile-curve-intersections" pointer-events="none">
                <polyline v-for="segment in report.diagnosticDisplay??[]" :key="'curve'+segment.curve" data-diagnostic="profile-curve-segment" :data-segment-index="segment.curve" :data-status="segment.kind" :points="segment.points.map(p=>screen(p).join(',')).join(' ')" fill="none" :stroke="segment.kind==='unproven'?'#ffc977':'#ff647c'" :stroke-dasharray="segment.kind==='unproven'?'5 3':undefined" stroke-width="4" vector-effect="non-scaling-stroke"/>
                <circle v-for="(event,i) in diagnostics.points" :key="i" data-diagnostic="profile-curve-intersection" :data-segments="[event.first.curve,event.second.curve].join(',')" :cx="screen(event.point)[0]" :cy="screen(event.point)[1]" :r="view!/90" fill="#ff647c" stroke="white" stroke-width="1" vector-effect="non-scaling-stroke"/>
              </g>
  <template v-if="mode==='status'">
                <small v-if="diagnostics" data-testid="profile-intersection-coverage">{{ label('Завершено пар: ','Segment pairs resolved: ')+(report.intersections!.resolvedPairs)+' / '+(report.intersections!.totalPairs)+'. '+label('Самопересечения одной кривой не проверены.','Self-intersections within a single curve are outside this check.') }}</small>
                <small v-if="diagnostics?.endpointBands.length" data-testid="profile-intersection-endpoint-bands">{{ label('Жёлтым — стык или другой контакт в пределах допуска. Проверка не отличает их; проверьте стыки отдельно.','Yellow: a join or another contact within tolerance. This check cannot distinguish them; inspect the joins separately.') }}</small>
                <small v-if="report.intersectionDiagnosticError" data-testid="profile-intersection-unavailable">{{ label('Диагностика пересечений не завершена. Упростите выбранные кривые или проверьте сегменты отдельно.','Intersection diagnostics did not finish. Simplify the selected curves or inspect segments separately.') }}</small>
  </template>
</template>
