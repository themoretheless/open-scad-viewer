<script lang="ts">
import { computed, defineComponent, type PropType } from 'vue'
import { parseFormaUi, renderFormaAst } from '../forma/runtime'

export default defineComponent({
  name: 'FormaRegion',
  props: {
    source: {
      type: String,
      required: true,
    },
    state: {
      type: Object as PropType<Record<string, unknown>>,
      default: () => ({}),
    },
    locale: {
      type: String,
      default: 'ru',
    },
  },
  emits: {
    action: (_name: string, _args: unknown[], _event?: Event) => true,
    binding: (_path: string, _value: unknown) => true,
  },
  setup(props, { slots, emit }) {
    const ast = computed(() => parseFormaUi(props.source))
    return () =>
      renderFormaAst(ast.value, props.state, props.locale, {
        slots,
        onAction: (name, args, event) => emit('action', name, args, event),
        onBinding: (path, value) => emit('binding', path, value),
      })
  },
})
</script>
