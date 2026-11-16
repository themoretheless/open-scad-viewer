/**
 * Dependency-free accessibility audit of a rendered DOM subtree (a subset of
 * axe-core rules): interactive controls need an accessible name, images need
 * alternative text, `aria-labelledby`/`label[for]` must resolve and element ids
 * must be unique. Closed `<details>` bodies and subtrees with `hidden`, `inert` or `aria-hidden="true"` are
 * skipped. Exposed in the browser through `window.__osvTest.accessibility()`.
 */
export interface AuditNode {
  readonly tagName: string
  readonly textContent: string | null
  readonly children: ArrayLike<AuditNode>
  getAttribute(name: string): string | null
}

export type AccessibilityRule = 'control-name' | 'image-alt' | 'broken-reference' | 'duplicate-id'

export interface AccessibilityIssue {
  rule: AccessibilityRule
  /** Short selector-like description: `button.class#id`. */
  element: string
}

const ROLE_CONTROLS = new Set([
  'button',
  'checkbox',
  'combobox',
  'link',
  'menuitem',
  'menuitemcheckbox',
  'menuitemradio',
  'option',
  'radio',
  'slider',
  'spinbutton',
  'switch',
  'tab',
  'textbox',
])
/** Controls whose own text content names them. */
const NAME_FROM_CONTENT = new Set(['button', 'a', 'summary', 'option'])
const FORM_FIELDS = new Set(['input', 'select', 'textarea'])

function describe(node: AuditNode): string {
  const tag = node.tagName.toLowerCase()
  const id = node.getAttribute('id')
  const cls = (node.getAttribute('class') ?? '').trim().split(/\s+/).filter(Boolean)[0]
  return `${tag}${cls ? `.${cls}` : ''}${id ? `#${id}` : ''}`
}

function text(node: AuditNode): string {
  return (node.textContent ?? '').trim()
}

function skipped(node: AuditNode): boolean {
  return (
    node.getAttribute('hidden') !== null ||
    node.getAttribute('inert') !== null ||
    node.getAttribute('aria-hidden') === 'true'
  )
}

function isControl(node: AuditNode, tag: string): boolean {
  const role = node.getAttribute('role')
  if (role === 'presentation' || role === 'none') return false
  if (role && ROLE_CONTROLS.has(role)) return true
  if (tag === 'input') return (node.getAttribute('type') ?? 'text').toLowerCase() !== 'hidden'
  if (tag === 'a') return node.getAttribute('href') !== null
  return tag === 'button' || tag === 'select' || tag === 'textarea' || tag === 'summary'
}

export function auditAccessibility(root: AuditNode): AccessibilityIssue[] {
  const ids = new Map<string, AuditNode[]>()
  const labelFor = new Map<string, string>()
  const visit: AuditNode[] = []
  const walk = (node: AuditNode) => {
    if (skipped(node)) return
    const tag = node.tagName.toLowerCase()
    const id = node.getAttribute('id')
    if (id) ids.set(id, [...(ids.get(id) ?? []), node])
    if (tag === 'label') {
      const target = node.getAttribute('for')
      if (target) labelFor.set(target, `${labelFor.get(target) ?? ''} ${text(node)}`.trim())
    }
    visit.push(node)
    // A closed <details> renders only its <summary>.
    const closed = tag === 'details' && node.getAttribute('open') === null
    for (let i = 0; i < node.children.length; i++) {
      const child = node.children[i]
      if (!closed || child.tagName.toLowerCase() === 'summary') walk(child)
    }
  }
  walk(root)
  const issues: AccessibilityIssue[] = []
  const resolve = (refs: string) =>
    refs
      .split(/\s+/)
      .filter(Boolean)
      .map(ref => {
        const target = ids.get(ref)?.[0]
        return target ? text(target) : null
      })
  const labelText = new Map<AuditNode, string>()
  const collectLabels = (node: AuditNode, label: string | null) => {
    if (skipped(node)) return
    const own = node.tagName.toLowerCase() === 'label' ? text(node) : label
    if (own !== null) labelText.set(node, own)
    for (let i = 0; i < node.children.length; i++) collectLabels(node.children[i], own)
  }
  collectLabels(root, null)
  for (const node of visit) {
    const tag = node.tagName.toLowerCase()
    const labelledBy = node.getAttribute('aria-labelledby')
    if (labelledBy) {
      const parts = resolve(labelledBy)
      if (parts.some(p => p === null)) issues.push({ rule: 'broken-reference', element: describe(node) })
    }
    if (tag === 'img') {
      const role = node.getAttribute('role')
      if (
        node.getAttribute('alt') === null &&
        role !== 'presentation' &&
        role !== 'none' &&
        !node.getAttribute('aria-label')
      )
        issues.push({ rule: 'image-alt', element: describe(node) })
      continue
    }
    if (!isControl(node, tag)) continue
    const id = node.getAttribute('id')
    const role = node.getAttribute('role')
    const type = (node.getAttribute('type') ?? '').toLowerCase()
    const names = [
      node.getAttribute('aria-label')?.trim(),
      labelledBy ? resolve(labelledBy).filter(Boolean).join(' ') : '',
      id ? labelFor.get(id) : '',
      FORM_FIELDS.has(tag) ? labelText.get(node) : '',
      NAME_FROM_CONTENT.has(tag) || (role && ROLE_CONTROLS.has(role) && !FORM_FIELDS.has(tag)) ? text(node) : '',
      tag === 'input' && ['submit', 'button', 'reset'].includes(type) ? node.getAttribute('value') : '',
      tag === 'input' && type === 'image' ? node.getAttribute('alt') : '',
      node.getAttribute('title'),
      FORM_FIELDS.has(tag) ? node.getAttribute('placeholder') : '',
    ]
    if (!names.some(n => n && n.trim())) issues.push({ rule: 'control-name', element: describe(node) })
  }
  for (const [target] of labelFor) {
    if (!ids.has(target)) issues.push({ rule: 'broken-reference', element: `label[for=${target}]` })
  }
  for (const [, nodes] of ids) {
    if (nodes.length > 1) issues.push({ rule: 'duplicate-id', element: describe(nodes[1]) })
  }
  return issues
}
