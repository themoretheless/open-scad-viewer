<script setup lang="ts">
import {ref} from 'vue'
import type {NurbsSurface} from '../services/nurbsSurface'
import ProfileBridgeCheck from './ProfileBridgeCheck.vue'
import OffsetDeviationCheck from './OffsetDeviationCheck.vue'
defineProps<{surfaces:[NurbsSurface,NurbsSurface];active:boolean;ru:boolean}>()
const profile=ref<InstanceType<typeof ProfileBridgeCheck>|null>(null)
defineExpose({open:()=>profile.value?.open()})
</script>
<template>
 <ProfileBridgeCheck ref="profile" :surfaces="surfaces" :active="active" :ru="ru" />
 <OffsetDeviationCheck :surfaces="surfaces" :active="active" :ru="ru" />
</template>
