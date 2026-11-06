<script setup lang="ts">
import type {ShellDistanceResult} from '../services/solidMeasurements'
defineProps<{value:ShellDistanceResult;ru:boolean}>()
const label=(ru:boolean,a:string,b:string)=>ru?a:b
</script>
<template>
              <output data-shell-distance>{{ value.distanceIntervalMm.map(v=>v===null?'?':Number(v.toPrecision(12)).toString()).join(' … ') }} mm</output>
              <small v-if="value.converged">{{ label(ru,'Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.') }}</small>
              <p v-else-if="value.reason==='empty-domain'" role="status">{{ label(ru,'Не найден материал граней. Проверьте контуры выбранных тел.','No face material was found. Check the selected bodies’ trim loops.') }}</p>
              <p v-else-if="value.distanceIntervalMm[1]===null" role="status">{{ label(ru,'Расчёт не завершён: допустимая пара точек ещё не найдена, верхняя граница неизвестна. Увеличьте объём расчёта.','Calculation incomplete: no admissible point pair was found yet, so the upper bound is unknown. Increase the calculation budget.') }}</p>
              <p v-else role="status">{{ label(ru,'Расчёт не завершён: показаны нижняя и верхняя границы. Увеличьте объём расчёта; при максимальном лимите результат остаётся неполным.','Calculation incomplete: lower and upper bounds are shown. Increase the calculation budget; at the maximum budget, the result remains incomplete.') }}</p>
              <small>{{ label(ru,'Измерены границы тел. Вложение и пересечение объёмов не классифицированы.','Body boundaries are measured. Volume containment and overlap are not classified.') }}</small>
              <small v-if="value.faces">{{ label(ru,'Грани точек A / B: ','Faces at points A / B: ') }}{{ value.faces.map(f=>f+1).join(' / ') }}</small>
            </template>
