import { describe, expect, it } from 'vitest'
import { auditAccessibility, type AuditNode } from '../src/dev/accessibilityAudit'

type Child = AuditNode | string
function el(tagName: string, attrs: Record<string, string> = {}, ...kids: Child[]): AuditNode {
  const children = kids.filter((k): k is AuditNode => typeof k !== 'string')
  const own = kids.filter(k => typeof k === 'string').join('')
  return {
    tagName: tagName.toUpperCase(),
    children,
    get textContent() {
      return own + children.map(c => c.textContent ?? '').join('')
    },
    getAttribute: name => attrs[name] ?? null,
  }
}

describe('auditAccessibility', () => {
  it('accepts controls named by text, aria-label, labels, title and labelledby', () => {
    const root = el(
      'div',
      {},
      el('button', {}, 'Save'),
      el('button', { 'aria-label': 'Close' }, el('svg')),
      el('label', {}, 'Width', el('input', { type: 'number' })),
      el('label', { for: 'h' }, 'Height'),
      el('input', { id: 'h' }),
      el('select', { title: 'Units' }),
      el('span', { id: 'cap' }, 'Depth'),
      el('input', { 'aria-labelledby': 'cap' }),
      el('div', { role: 'tab' }, 'Views'),
      el('img', { alt: '' }),
      el('input', { type: 'hidden' }),
      el('a', {}, 'not a link'),
    )
    expect(auditAccessibility(root)).toEqual([])
  })

  it('reports unnamed controls, missing alt, broken references and duplicate ids', () => {
    const root = el(
      'div',
      {},
      el('button', { class: 'icon tool' }, el('svg')),
      el('input', { id: 'x' }),
      el('div', { role: 'switch' }),
      el('img', { src: 'a.png' }),
      el('input', { 'aria-labelledby': 'missing' }),
      el('label', { for: 'nowhere' }, 'Ghost'),
      el('span', { id: 'x' }),
    )
    expect(auditAccessibility(root)).toEqual([
      { rule: 'control-name', element: 'button.icon' },
      { rule: 'control-name', element: 'input#x' },
      { rule: 'control-name', element: 'div' },
      { rule: 'image-alt', element: 'img' },
      { rule: 'broken-reference', element: 'input' },
      { rule: 'control-name', element: 'input' },
      { rule: 'broken-reference', element: 'label[for=nowhere]' },
      { rule: 'duplicate-id', element: 'span#x' },
    ])
  })

  it('skips hidden subtrees and closed details bodies', () => {
    const root = el(
      'div',
      {},
      el('div', { hidden: '' }, el('button')),
      el('div', { 'aria-hidden': 'true' }, el('button')),
      el('details', {}, el('summary', {}, 'More'), el('button')),
      el('details', { open: '' }, el('summary', {}, 'Open'), el('button')),
    )
    expect(auditAccessibility(root)).toEqual([{ rule: 'control-name', element: 'button' }])
  })
})
