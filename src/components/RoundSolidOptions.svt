<script setup lang="ts">
defineProps<{label:(ru:string,en:string)=>string;primitiveG2Invalid:boolean}>()
const primitiveTopRadius=defineModel<number>('topRadius',{required:true})
const primitiveInnerRadius=defineModel<number>('innerRadius',{required:true})
const primitiveG2Enabled=defineModel<boolean>('g2Enabled',{required:true})
const primitiveG2Size=defineModel<number>('g2Size',{required:true})
const individual=defineModel<boolean>('individual',{required:true})
const transitionSizes=defineModel<[number,number,number,number]>('transitionSizes',{required:true})
const corners=[['Наружный снизу','Outer bottom'],['Наружный сверху','Outer top'],['Внутренний сверху','Inner top'],['Внутренний снизу','Inner bottom']]
function setExtent(index:number,value:number){const next:[number,number,number,number]=[...transitionSizes.value];next[index]=value;transitionSizes.value=next}
</script>
<template>
      <details><summary>{{ label('Параметры круглых тел','Round solid options') }}</summary>
      <label>{{ label('Верхний радиус конуса, мм','Frustum top radius, mm') }} <input v-model.number="primitiveTopRadius" type="number" min="0.00001" /></label>
      <label>{{ label('Внутренний радиус трубы, мм','Tube inner radius, mm') }} <input v-model.number="primitiveInnerRadius" type="number" :aria-invalid="primitiveG2Invalid" min="0.00001" /></label>
      <label><input v-model="primitiveG2Enabled" type="checkbox" /> {{ label('G2-переходы торцов трубы','G2 tube end transitions') }}</label>
      <label v-if="primitiveG2Enabled && !individual">{{ label('Размер G2-перехода, мм','G2 transition size, mm') }} <input v-model.number="primitiveG2Size" type="number" min="0.000001" step="0.1" :aria-invalid="primitiveG2Invalid" /></label>
      <label v-if="primitiveG2Enabled"><input v-model="individual" type="checkbox" /> {{ label('Разные переходы торцов','Individual tube transitions') }}</label>
      <div v-if="primitiveG2Enabled && individual" class="individual-transitions"><label v-for="(corner,index) in corners" :key="index">{{ label(corner[0],corner[1]) }} · {{ label('мм','mm') }} <input :value="transitionSizes[index]" type="number" min="0.000001" step="0.1" :aria-invalid="primitiveG2Invalid" @input="setExtent(index,($event.target as HTMLInputElement).valueAsNumber)" /></label></div>
      <small v-if="primitiveG2Enabled">{{ label('Применяется при создании трубы. Размер ограничен толщиной стенки и высотой.','Applies when creating a tube. Size is bounded by wall thickness and height.') }}</small>
      </details>
</template>
<style scoped>
.individual-transitions{display:flex;flex-wrap:wrap;gap:6px}details{padding:4px 0}summary{cursor:pointer;margin-bottom:6px}label{display:inline-flex;align-items:center;gap:6px;margin:3px 12px 3px 0;font-size:12px}input[type=number]{width:80px;padding:4px;border:1px solid var(--border);border-radius:4px;background:var(--surface);color:var(--text)}small{display:block;color:var(--text-dim);font-size:11px;margin-top:4px}input[aria-invalid=true]{border-color:var(--danger)}
</style>
