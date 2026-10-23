<script setup lang="ts">
defineProps<{lang:string;evidence:{profileG2Certified?:boolean;profileG1Certified?:boolean;profileSeamCount?:number;stationContinuity?:string;capGeometricG2Certified?:boolean;capGeometricSeamCount?:number;capNormalChordUpper?:number;capCurvatureTensorUpper?:number}}>()
</script>
<template>
            <template v-if="evidence.profileG2Certified !== undefined">
              · {{ lang === 'ru' ? 'G1/G2 сохранённого профиля' : 'Saved profile G1/G2' }}:
              {{ evidence.profileG1Certified ? '✓' : '?' }} / {{ evidence.profileG2Certified ? '✓' : '?' }} ({{ evidence.profileSeamCount }})
              · {{ lang === 'ru' ? 'station-швы' : 'station seams' }}: {{ evidence.stationContinuity }}
            </template>
            <template v-if="evidence.capGeometricG2Certified !== undefined">
              · {{ lang === 'ru' ? 'Внутренние швы крышек G2 в допуске' : 'Cap internal seams G2 within tolerance' }}:
              {{ evidence.capGeometricG2Certified ? '✓' : '?' }} ({{ evidence.capGeometricSeamCount }})
              <template v-if="evidence.capGeometricG2Certified"> · Δn ≤ {{ evidence.capNormalChordUpper?.toExponential(2) }} · Δκ ≤ {{ evidence.capCurvatureTensorUpper?.toExponential(2) }} mm⁻¹</template>
              · {{ lang === 'ru' ? 'Торец–стенка' : 'Cap–wall' }}: C0
            </template>
</template>
