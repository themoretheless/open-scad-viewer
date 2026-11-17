import type { GeometryProjectSources } from '../kernel/geometryBuildEngine'
import { normalizeOpenScadProjectPath } from '../language/openScadProject'
import {
  MAX_WORKSPACE_LIBRARY_PATHS,
  MAX_WORKSPACE_PROJECT_FILES,
  type WorkspaceDocumentSnapshot,
  type WorkspaceProjectFile,
} from './workspaceDocument'

/** App-string keys for project file edits the UI rejects. */
export type ProjectFileEditError =
  | 'projectFileInvalidPath'
  | 'projectFileExists'
  | 'projectFileLimit'
  | 'projectFileMissing'
  | 'projectLibraryPathInvalid'
  | 'projectLibraryPathLimit'

export type ProjectFileEdit<T> =
  { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: ProjectFileEditError }

const fail = (error: ProjectFileEditError) => ({ ok: false, error }) as const

function canonicalPath(value: string): string | null {
  try {
    return normalizeOpenScadProjectPath(value.trim())
  } catch {
    return null
  }
}

function takenPath(files: readonly WorkspaceProjectFile[], mainFileName: string, path: string, except?: string) {
  return path === mainFileName || files.some(file => file.path === path && file.path !== except)
}

export function addProjectFile(
  files: readonly WorkspaceProjectFile[],
  mainFileName: string,
  rawPath: string,
  source = '',
): ProjectFileEdit<readonly WorkspaceProjectFile[]> {
  const path = canonicalPath(rawPath)
  if (path === null) return fail('projectFileInvalidPath')
  if (takenPath(files, mainFileName, path)) return fail('projectFileExists')
  if (files.length >= MAX_WORKSPACE_PROJECT_FILES) return fail('projectFileLimit')
  return { ok: true, value: [...files, { path, source }] }
}

export function renameProjectFile(
  files: readonly WorkspaceProjectFile[],
  mainFileName: string,
  from: string,
  rawPath: string,
): ProjectFileEdit<readonly WorkspaceProjectFile[]> {
  if (!files.some(file => file.path === from)) return fail('projectFileMissing')
  const path = canonicalPath(rawPath)
  if (path === null) return fail('projectFileInvalidPath')
  if (takenPath(files, mainFileName, path, from)) return fail('projectFileExists')
  return { ok: true, value: files.map(file => (file.path === from ? { path, source: file.source } : file)) }
}

export function removeProjectFile(
  files: readonly WorkspaceProjectFile[],
  path: string,
): ProjectFileEdit<readonly WorkspaceProjectFile[]> {
  if (!files.some(file => file.path === path)) return fail('projectFileMissing')
  return { ok: true, value: files.filter(file => file.path !== path) }
}

export function updateProjectFileSource(
  files: readonly WorkspaceProjectFile[],
  path: string,
  source: string,
): ProjectFileEdit<readonly WorkspaceProjectFile[]> {
  if (!files.some(file => file.path === path)) return fail('projectFileMissing')
  return { ok: true, value: files.map(file => (file.path === path ? { path, source } : file)) }
}

/** Parses one library directory per line (or comma separated), in search order. */
export function parseLibraryPathsInput(text: string): ProjectFileEdit<readonly string[]> {
  const paths: string[] = []
  for (const entry of text.split(/[\n,]/u)) {
    const trimmed = entry.trim().replace(/\/+$/u, '')
    if (!trimmed) continue
    const path = canonicalPath(trimmed)
    if (path === null) return fail('projectLibraryPathInvalid')
    if (!paths.includes(path)) paths.push(path)
  }
  if (paths.length > MAX_WORKSPACE_LIBRARY_PATHS) return fail('projectLibraryPathLimit')
  return { ok: true, value: paths }
}

/**
 * Build context for the worker, or `undefined` for a plain single-file
 * document so ordinary builds keep their established request shape.
 */
export function projectBuildSources(document: WorkspaceDocumentSnapshot): GeometryProjectSources | undefined {
  if (document.projectFiles.length === 0 && document.libraryPaths.length === 0) return undefined
  return {
    entrypoint: document.fileName,
    files: document.projectFiles.map(file => ({ path: file.path, source: file.source })),
    libraryPaths: [...document.libraryPaths],
  }
}
