import { describe, expect, it } from 'vitest'
import { MAX_WORKSPACE_SOURCE_LENGTH } from '../src/services/workspaceDocument'
import { encodeWorkspaceShare, readWorkspaceShareHash } from '../src/services/workspaceShare'

describe('workspace share hash', () => {
  it('round-trips Unicode OpenSCAD source through URL-safe base64', () => {
    const source = '// деталь 😀\ncube(2);'
    const encoded = encodeWorkspaceShare(source)

    expect(encoded).not.toMatch(/[+/=]/)
    expect(readWorkspaceShareHash(`#code=${encoded}`)).toBe(source)
  })

  it('rejects malformed and decoded-over-budget shares', () => {
    expect(readWorkspaceShareHash('#other=value')).toBeNull()
    expect(readWorkspaceShareHash('#code=%%%')).toBeNull()
    const oversized = 'x'.repeat(MAX_WORKSPACE_SOURCE_LENGTH + 1)
    expect(readWorkspaceShareHash(`#code=${encodeWorkspaceShare(oversized)}`)).toBeNull()
  })

  it('rejects invalid UTF-8 and non-canonical base64', () => {
    expect(readWorkspaceShareHash('#code=_w')).toBeNull()
    expect(readWorkspaceShareHash('#code=YQ=')).toBeNull()
    expect(readWorkspaceShareHash('#code=YR')).toBeNull()
  })
})
