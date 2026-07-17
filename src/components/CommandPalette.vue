<script setup lang="ts">
import { computed, nextTick, ref, useId, watch } from 'vue'

interface PaletteCommand {
  id: string
  label: string
  detail?: string
  shortcut?: string
  keywords?: string | readonly string[]
}

const props = defineProps<{
  open: boolean
  commands: readonly PaletteCommand[]
}>()

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

const filteredCommands = computed(() => {
  const terms = normalize(query.value).split(/\s+/).filter(Boolean)
  if (!terms.length) return props.commands

  return props.commands.filter(command => {
    const keywords = typeof command.keywords === 'string'
      ? command.keywords
      : command.keywords?.join(' ') ?? ''
    const searchable = normalize([
      command.label,
      command.detail ?? '',
      keywords,
    ].join(' '))
    return terms.every(term => searchable.includes(term))
  })
})

const activeOptionId = computed(() => activeIndex.value >= 0
  ? optionId(activeIndex.value)
  : undefined)

watch(() => props.open, async open => {
  if (open) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
    query.value = ''
    activeIndex.value = props.commands.length ? 0 : -1
    await nextTick()
    inputRef.value?.focus()
    inputRef.value?.select()
  } else if (previousFocus?.isConnected) {
    await nextTick()
    previousFocus.focus({ preventScroll: true })
    previousFocus = null
  }
})

watch(query, () => {
  activeIndex.value = filteredCommands.value.length ? 0 : -1
})

watch(filteredCommands, commands => {
  if (!commands.length) activeIndex.value = -1
  else if (activeIndex.value < 0 || activeIndex.value >= commands.length) activeIndex.value = 0
})

watch(activeIndex, async index => {
  if (index < 0) return
  await nextTick()
  dialogRef.value
    ?.querySelector<HTMLElement>(`[data-palette-index="${index}"]`)
    ?.scrollIntoView({ block: 'nearest' })
})

function normalize(value: string) {
  return value.normalize('NFKD').toLocaleLowerCase()
}

function optionId(index: number) {
  return `${listboxId}-option-${index}`
}

function moveActive(direction: 1 | -1) {
  const count = filteredCommands.value.length
  if (!count) return
  activeIndex.value = (activeIndex.value + direction + count) % count
}

function execute(command: PaletteCommand | undefined) {
  if (!command) return
  emit('execute', command.id)
  emit('close')
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
              :class="{ active: index === activeIndex }"
              type="button"
              role="option"
              tabindex="-1"
              :aria-selected="index === activeIndex"
              :data-palette-index="index"
              @pointerenter="activeIndex = index"
              @click="execute(command)"
            >
              <span class="command-copy">
                <span class="command-label">{{ command.label }}</span>
                <span v-if="command.detail" class="command-detail">{{ command.detail }}</span>
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
  background: rgba(4, 6, 10, 0.58);
  backdrop-filter: blur(3px);
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
