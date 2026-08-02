import { describe, expect, it } from 'vitest'
import { clampEditorWidth, editorWidthBounds } from '../src/services/layoutSizing'

describe('editor layout sizing', () => {
  it('reserves at least 320px for the viewport on narrow desktop layouts', () => {
    expect(editorWidthBounds(900)).toEqual({ min: 300, max: 580 })
    expect(clampEditorWidth(820, 900)).toBe(580)
  })

  it('caps wide layouts and remains defined below the desktop breakpoint', () => {
    expect(editorWidthBounds(1600).max).toBe(820)
    expect(editorWidthBounds(600)).toEqual({ min: 300, max: 300 })
  })
})
