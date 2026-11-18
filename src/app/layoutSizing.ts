export interface EditorWidthBounds {
  min: number
  max: number
}

export function editorWidthBounds(mainWidth: number): EditorWidthBounds {
  const min = 300
  return { min, max: Math.max(min, Math.min(820, mainWidth - 320)) }
}

export function clampEditorWidth(width: number, mainWidth: number): number {
  const bounds = editorWidthBounds(mainWidth)
  return Math.max(bounds.min, Math.min(bounds.max, width))
}
