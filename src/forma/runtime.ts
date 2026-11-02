import { h, type Slots, type VNode, type VNodeChild } from 'vue'
import {
  parseFormaUi,
  type FormaAstNode,
  type FormaComponentAst,
  type FormaElementNode,
} from './parser'

export interface FormaRenderContext {
  state: Record<string, unknown>
  tokens: Record<string, string>
  i18n: Record<string, string>
  locals: Record<string, unknown>
  slots?: Slots
  onAction?: (name: string, args: unknown[], event?: Event) => void
  onBinding?: (path: string, value: unknown) => void
}

function getByPath(obj: unknown, path: string): unknown {
  if (!path) return obj
  const parts = path.split('.')
  let cur: unknown = obj
  for (const part of parts) {
    if (cur == null || typeof cur !== 'object') return undefined
    cur = (cur as Record<string, unknown>)[part]
  }
  return cur
}

export function evalFormaExpr(expr: string, ctx: FormaRenderContext): unknown {
  const trimmed = expr.trim()
  if (!trimmed) return ''
  if (trimmed === 'true') return true
  if (trimmed === 'false') return false
  if (trimmed === 'null' || trimmed === 'none') return null
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    try {
      return JSON.parse(trimmed)
    } catch {
      return trimmed.slice(1, -1)
    }
  }
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed)
  if (/^-?\d+(\.\d+)?(px|%|rem|em|vh|vw|dvh|fr)$/.test(trimmed)) return trimmed

  if (trimmed.startsWith('!')) {
    return !Boolean(evalFormaExpr(trimmed.slice(1), ctx))
  }

  // Ternary expression: cond ? a : b (top-level)
  const qIdx = findTopLevelChar(trimmed, '?')
  if (qIdx !== -1) {
    const cIdx = findTopLevelChar(trimmed, ':', qIdx + 1)
    if (cIdx !== -1) {
      const condVal = evalFormaExpr(trimmed.slice(0, qIdx), ctx)
      return condVal
        ? evalFormaExpr(trimmed.slice(qIdx + 1, cIdx), ctx)
        : evalFormaExpr(trimmed.slice(cIdx + 1), ctx)
    }
  }

  for (const op of ['||', '&&', '===', '!==', '==', '!=', '>=', '<=', '>', '<', '+']) {
    const idx = findTopLevelOp(trimmed, op)
    if (idx !== -1) {
      const left = evalFormaExpr(trimmed.slice(0, idx), ctx)
      const right = evalFormaExpr(trimmed.slice(idx + op.length), ctx)
      switch (op) {
        case '||': return Boolean(left) ? left : right
        case '&&': return Boolean(left) ? right : left
        case '===':
        case '==': return left === right
        case '!==':
        case '!=': return left !== right
        case '>=': return Number(left) >= Number(right)
        case '<=': return Number(left) <= Number(right)
        case '>': return Number(left) > Number(right)
        case '<': return Number(left) < Number(right)
        case '+':
          if (typeof left === 'string' || typeof right === 'string') {
            return `${left ?? ''}${right ?? ''}`
          }
          return Number(left) + Number(right)
      }
    }
  }

  if (trimmed.startsWith('state.')) return getByPath(ctx.state, trimmed.slice(6))
  if (trimmed.startsWith('tokens.')) return ctx.tokens[trimmed.slice(7)] ?? ''
  if (trimmed.startsWith('i18n.')) return ctx.i18n[trimmed.slice(5)] ?? trimmed.slice(5)

  if (trimmed in ctx.locals) return ctx.locals[trimmed]
  const dotIdx = trimmed.indexOf('.')
  if (dotIdx !== -1) {
    const head = trimmed.slice(0, dotIdx)
    const tail = trimmed.slice(dotIdx + 1)
    if (head in ctx.locals) return getByPath(ctx.locals[head], tail)
    if (head in ctx.state) return getByPath(ctx.state[head], tail)
  }
  if (trimmed in ctx.state) return ctx.state[trimmed]
  if (trimmed in ctx.i18n) return ctx.i18n[trimmed]
  if (trimmed in ctx.tokens) return ctx.tokens[trimmed]

  return trimmed
}

function findTopLevelChar(s: string, target: string, start = 0): number {
  let depth = 0
  let quote: string | null = null
  for (let i = start; i < s.length; i++) {
    const ch = s[i]
    if (quote) {
      if (ch === quote && s[i - 1] !== '\\') quote = null
      continue
    }
    if (ch === '"' || ch === "'") {
      quote = ch
      continue
    }
    if (ch === '(' || ch === '[') depth++
    else if (ch === ')' || ch === ']') depth = Math.max(0, depth - 1)
    else if (ch === target && depth === 0) return i
  }
  return -1
}

function findTopLevelOp(s: string, op: string): number {
  let depth = 0
  let quote: string | null = null
  for (let i = 0; i <= s.length - op.length; i++) {
    const ch = s[i]
    if (quote) {
      if (ch === quote && s[i - 1] !== '\\') quote = null
      continue
    }
    if (ch === '"' || ch === "'") {
      quote = ch
      continue
    }
    if (ch === '(' || ch === '[') depth++
    else if (ch === ')' || ch === ']') depth = Math.max(0, depth - 1)
    else if (depth === 0 && s.startsWith(op, i)) {
      if (op === '=' || op === '!' || op === '<' || op === '>') {
        if (s[i + op.length] === '=') continue
      }
      return i
    }
  }
  return -1
}

function evalProp(el: FormaElementNode, name: string, ctx: FormaRenderContext): unknown {
  if (name in el.bindings) {
    return getByPath(ctx.state, el.bindings[name])
  }
  if (!(name in el.props)) return undefined
  return evalFormaExpr(el.props[name], ctx)
}

function buildAriaAndDomAttrs(
  el: FormaElementNode,
  ctx: FormaRenderContext,
  extraClass = '',
): Record<string, unknown> {
  const attrs: Record<string, unknown> = {}
  const cls: string[] = []
  if (extraClass) cls.push(extraClass)
  const userClass = evalProp(el, 'class', ctx) ?? evalProp(el, 'className', ctx)
  if (userClass) cls.push(String(userClass))
  if (Boolean(evalProp(el, 'active', ctx))) cls.push('active')
  if (cls.length) attrs.class = cls.join(' ')

  const id = evalProp(el, 'id', ctx)
  if (id !== undefined && id !== null && id !== '') attrs.id = String(id)
  const role = evalProp(el, 'role', ctx)
  if (role) attrs.role = String(role)
  const title = evalProp(el, 'title', ctx) ?? evalProp(el, 'tooltip', ctx)
  if (title !== undefined && title !== null && title !== '') attrs.title = String(title)
  const ariaLabel = evalProp(el, 'ariaLabel', ctx)
  if (ariaLabel !== undefined && ariaLabel !== null && ariaLabel !== '') {
    attrs['aria-label'] = String(ariaLabel)
  }
  const ariaPressed = evalProp(el, 'ariaPressed', ctx)
  if (ariaPressed !== undefined && ariaPressed !== null) {
    attrs['aria-pressed'] = Boolean(ariaPressed)
  }
  const ariaExpanded = evalProp(el, 'ariaExpanded', ctx)
  if (ariaExpanded !== undefined && ariaExpanded !== null) {
    attrs['aria-expanded'] = Boolean(ariaExpanded)
  }
  const ariaSelected = evalProp(el, 'ariaSelected', ctx)
  if (ariaSelected !== undefined && ariaSelected !== null) {
    attrs['aria-selected'] = Boolean(ariaSelected)
  }
  const ariaControls = evalProp(el, 'ariaControls', ctx)
  if (ariaControls) attrs['aria-controls'] = String(ariaControls)
  const ariaKeyshortcuts = evalProp(el, 'ariaKeyshortcuts', ctx)
  if (ariaKeyshortcuts) attrs['aria-keyshortcuts'] = String(ariaKeyshortcuts)
  const ariaHaspopup = evalProp(el, 'ariaHaspopup', ctx)
  if (ariaHaspopup) attrs['aria-haspopup'] = String(ariaHaspopup)
  const ariaLive = evalProp(el, 'ariaLive', ctx)
  if (ariaLive) attrs['aria-live'] = String(ariaLive)
  const ariaOrientation = evalProp(el, 'ariaOrientation', ctx)
  if (ariaOrientation) attrs['aria-orientation'] = String(ariaOrientation)
  const ariaHidden = evalProp(el, 'ariaHidden', ctx)
  if (ariaHidden !== undefined) attrs['aria-hidden'] = String(Boolean(ariaHidden))
  const tabindex = evalProp(el, 'tabindex', ctx)
  if (tabindex !== undefined && tabindex !== null) attrs.tabindex = Number(tabindex)
  const disabled = evalProp(el, 'disabled', ctx)
  if (disabled !== undefined) attrs.disabled = Boolean(disabled)
  const testId = evalProp(el, 'testId', ctx)
  if (testId) attrs['data-testid'] = String(testId)

  const style: Record<string, string> = {}
  const gap = evalProp(el, 'gap', ctx)
  if (gap !== undefined) style.gap = typeof gap === 'number' ? `${gap}px` : String(gap)
  const padding = evalProp(el, 'padding', ctx)
  if (padding !== undefined) {
    style.padding = typeof padding === 'number' ? `${padding}px` : String(padding)
  }
  const width = evalProp(el, 'width', ctx)
  if (width !== undefined) style.width = typeof width === 'number' ? `${width}px` : String(width)
  const height = evalProp(el, 'height', ctx)
  if (height !== undefined) {
    style.height = typeof height === 'number' ? `${height}px` : String(height)
  }
  if (Object.keys(style).length) attrs.style = style

  if (el.actions.clicked) {
    const act = el.actions.clicked
    attrs.onClick = (ev: Event) => {
      const evaluatedArgs = act.args.map(a => evalFormaExpr(a, ctx))
      ctx.onAction?.(act.name, evaluatedArgs, ev)
    }
  }
  if (el.actions.keydown) {
    const act = el.actions.keydown
    attrs.onKeydown = (ev: Event) => {
      const evaluatedArgs = act.args.map(a => evalFormaExpr(a, ctx))
      ctx.onAction?.(act.name, evaluatedArgs, ev)
    }
  }
  return attrs
}

function renderElement(el: FormaElementNode, ctx: FormaRenderContext): VNodeChild {
  const children = renderNodes(el.children, ctx)
  const textProp = evalProp(el, 'text', ctx) ?? evalProp(el, 'label', ctx)
  const content: VNodeChild[] =
    textProp !== undefined && textProp !== null && textProp !== ''
      ? [String(textProp), ...children]
      : children

  switch (el.type) {
    case 'Slot': {
      const slotName = String(evalProp(el, 'name', ctx) ?? 'default')
      return ctx.slots?.[slotName]?.() ?? children
    }
    case 'Nav':
      return h('nav', buildAriaAndDomAttrs(el, ctx, 'forma-nav'), content)
    case 'Footer':
      return h('footer', buildAriaAndDomAttrs(el, ctx, 'forma-footer'), content)
    case 'Section':
      return h('section', buildAriaAndDomAttrs(el, ctx, 'forma-section'), content)
    case 'Row':
      return h('div', buildAriaAndDomAttrs(el, ctx, 'forma-row'), content)
    case 'Column':
      return h('div', buildAriaAndDomAttrs(el, ctx, 'forma-col'), content)
    case 'Stack':
    case 'Surface':
    case 'Frame':
      return h('div', buildAriaAndDomAttrs(el, ctx, 'forma-surface'), content)
    case 'Spacer':
      return h('span', buildAriaAndDomAttrs(el, ctx, 'toolbar-spacer'), [])
    case 'Divider':
      return h('span', buildAriaAndDomAttrs(el, ctx, 'vt-sep'), [])
    case 'Text':
    case 'Span':
      return h('span', buildAriaAndDomAttrs(el, ctx), content)
    case 'Strong':
      return h('strong', buildAriaAndDomAttrs(el, ctx), content)
    case 'Paragraph':
      return h('p', buildAriaAndDomAttrs(el, ctx), content)
    case 'Keycap':
      return h('kbd', buildAriaAndDomAttrs(el, ctx, 'forma-kbd'), content)
    case 'Badge':
      return h('span', buildAriaAndDomAttrs(el, ctx, 'forma-badge'), content)
    case 'StatusDot':
      return h('i', buildAriaAndDomAttrs(el, ctx, 'status-dot'), [])
    case 'SvgIcon': {
      const size = Number(evalProp(el, 'size', ctx) ?? 15)
      const strokeWidth = Number(evalProp(el, 'strokeWidth', ctx) ?? 2)
      const fill = String(evalProp(el, 'fill', ctx) ?? 'none')
      const pathD = evalProp(el, 'path', ctx)
      const path2D = evalProp(el, 'path2', ctx)
      const svgChildren: VNode[] = []
      if (pathD) svgChildren.push(h('path', { d: String(pathD) }))
      if (path2D) svgChildren.push(h('path', { d: String(path2D) }))
      return h(
        'svg',
        {
          ...buildAriaAndDomAttrs(el, ctx),
          width: size,
          height: size,
          viewBox: '0 0 24 24',
          fill,
          stroke: fill === 'currentColor' ? 'none' : 'currentColor',
          'stroke-width': strokeWidth,
          'stroke-linecap': 'round',
          'stroke-linejoin': 'round',
          'aria-hidden': 'true',
        },
        [...svgChildren, ...children],
      )
    }
    case 'Button':
    case 'PrimaryButton':
    case 'GhostButton':
    case 'SegmentButton':
    case 'IconButton':
    case 'QuietIconButton': {
      const defaultCls =
        el.type === 'PrimaryButton'
          ? 'btn btn-primary forma-btn'
          : el.type === 'IconButton' || el.type === 'QuietIconButton'
            ? 'icon-btn forma-icon-btn'
            : el.type === 'SegmentButton'
              ? 'forma-seg-btn'
              : 'btn forma-btn'
      const attrs = buildAriaAndDomAttrs(el, ctx, defaultCls)
      attrs.type = String(evalProp(el, 'buttonType', ctx) ?? 'button')
      return h('button', attrs, content)
    }
    case 'Checkbox': {
      const checked = Boolean(evalProp(el, 'checked', ctx))
      const bindPath = el.bindings.checked
      const labelText = evalProp(el, 'label', ctx)
      return h('label', buildAriaAndDomAttrs(el, ctx, 'auto-check forma-checkbox'), [
        h('input', {
          type: 'checkbox',
          checked,
          onChange: (ev: Event) => {
            const next = (ev.target as HTMLInputElement).checked
            if (bindPath) ctx.onBinding?.(bindPath, next)
            if (el.actions.changed) {
              ctx.onAction?.(el.actions.changed.name, [next], ev)
            }
          },
        }),
        labelText ? ` ${String(labelText)}` : null,
        ...children,
      ])
    }
    case 'TextField': {
      const val = String(evalProp(el, 'value', ctx) ?? '')
      const bindPath = el.bindings.value
      const attrs = buildAriaAndDomAttrs(el, ctx, 'forma-input')
      attrs.type = String(evalProp(el, 'inputType', ctx) ?? 'text')
      attrs.value = val
      const placeholder = evalProp(el, 'placeholder', ctx)
      if (placeholder) attrs.placeholder = String(placeholder)
      const maxlength = evalProp(el, 'maxlength', ctx)
      if (maxlength) attrs.maxlength = Number(maxlength)
      attrs.onInput = (ev: Event) => {
        const next = (ev.target as HTMLInputElement).value
        if (bindPath) ctx.onBinding?.(bindPath, next)
        if (el.actions.changed) ctx.onAction?.(el.actions.changed.name, [next], ev)
      }
      return h('input', attrs)
    }
    case 'ProgressBar': {
      const max = Number(evalProp(el, 'max', ctx) ?? 100)
      const value = Number(evalProp(el, 'value', ctx) ?? 0)
      return h('progress', { ...buildAriaAndDomAttrs(el, ctx), max, value })
    }
    default:
      return h('div', buildAriaAndDomAttrs(el, ctx), content)
  }
}

export function renderNodes(nodes: FormaAstNode[], ctx: FormaRenderContext): VNodeChild[] {
  const out: VNodeChild[] = []
  for (const node of nodes) {
    if (node.kind === 'let') {
      ctx.locals[node.name] = evalFormaExpr(node.expr, ctx)
      continue
    }
    if (node.kind === 'if') {
      const cond = Boolean(evalFormaExpr(node.condition, ctx))
      const branch = cond ? node.thenBranch : node.elseBranch
      out.push(...renderNodes(branch, ctx))
      continue
    }
    if (node.kind === 'for') {
      const rawList = evalFormaExpr(node.listExpr, ctx)
      const list = Array.isArray(rawList) ? rawList : []
      list.forEach((item, index) => {
        const childCtx: FormaRenderContext = {
          ...ctx,
          locals: {
            ...ctx.locals,
            [node.itemVar]: item,
            ...(node.indexVar ? { [node.indexVar]: index } : {}),
          },
        }
        out.push(...renderNodes(node.body, childCtx))
      })
      continue
    }
    if (node.kind === 'element') {
      out.push(renderElement(node, ctx))
    }
  }
  return out
}

export function renderFormaAst(
  ast: FormaComponentAst,
  state: Record<string, unknown>,
  locale: string,
  options: {
    slots?: Slots
    onAction?: (name: string, args: unknown[], event?: Event) => void
    onBinding?: (path: string, value: unknown) => void
  } = {},
): VNodeChild[] {
  const ctx: FormaRenderContext = {
    state,
    tokens: ast.tokens,
    i18n: ast.i18n[locale] ?? ast.i18n.ru ?? {},
    locals: {},
    slots: options.slots,
    onAction: options.onAction,
    onBinding: options.onBinding,
  }
  return renderNodes(ast.children, ctx)
}

export { parseFormaUi }
