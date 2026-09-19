<script setup lang="ts">
import { computed, nextTick, ref, useId, watch } from 'vue'
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
}>(), { restoreFocus: true })

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
    const target = previousFocus
    await nextTick()
    if (props.restoreFocus && target?.isConnected) target.focus({ preventScroll: true })
    previousFocus = null
  }
})

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
              aria-expanded="true"
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
              :aria-describedby="!isPaletteCommandEnabled(command) ? reasonId(index) : undefined"
              :data-palette-index="index"
              @pointerenter="activatePointer(index)"
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
              <kbd v-if="command.shortcut" class="shortcut">{{ command.shortcut }}</kbd>
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

<style scoped>
.palette-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: min(14vh, 120px) 16px 24px;
  /* A light veil: the scene stays readable behind the palette. */
  background: rgba(4, 6, 10, 0.42);
  backdrop-filter: blur(1.5px);
}

.palette {
  width: min(620px, 100%);
  overflow: hidden;
  color: var(--text, #eef0f5);
  background: color-mix(in srgb, var(--surface, #1a1c22) 96%, transparent);
  border: 1px solid var(--border, #30343e);
  border-radius: 13px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.46);
}

.search-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 54px;
  padding: 8px 13px;
  border-bottom: 1px solid var(--border, #30343e);
}

.search-icon {
  flex: 0 0 auto;
  fill: none;
  stroke: var(--text-dim, #a4a9b5);
  stroke-linecap: round;
  stroke-width: 1.8;
}

.search-input {
  min-width: 0;
  flex: 1;
  padding: 6px 0;
  color: inherit;
  background: transparent;
  border: 0;
  outline: 0;
  font: inherit;
  font-size: 1rem;
}

.search-input::placeholder { color: var(--text-dim, #a4a9b5); }
.search-input::-webkit-search-cancel-button { display: none; }

kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 21px;
  min-height: 20px;
  padding: 1px 5px;
  color: var(--text-dim, #a4a9b5);
  background: var(--surface-raised, #22252d);
  border: 1px solid var(--border, #30343e);
  border-radius: 5px;
  box-shadow: inset 0 -1px rgba(0, 0, 0, 0.18);
  font: 600 0.68rem/1 ui-monospace, "SFMono-Regular", Consolas, monospace;
  white-space: nowrap;
}

.escape-key { flex: 0 0 auto; }

.command-list {
  max-height: min(430px, 58vh);
  overflow-y: auto;
  padding: 6px;
  scrollbar-width: thin;
}

.command-option {
  width: 100%;
  min-height: 48px;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 7px 10px;
  color: inherit;
  text-align: left;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 8px;
  cursor: pointer;
}

.command-option:hover,
.command-option.active {
  background: var(--hover, #2a2e38);
  border-color: color-mix(in srgb, var(--accent, #559dff) 25%, transparent);
}

.command-option.is-disabled {
  opacity: 0.56;
  cursor: not-allowed;
}

.command-option.is-disabled:hover {
  background: transparent;
  border-color: transparent;
}

.command-copy {
  min-width: 0;
  flex: 1;
  display: grid;
  gap: 2px;
}

.command-label {
  overflow: hidden;
  font-size: 0.86rem;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.command-detail {
  overflow: hidden;
  color: var(--text-dim, #a4a9b5);
  font-size: 0.72rem;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.command-disabled-reason {
  overflow: hidden;
  color: var(--warning, #f5bd55);
  font-size: 0.68rem;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shortcut { flex: 0 0 auto; }

.empty-state {
  margin: 0;
  padding: 34px 18px;
  color: var(--text-dim, #a4a9b5);
  font-size: 0.8rem;
  text-align: center;
}

.palette-footer {
  min-height: 32px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 14px;
  padding: 5px 11px;
  color: var(--text-dim, #a4a9b5);
  background: color-mix(in srgb, var(--surface-raised, #22252d) 62%, transparent);
  border-top: 1px solid var(--border, #30343e);
  font-size: 0.66rem;
}

.palette-footer span { display: inline-flex; align-items: center; gap: 4px; }
.palette-footer kbd { min-width: 18px; min-height: 17px; padding: 1px 4px; font-size: 0.6rem; }

.sr-only {
  position: absolute !important;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.palette-fade-enter-active,
.palette-fade-leave-active { transition: opacity 120ms ease; }
.palette-fade-enter-active .palette,
.palette-fade-leave-active .palette { transition: transform 120ms ease, opacity 120ms ease; }
.palette-fade-enter-from,
.palette-fade-leave-to { opacity: 0; }
.palette-fade-enter-from .palette,
.palette-fade-leave-to .palette { opacity: 0; transform: translateY(-8px) scale(0.985); }

@media (max-width: 560px) {
  .palette-backdrop { padding: 7dvh 9px 12px; }
  .palette { border-radius: 11px; }
  .command-list { max-height: 64dvh; }
  .palette-footer { display: none; }
  .command-option { min-height: 52px; }
}

@media (prefers-reduced-motion: reduce) {
  .palette-fade-enter-active,
  .palette-fade-leave-active,
  .palette-fade-enter-active .palette,
  .palette-fade-leave-active .palette { transition-duration: 0.01ms; }
}
</style>
