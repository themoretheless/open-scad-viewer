import { h, type Slots, type VNode, type VNodeChild } from 'svet'
import {
  parseFormaUi,
  type FormaAstNode,
  type FormaComponentAst,
  type FormaElementNode,
} from './parser'
import {
  batch,
  compileFormaExpression,
  type CompiledExprAst,
  signal,
  type Signal,
} from './svet'

export interface FormaRenderContext {
  state: Record<string, unknown>
  signals?: Map<string, Signal<unknown>>
  tokens: Record<string, string>
  i18n: Record<string, string>
  locals: Record<string, unknown>
  slots?: Slots
  onAction?: (name: string, args: unknown[], event?: Event) => void
  onBinding?: (path: string, value: unknown) => void
}

function getByPathParts(obj: unknown, parts: readonly string[]): unknown {
  let cur: unknown = obj
  for (let i = 0; i < parts.length; i++) {
    if (cur == null || typeof cur !== 'object') return undefined
    cur = (cur as Record<string, unknown>)[parts[i]]
  }
  return cur
}

function readStateField(ctx: FormaRenderContext, key: string): unknown {
  if (ctx.signals) {
    let sig = ctx.signals.get(key)
    if (!sig) {
      sig = signal(ctx.state[key])
      ctx.signals.set(key, sig)
    }
    return sig()
  }
  return ctx.state[key]
}

export function evalCompiledAst(ast: CompiledExprAst, ctx: FormaRenderContext): unknown {
  switch (ast.type) {
    case 'literal':
      return ast.value
    case 'token':
      return ctx.tokens[ast.key] ?? ''
    case 'i18n':
      return ctx.i18n[ast.key] ?? ast.key
    case 'state': {
      const head = ast.path[0]
      const rootVal = readStateField(ctx, head)
      return ast.path.length === 1 ? rootVal : getByPathParts(rootVal, ast.path.slice(1))
    }
    case 'ident': {
      const { name, path } = ast
      if (name in ctx.locals) {
        return path.length === 0 ? ctx.locals[name] : getByPathParts(ctx.locals[name], path)
      }
      if (name in ctx.state || ctx.signals?.has(name)) {
        const rootVal = readStateField(ctx, name)
        return path.length === 0 ? rootVal : getByPathParts(rootVal, path)
      }
      if (path.length === 0) {
        if (name in ctx.i18n) return ctx.i18n[name]
        if (name in ctx.tokens) return ctx.tokens[name]
      }
      return path.length === 0 ? name : `${name}.${path.join('.')}`
    }
    case 'unary': {
      const v = evalCompiledAst(ast.arg, ctx)
      if (ast.op === '!') return !Boolean(v)
      if (ast.op === '-') return -Number(v)
      return +Number(v)
    }
    case 'binary': {
      if (ast.op === '||') {
        const l = evalCompiledAst(ast.left, ctx)
        return Boolean(l) ? l : evalCompiledAst(ast.right, ctx)
      }
      if (ast.op === '&&') {
        const l = evalCompiledAst(ast.left, ctx)
        return Boolean(l) ? evalCompiledAst(ast.right, ctx) : l
      }
      if (ast.op === '??') {
        const l = evalCompiledAst(ast.left, ctx)
        return l !== null && l !== undefined ? l : evalCompiledAst(ast.right, ctx)
      }
      const left = evalCompiledAst(ast.left, ctx)
      const right = evalCompiledAst(ast.right, ctx)
      switch (ast.op) {
        case '===':
        case '==':
          return left === right
        case '!==':
        case '!=':
          return left !== right
        case '>=':
          return Number(left) >= Number(right)
        case '<=':
          return Number(left) <= Number(right)
        case '>':
          return Number(left) > Number(right)
        case '<':
          return Number(left) < Number(right)
        case '+':
          if (typeof left === 'string' || typeof right === 'string') {
            return `${left ?? ''}${right ?? ''}`
          }
          return Number(left) + Number(right)
        case '-':
          return Number(left) - Number(right)
        case '*':
          return Number(left) * Number(right)
        case '/':
          return Number(left) / Number(right)
        case '%':
          return Number(left) % Number(right)
        default:
          return undefined
      }
    }
    case 'conditional':
      return evalCompiledAst(ast.test, ctx)
        ? evalCompiledAst(ast.consequent, ctx)
        : evalCompiledAst(ast.alternate, ctx)
  }
}

export function evalFormaExpr(expr: string, ctx: FormaRenderContext): unknown {
  return evalCompiledAst(compileFormaExpression(expr), ctx)
}

function evalProp(el: FormaElementNode, name: string, ctx: FormaRenderContext): unknown {
  if (name in el.bindings) {
    const path = el.bindings[name].split('.')
    const rootVal = readStateField(ctx, path[0])
    return path.length === 1 ? rootVal : getByPathParts(rootVal, path.slice(1))
  }
  if (!(name in el.props)) return undefined
  return evalCompiledAst(compileFormaExpression(el.props[name]), ctx)
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
    const compiledArgs = act.args.map(compileFormaExpression)
    attrs.onClick = (ev: Event) => {
      const evaluatedArgs = compiledArgs.map(a => evalCompiledAst(a, ctx))
      ctx.onAction?.(act.name, evaluatedArgs, ev)
    }
  }
  if (el.actions.keydown) {
    const act = el.actions.keydown
    const compiledArgs = act.args.map(compileFormaExpression)
    attrs.onKeydown = (ev: Event) => {
      const evaluatedArgs = compiledArgs.map(a => evalCompiledAst(a, ctx))
      ctx.onAction?.(act.name, evaluatedArgs, ev)
    }
  }
  return attrs
}

function renderElement(el: FormaElementNode, ctx: FormaRenderContext, key?: unknown): VNodeChild {
  const children = renderNodes(el.children, ctx)
  const textProp = evalProp(el, 'text', ctx) ?? evalProp(el, 'label', ctx)
  const content: VNodeChild[] =
    textProp !== undefined && textProp !== null && textProp !== ''
      ? [String(textProp), ...children]
      : children
  const withKey = (attrs: Record<string, unknown>) =>
    key !== undefined ? { ...attrs, key } : attrs

  switch (el.type) {
    case 'Slot': {
      const slotName = String(evalProp(el, 'name', ctx) ?? 'default')
      return ctx.slots?.[slotName]?.() ?? children
    }
    case 'Nav':
      return h('nav', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-nav')), content)
    case 'Footer':
      return h('footer', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-footer')), content)
    case 'Section':
      return h('section', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-section')), content)
    case 'Row':
      return h('div', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-row')), content)
    case 'Column':
      return h('div', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-col')), content)
    case 'Stack':
    case 'Surface':
    case 'Frame':
      return h('div', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-surface')), content)
    case 'Spacer':
      return h('span', withKey(buildAriaAndDomAttrs(el, ctx, 'toolbar-spacer')), [])
    case 'Divider':
      return h('span', withKey(buildAriaAndDomAttrs(el, ctx, 'vt-sep')), [])
    case 'Text':
    case 'Span':
      return h('span', withKey(buildAriaAndDomAttrs(el, ctx)), content)
    case 'Strong':
      return h('strong', withKey(buildAriaAndDomAttrs(el, ctx)), content)
    case 'Paragraph':
      return h('p', withKey(buildAriaAndDomAttrs(el, ctx)), content)
    case 'Keycap':
      return h('kbd', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-kbd')), content)
    case 'Badge':
      return h('span', withKey(buildAriaAndDomAttrs(el, ctx, 'forma-badge')), content)
    case 'StatusDot':
      return h('i', withKey(buildAriaAndDomAttrs(el, ctx, 'status-dot')), [])
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
        withKey({
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
        }),
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
      const attrs = withKey(buildAriaAndDomAttrs(el, ctx, defaultCls))
      attrs.type = String(evalProp(el, 'buttonType', ctx) ?? 'button')
      return h('button', attrs, content)
    }
    case 'Checkbox': {
      const checked = Boolean(evalProp(el, 'checked', ctx))
      const bindPath = el.bindings.checked
      const labelText = evalProp(el, 'label', ctx)
      return h('label', withKey(buildAriaAndDomAttrs(el, ctx, 'auto-check forma-checkbox')), [
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
      const attrs = withKey(buildAriaAndDomAttrs(el, ctx, 'forma-input'))
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
      return h('progress', withKey({ ...buildAriaAndDomAttrs(el, ctx), max, value }))
    }
    default:
      return h('div', withKey(buildAriaAndDomAttrs(el, ctx)), content)
  }
}

export function renderNodes(nodes: FormaAstNode[], ctx: FormaRenderContext): VNodeChild[] {
  const out: VNodeChild[] = []
  for (const node of nodes) {
    if (node.kind === 'let') {
      ctx.locals[node.name] = evalCompiledAst(compileFormaExpression(node.expr), ctx)
      continue
    }
    if (node.kind === 'if') {
      const cond = Boolean(evalCompiledAst(compileFormaExpression(node.condition), ctx))
      const branch = cond ? node.thenBranch : node.elseBranch
      out.push(...renderNodes(branch, ctx))
      continue
    }
    if (node.kind === 'for') {
      const rawList = evalCompiledAst(compileFormaExpression(node.listExpr), ctx)
      const list = Array.isArray(rawList) ? rawList : []
      const keyAst = node.keyExpr ? compileFormaExpression(node.keyExpr) : null
      list.forEach((item, index) => {
        const childCtx: FormaRenderContext = {
          ...ctx,
          locals: {
            ...ctx.locals,
            [node.itemVar]: item,
            ...(node.indexVar ? { [node.indexVar]: index } : {}),
          },
        }
        const itemKey = keyAst ? evalCompiledAst(keyAst, childCtx) : index
        for (const bodyNode of node.body) {
          if (bodyNode.kind === 'element') {
            out.push(renderElement(bodyNode, childCtx, itemKey))
          } else {
            out.push(...renderNodes([bodyNode], childCtx))
          }
        }
      })
      continue
    }
    if (node.kind === 'element') {
      out.push(renderElement(node, ctx))
    }
  }
  return out
}

export function syncFormaSignals(
  signals: Map<string, Signal<unknown>>,
  nextState: Record<string, unknown>,
): void {
  batch(() => {
    for (const key of Object.keys(nextState)) {
      const nextVal = nextState[key]
      let sig = signals.get(key)
      if (!sig) {
        sig = signal(nextVal)
        signals.set(key, sig)
      } else {
        sig.set(nextVal)
      }
    }
  })
}

export function renderFormaAst(
  ast: FormaComponentAst,
  state: Record<string, unknown>,
  locale: string,
  options: {
    signals?: Map<string, Signal<unknown>>
    slots?: Slots
    onAction?: (name: string, args: unknown[], event?: Event) => void
    onBinding?: (path: string, value: unknown) => void
  } = {},
): VNodeChild[] {
  const ctx: FormaRenderContext = {
    state,
    signals: options.signals,
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
