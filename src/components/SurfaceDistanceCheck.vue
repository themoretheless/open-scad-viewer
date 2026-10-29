<script setup lang="ts">
import type {NurbsSurfaceDistance} from '../services/nurbsSurface'
const props=defineProps<{value:NurbsSurfaceDistance|null;error:string;pending:boolean;retryVisible:boolean;ru:boolean}>()
const opened=defineModel<boolean>('opened',{required:true}),budget=defineModel<number>('budget',{required:true})
const emit=defineEmits<{retry:[]}>()
const label=(ru:string,en:string)=>props.ru?ru:en
</script>
<template>
              <section aria-label="surface-distance" class="body-diagnostics">
                <button @click="opened=!opened" :aria-pressed="opened">{{ label('Расстояние между поверхностями','Distance between surfaces') }}</button>
                <template v-if="opened">
                  <label>{{ label('Объём расчёта','Calculation budget') }}<select v-model.number="budget" :aria-label="label('Объём расчёта поверхностей','Surface calculation budget')"><option :value="10000">{{ label('Обычный','Standard') }}</option><option :value="100000">{{ label('Расширенный','Extended') }}</option></select></label>
                  <small v-if="pending" role="status">{{ label('Измеряю расстояние…','Measuring distance…') }} <button @click="opened=false">Esc</button></small>
                  <template v-if="value">
                    <output data-surface-distance>{{ value.distanceIntervalMm.map(v=>Number(v.toPrecision(12)).toString()).join(' … ') }} mm</output>
                    <small v-if="value.converged">{{ label('Допуск расстояния достигнут: 0,001 мм.','Distance tolerance reached: 0.001 mm.') }}</small>
                    <p v-else role="status">{{ label('Расчёт не завершён: показаны нижняя и верхняя границы. Увеличьте объём расчёта или проверьте меньшие участки поверхностей.','Calculation incomplete: lower and upper bounds are shown. Increase the calculation budget or inspect smaller surface regions.') }}</p>
                    <small>{{ label('Измерены полные выбранные NURBS-поверхности.','The complete selected NURBS surfaces are measured.') }}</small>
                  </template>
                  <p v-if="error" role="alert">{{ error }}</p><button v-if="retryVisible" @click="emit('retry')">{{ label('Повторить измерение поверхностей','Retry surface distance') }}</button>
                </template>
              </section>
</template>
<style scoped>
.body-diagnostics{display:grid;gap:8px;padding:8px 0}.body-diagnostics label{display:grid;gap:4px}.body-diagnostics p{margin:0}.body-diagnostics small{color:var(--text-dim)}
</style>
