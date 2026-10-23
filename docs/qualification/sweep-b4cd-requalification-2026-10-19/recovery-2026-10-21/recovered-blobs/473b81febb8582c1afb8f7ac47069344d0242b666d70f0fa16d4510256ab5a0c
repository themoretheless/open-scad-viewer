<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { MeshData } from '../core/mesh'
import { prepareVrScene, type VrMesh } from '../services/vrScene'
import { VrPresentation, vrSystem } from '../services/vrSession'

const props = withDefaults(defineProps<{ meshes?: readonly MeshData[]; visibility?: readonly boolean[]; isolated?: boolean; selectedIndex?: number | null; getScene?: () => VrMesh[]; available?: boolean; locale: string }>(), { available: undefined })
const ru = computed(() => props.locale === 'ru')
const supported = ref(false), checking = ref(true), active = ref(false), busy = ref(false)
const open = ref(false), error = ref('')
const hasGeometry = computed(() => (props.available ?? props.meshes?.some((mesh, index) => mesh.indices.length && props.visibility?.[index] !== false && (!props.isolated || props.selectedIndex === index)) ?? false))
let mounted = false
const system = vrSystem()
const presentation = new VrPresentation(value => { active.value = value }, () => {
  error.value = ru.value ? 'Сессия VR прервана. Попробуйте войти снова.' : 'VR session interrupted. Please try again.'
})
async function check() {
  checking.value = true
  try { supported.value = !!(window.isSecureContext && await system?.isSessionSupported('immersive-vr')) }
  catch { supported.value = false }
  finally { if (mounted) checking.value = false }
}
async function toggle() {
  busy.value = true
  error.value = ''
  try {
    if (active.value) await presentation.stop()
    else if (system) await presentation.start(system, props.getScene?.() ?? prepareVrScene(props.meshes ?? [], props.visibility ?? [], props.isolated ?? false, props.selectedIndex ?? null))
  } catch {
    error.value = ru.value ? 'Не удалось открыть VR. Проверьте подключение шлема и разрешение браузера, затем повторите.' : 'Could not enter VR. Check your headset connection and browser permission, then retry.'
  } finally { busy.value = false }
}
onMounted(() => { mounted = true; void check(); system?.addEventListener('devicechange', check) })
onUnmounted(() => { mounted = false; system?.removeEventListener('devicechange', check); presentation.dispose() })
</script>

<template>
  <div class="vr-controls" @keydown.esc.stop="open = false">
    <button class="view-btn" type="button" :aria-expanded="open" :aria-label="ru ? 'Режим VR' : 'VR mode'" @click="open = !open">VR</button>
    <section v-if="open" class="vr-panel" :aria-label="ru ? 'Режим VR' : 'VR mode'">
      <strong>{{ ru ? 'Просмотр в VR' : 'View in VR' }}</strong>
      <p>{{ ru ? 'Снимок видимых моделей · размер до 60 см. Триггер контроллера — вернуть модель перед собой. Выход — через меню шлема или кнопку ниже.' : 'Snapshot of visible models · fitted to 60 cm. Use the controller trigger to recenter. Exit with the headset menu or the button below.' }}</p>
      <p>{{ ru ? 'Матовые непрозрачные поверхности; сечения и редактирование в VR не отображаются.' : 'Opaque matte surfaces; section cuts and editing are not shown in VR.' }}</p>
      <p v-if="checking" role="status">{{ ru ? 'Проверка шлема…' : 'Checking headset…' }}</p>
      <p v-else-if="!supported" role="status">{{ ru ? 'Нужен шлем с поддержкой WebXR и HTTPS (или localhost). Откройте приложение в браузере шлема либо подключите шлем к компьютеру.' : 'Requires a WebXR headset and HTTPS (or localhost). Open the app in your headset browser or connect a headset to your computer.' }}</p>
      <p v-else-if="!hasGeometry && !active" role="status">{{ ru ? 'Сначала постройте и покажите модель.' : 'Build and show a model first.' }}</p>
      <p v-if="error" role="alert">{{ error }}</p>
      <button type="button" class="view-btn" :disabled="busy || checking || (!active && (!supported || !hasGeometry))" @click="toggle">{{ busy ? (ru ? 'Подождите…' : 'Please wait…') : active ? (ru ? 'Выйти из VR' : 'Exit VR') : (ru ? 'Войти в VR' : 'Enter VR') }}</button>
      <button v-if="!supported" :disabled="checking" type="button" class="view-btn" @click="check">{{ ru ? 'Проверить снова' : 'Check again' }}</button>
    </section>
  </div>
</template>

<style scoped>
.vr-controls { position: relative; }
.vr-controls button { color: var(--text, #eef3fa); background: var(--surface-raised, #243247); border: 1px solid var(--border, #536277); border-radius: 5px; padding: 6px 10px; cursor: pointer; }
.vr-controls button:disabled { opacity: 0.5; cursor: default; }
.vr-panel { position: absolute; top: calc(100% + 8px); left: 0; z-index: 30; width: min(320px, 75vw); padding: 16px; border: 1px solid #536277; border-radius: 10px; background: var(--surface, #17202e); color: var(--text, #eef3fa); box-shadow: 0 8px 28px #0006; font-size: 13px; line-height: 1.5; }
.vr-panel p { margin: 10px 0; }
.vr-panel button { margin-right: 6px; }
</style>
