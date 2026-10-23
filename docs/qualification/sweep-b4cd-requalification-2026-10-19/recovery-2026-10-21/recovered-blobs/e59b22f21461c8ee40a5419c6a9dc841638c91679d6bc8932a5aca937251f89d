/** Product workspace: Solid · Mesh. Source is an editor drawer inside both. */

export type WorkspaceMode = 'solid' | 'mesh'

export const WORKSPACE_MODES: readonly WorkspaceMode[] = Object.freeze(['solid', 'mesh'])

export function workspaceModeLabel(mode: WorkspaceMode, _locale: 'ru' | 'en' = 'en'): string {
  return mode === 'solid' ? 'Solid' : 'Mesh'
}

export function workspaceModeHint(mode: WorkspaceMode, locale: 'ru' | 'en' = 'en'): string {
  if (locale === 'ru') {
    return mode === 'solid'
      ? 'Точные тела: NURBS и B-rep (Plasticity-like)'
      : 'Полигоны: вершины/рёбра/грани (Blender-like)'
  }
  return mode === 'solid'
    ? 'Exact solids: NURBS and B-rep (Plasticity-like)'
    : 'Polygons: vertices/edges/faces (Blender-like)'
}

export function isWorkspaceMode(value: unknown): value is WorkspaceMode {
  return value === 'solid' || value === 'mesh'
}
