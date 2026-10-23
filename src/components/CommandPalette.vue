<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'
import {
  isPaletteCommandEnabled,
  nextEnabledCommandIndex,
  nextPaletteCommandIndex,
  rankPaletteCommands,
  type PaletteCommand,
} from '../services/commandSearch'

const props = withDefaults(defineProps<{
  open: boolean
  commands: readonly PaletteCommand[]
  restoreFocus?: boolean
  platform?: 'mac' | 'windows-linux' | 'unknown'
}>(), { restoreFocus: true, platform: 'unknown' })

function formatShortcutDisplay(shortcut: string | undefined): string {
  if (!shortcut) return ''
  if (props.platform === 'mac') {
    return shortcut.replace(/Ctrl\/⌘\+/g, '⌘').replace(/Ctrl\+/g, '⌃').replace(/Alt\+/g, '⌥').replace(/Shift\+/g, '⇧')
  }
  if (props.platform === 'windows-linux') {
    return shortcut.replace(/Ctrl\/⌘\+/g, 'Ctrl+').replace(/\s*\/\s*⌘⌥[A-Za-z0-9]+/g, '')
  }
  return shortcut
}

const emit = defineEmits<{
  execute: [id: string]
  close: []
}>()

const instanceId = useId()
const titleId = `${instanceId}-title`
const inputId = `${instanceId}-input`
const listboxId = `${instanceId}-listbox`
const query = ref('')
const activeIndex = ref(-1)
const inputRef = ref<HTMLInputElement | null>(null)
const dialogRef = ref<HTMLElement | null>(null)
let previousFocus: HTMLElement | null = null
function restorePreviousFocus() {
  const target=previousFocus
  previousFocus=null
  void nextTick(()=>{if(props.restoreFocus && target?.isConnected && (document.activeElement===document.body || dialogRef.value?.contains(document.activeElement)))target.focus({preventScroll:true})})
}
onBeforeUnmount(restorePreviousFocus)

const filteredCommands = computed(() => rankPaletteCommands(props.commands, query.value))

const activeOptionId = computed(() => activeIndex.value >= 0
  ? optionId(activeIndex.value)
  : undefined)

watch(() => props.open, async open => {
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    query.value = ''
    activeIndex.value = nextEnabledCommandIndex(filteredCommands.value, -1, 1)
    await nextTick()
    inputRef.value?.focus()
    inputRef.value?.select()
  } else {
    restorePreviousFocus()
  }
}, { immediate: true })

watch(filteredCommands, commands => {
  activeIndex.value = nextEnabledCommandIndex(commands, -1, 1)
})

watch(activeIndex, async index => {
  if (index < 0) return
  await nextTick()
  dialogRef.value
    ?.querySelector<HTMLElement>(`[data-palette-index="${index}"]`)
    ?.scrollIntoView({ block: 'nearest' })
})

function optionId(index: number) {
  return `${listboxId}-option-${index}`
}

function reasonId(index: number) {
  return `${optionId(index)}-reason`
}

function moveActive(direction: 1 | -1) {
  activeIndex.value = nextPaletteCommandIndex(
    filteredCommands.value,
    activeIndex.value,
    direction,
  )
}

function execute(command: PaletteCommand | undefined) {
  if (!command || !isPaletteCommandEnabled(command)) return
  emit('execute', command.id)
  emit('close')
}

function activatePointer(index: number) {
  activeIndex.value = index
}

function requestClose() {
  emit('close')
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    event.stopPropagation()
    moveActive(1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    event.stopPropagation()
    moveActive(-1)
  } else if (event.key === 'Enter') {
    event.preventDefault()
    event.stopPropagation()
    execute(filteredCommands.value[activeIndex.value])
  } else if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    requestClose()
  } else if (event.key === 'Tab') {
    // The combobox owns keyboard navigation, so it is the dialog's only tab stop.
    event.preventDefault()
    inputRef.value?.focus()
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="palette-fade">
      <div v-if="open" class="palette-backdrop" @pointerdown.self="requestClose">
        <section
          ref="dialogRef"
          class="palette"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="titleId"
          @keydown="handleKeydown"
          @pointerdown.stop
        >
          <h2 :id="titleId" class="sr-only">Commands / Команды</h2>

          <div class="search-row">
            <svg class="search-icon" width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="11" cy="11" r="7" />
              <path d="m16.5 16.5 4 4" />
            </svg>
            <input
              :id="inputId"
              ref="inputRef"
              v-model="query"
              class="search-input"
              type="search"
              role="combobox"
              aria-autocomplete="list"
              aria-haspopup="listbox"
              :aria-expanded="open"
              :aria-controls="listboxId"
              :aria-activedescendant="activeOptionId"
              aria-label="Search commands / Поиск команд"
              placeholder="Search commands / Поиск команд"
              autocomplete="off"
              spellcheck="false"
            >
            <kbd class="escape-key">Esc</kbd>
          </div>

          <div :id="listboxId" class="command-list" role="listbox" aria-label="Commands / Команды">
            <button
              v-for="(command, index) in filteredCommands"
              :id="optionId(index)"
              :key="command.id"
              class="command-option"
              :class="{ active: index === activeIndex, 'is-disabled': !isPaletteCommandEnabled(command) }"
              type="button"
              role="option"
              tabindex="-1"
              :aria-selected="index === activeIndex"
              :aria-disabled="!isPaletteCommandEnabled(command)"
              :aria-posinset="index + 1"
              :aria-setsize="filteredCommands.length"
              :aria-describedby="!isPaletteCommandEnabled(command) ? reasonId(index) : undefined"
              :data-palette-index="index"
              @pointermove="activatePointer(index)"
              @click="execute(command)"
            >
              <span class="command-copy">
                <span class="command-label">{{ command.label }}</span>
                <span v-if="command.detail" class="command-detail">{{ command.detail }}</span>
                <span
                  v-if="!isPaletteCommandEnabled(command)"
                  :id="reasonId(index)"
                  class="command-disabled-reason"
                >
                  {{ command.disabledReason || 'Unavailable / Недоступно' }}
                </span>
              </span>
              <kbd v-if="command.shortcut" class="shortcut">{{ formatShortcutDisplay(command.shortcut) }}</kbd>
            </button>

            <p v-if="!filteredCommands.length" class="empty-state" role="status">
              No matching commands / Команды не найдены
            </p>
          </div>

          <footer class="palette-footer" aria-hidden="true">
            <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span>
            <span><kbd>↵</kbd> run</span>
            <span><kbd>Esc</kbd> close</span>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped src="./CommandPalette.css"></style>
