/** Product workspace: Code · Solid · Mesh. */

export type WorkspaceMode = 'code' | 'solid' | 'mesh'

export const WORKSPACE_MODES: readonly WorkspaceMode[] = Object.freeze(['code', 'solid', 'mesh'])

export function workspaceModeLabel(mode: WorkspaceMode, locale: 'ru' | 'en' = 'en'): string {
  if (locale === 'ru') {
    return mode === 'code' ? 'Code' : mode === 'solid' ? 'Solid' : 'Mesh'
  }
  return mode === 'code' ? 'Code' : mode === 'solid' ? 'Solid' : 'Mesh'
}

export function workspaceModeHint(mode: WorkspaceMode, locale: 'ru' | 'en' = 'en'): string {
  if (locale === 'ru') {
    if (mode === 'code') return 'Параметрическая программа (.scad)'
    if (mode === 'solid') return 'CAD-лепка: эскиз, push/pull, fillet (Plasticity-like)'
    return 'Полигоны: вершины/рёбра/грани (Blender-like)'
  }
  if (mode === 'code') return 'Parametric program (.scad)'
  if (mode === 'solid') return 'CAD sculpt: sketch, push/pull, fillet (Plasticity-like)'
  return 'Polygons: vertices/edges/faces (Blender-like)'
}

export function isWorkspaceMode(value: unknown): value is WorkspaceMode {
  return value === 'code' || value === 'solid' || value === 'mesh'
}
