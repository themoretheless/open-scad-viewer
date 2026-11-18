import { computed, ref, watch, type ComputedRef, type Ref } from 'svet'
import {
  addProjectFile,
  parseLibraryPathsInput,
  projectBuildSources,
  removeProjectFile,
  renameProjectFile,
  updateProjectFileSource,
  type ProjectFileEdit,
  type ProjectFileEditError,
} from '../app/projectFiles'
import {
  updateWorkspaceDocument,
  type WorkspaceDocumentSnapshot,
  type WorkspaceProjectFile,
} from '../app/workspaceDocument'
import type { GeometryProjectSources } from '../kernel/geometryBuildEngine'

export interface UseProjectFilesOptions {
  workspaceDocument: Ref<WorkspaceDocumentSnapshot>
  /** Persists a metadata edit (same path as parameter presets). */
  persist: () => void
  /** Called after any edit that changes the build context. */
  onProjectChanged: () => void
}

export interface UseProjectFilesReturn {
  /** Active editor tab: `null` is the main document, otherwise an auxiliary path. */
  activePath: Ref<string | null>
  files: ComputedRef<readonly WorkspaceProjectFile[]>
  libraryPaths: ComputedRef<readonly string[]>
  project: ComputedRef<GeometryProjectSources | undefined>
  /** Stable content key; changes exactly when the build context changes. */
  projectKey: ComputedRef<string>
  error: Ref<ProjectFileEditError | null>
  activeSource: ComputedRef<string>
  addFile: (path: string) => boolean
  renameFile: (from: string, to: string) => boolean
  removeFile: (path: string) => boolean
  updateActiveSource: (source: string) => boolean
  setLibraryPaths: (text: string) => boolean
}

export function useProjectFiles(options: UseProjectFilesOptions): UseProjectFilesReturn {
  const { workspaceDocument } = options
  const activePath = ref<string | null>(null)
  const error = ref<ProjectFileEditError | null>(null)
  const files = computed(() => workspaceDocument.value.projectFiles)
  const libraryPaths = computed(() => workspaceDocument.value.libraryPaths)
  const project = computed(() => projectBuildSources(workspaceDocument.value))
  const projectKey = computed(() => JSON.stringify(project.value ?? null))
  const activeSource = computed(() => files.value.find(file => file.path === activePath.value)?.source ?? '')

  // A replaced or restored document may no longer contain the active file.
  watch(files, (next: readonly WorkspaceProjectFile[]) => {
    if (activePath.value !== null && !next.some(file => file.path === activePath.value)) activePath.value = null
  })

  function commit(
    edit: ProjectFileEdit<readonly unknown[]>,
    update: (value: never) => Parameters<typeof updateWorkspaceDocument>[1],
  ): boolean {
    if (!edit.ok) {
      error.value = edit.error
      return false
    }
    const previous = workspaceDocument.value
    try {
      workspaceDocument.value = updateWorkspaceDocument(previous, update(edit.value as never))
    } catch {
      error.value = 'projectFileLimit'
      return false
    }
    error.value = null
    if (workspaceDocument.value !== previous) {
      options.persist()
      options.onProjectChanged()
    }
    return true
  }

  const setFiles = (projectFiles: readonly WorkspaceProjectFile[]) => ({ projectFiles })
  const mainFileName = () => workspaceDocument.value.fileName

  return {
    activePath,
    files,
    libraryPaths,
    project,
    projectKey,
    error,
    activeSource,
    addFile(path) {
      const edit = addProjectFile(files.value, mainFileName(), path)
      const ok = commit(edit, setFiles)
      if (ok && edit.ok) activePath.value = edit.value[edit.value.length - 1]!.path
      return ok
    },
    renameFile(from, to) {
      const edit = renameProjectFile(files.value, mainFileName(), from, to)
      const index = files.value.findIndex(file => file.path === from)
      const wasActive = activePath.value === from
      const ok = commit(edit, setFiles)
      if (ok && edit.ok && wasActive) activePath.value = edit.value[index]!.path
      return ok
    },
    removeFile(path) {
      const ok = commit(removeProjectFile(files.value, path), setFiles)
      if (ok && activePath.value === path) activePath.value = null
      return ok
    },
    updateActiveSource(source) {
      if (activePath.value === null) return false
      return commit(updateProjectFileSource(files.value, activePath.value, source), setFiles)
    },
    setLibraryPaths(text) {
      return commit(parseLibraryPathsInput(text), (libraryPaths: readonly string[]) => ({ libraryPaths }))
    },
  }
}
