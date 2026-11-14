import { computed, ref, watch, type Ref } from 'svet'
import type { DirectBody, DirectDocument } from '../app/directModeling'
import type { WorkspaceMode } from '../app/workspaceModes'
import { storageSet } from '../core/safeStorage'
import type { MeshData } from '../core/mesh'
import { stringifyMeshJson } from '../exchange/meshJson'
import { emptyMeshDocument, type MeshWorkspaceDocument } from '../kernel/meshEditing'
import {
  meshDataToEditablePolygon,
  meshDocumentToSolidDocument,
  polygonToMeshObject,
  sceneMeshesToSolidDocument,
} from '../kernel/solidBridge'
import { SOURCE_FILE_EXTENSION } from '../language/rushFrontendDetect'

export interface GroupSourceRequest {
  name: string
  source: string
  replaces: string | null
}

export interface UseWorkspaceModesOptions {
  code: Ref<string>
  fileName: Ref<string>
  lang: Ref<'ru' | 'en'>
  error: Ref<string>
  /** Read lazily: the scene owner is created after the workspace modes. */
  sceneMeshes: () => readonly MeshData[]
}

const SOLID_DRAFT_KEY = 'scad-solid-modeler-v1'
const message = (caught: unknown) => (caught instanceof Error ? caught.message : String(caught))

function sceneMeshesToMeshDocument(meshes: readonly MeshData[], lang: 'ru' | 'en'): MeshWorkspaceDocument {
  const doc = emptyMeshDocument()
  const prefix = lang === 'ru' ? 'Объект' : 'Object'
  meshes.forEach((mesh, index) => {
    const polygon = meshDataToEditablePolygon(mesh)
    if (polygon)
      doc.objects.push(
        polygonToMeshObject(polygon, `${prefix} ${index + 1}`, `scene-${index + 1}-${Date.now().toString(36)}`),
      )
  })
  return doc
}

/**
 * Solid and Mesh workspaces, the Source drawer over them, and the hand-offs between
 * them (code to Solid or Mesh, Mesh to Solid, one group's source built as exact solids).
 */
export function useWorkspaceModes(options: UseWorkspaceModesOptions) {
  const { code, fileName, lang, error } = options
  const directModelerOpen = ref(false)
  const meshModelerOpen = ref(false)
  const solidSeedDocument = ref<DirectDocument | null>(null)
  const meshSeedDocument = ref<MeshWorkspaceDocument | null>(null)
  /** Source is no longer a workspace of its own; it opens as a drawer over either one. */
  const editorOpen = ref(false)
  const workspaceMode = computed<WorkspaceMode>(() => (meshModelerOpen.value ? 'mesh' : 'solid'))
  const solidBuilding = ref(false)
  let solidBuildAbort: AbortController | null = null
  const solidAppendBodies = ref<{ bodies: DirectBody[]; token: number; group?: GroupSourceRequest } | null>(null)
  /** Set while the left panel edits one scene group's source instead of the document. */
  const groupEdit = ref<GroupSourceRequest | null>(null)

  function cancelSolidBuild() {
    solidBuildAbort?.abort()
  }
  // A completed worker must not publish a superseded source snapshot.
  watch([code, fileName, () => groupEdit.value?.source], cancelSolidBuild, { flush: 'sync' })

  function showSolid(seed?: DirectDocument | null) {
    if (seed !== undefined) solidSeedDocument.value = seed
    meshModelerOpen.value = false
    directModelerOpen.value = true
  }
  function showMesh() {
    directModelerOpen.value = false
    meshModelerOpen.value = true
  }

  function openGroupEditor(request: GroupSourceRequest) {
    cancelSolidBuild()
    groupEdit.value = { ...request }
    editorOpen.value = true
  }

  function openWorkspaceMode(mode: WorkspaceMode) {
    if (mode === 'solid') showSolid()
    else showMesh()
  }

  /** Builds one group's own source and hands the bodies back tagged with its name. */
  async function buildSolidGroup(request: GroupSourceRequest) {
    if (solidBuilding.value) return false
    solidBuilding.value = true
    const abort = new AbortController()
    solidBuildAbort = abort
    error.value = ''
    try {
      const { buildExactSolidsInWorker } = await import('../kernel/solid/exactSolidClient')
      const built = await buildExactSolidsInWorker(request.source, abort.signal)
      if (abort.signal.aborted) return false
      if (built.length === 0) {
        error.value =
          lang.value === 'ru' ? 'Код группы не описывает ни одного тела.' : 'The group source describes no solids.'
        return false
      }
      solidAppendBodies.value = {
        bodies: built.map(body => ({ ...body, group: request.name })),
        group: { name: request.name, source: request.source, replaces: request.replaces },
        token: (solidAppendBodies.value?.token ?? 0) + 1,
      }
      // Do not discard edits made while the requested snapshot was being built.
      if (groupEdit.value?.name === request.name && groupEdit.value.source === request.source) groupEdit.value = null
      return true
    } catch (caught) {
      if (!abort.signal.aborted) error.value = message(caught)
      return false
    } finally {
      if (solidBuildAbort === abort) solidBuildAbort = null
      solidBuilding.value = false
    }
  }

  /**
   * The editor's document becomes one group in the Solid scene, named after the file
   * and carrying its source, so the dock lists it as code that can be edited and
   * rebuilt; building again replaces the group instead of stacking copies.
   */
  async function buildSolidFromSource() {
    const name = fileName.value.replace(SOURCE_FILE_EXTENSION, '') || 'source'
    if (!(await buildSolidGroup({ name, source: code.value, replaces: name }))) return
    showSolid()
    editorOpen.value = false
  }

  function bringCodeToMesh() {
    if (!options.sceneMeshes().length) return
    try {
      meshSeedDocument.value = sceneMeshesToMeshDocument(options.sceneMeshes(), lang.value)
      solidSeedDocument.value = null
      showMesh()
    } catch (caught) {
      error.value = message(caught)
    }
  }

  function bringCodeToSolid() {
    const meshes = options.sceneMeshes()
    if (!meshes.length) return
    try {
      showSolid(sceneMeshesToSolidDocument([...meshes], lang.value === 'ru' ? 'Тело' : 'Body'))
    } catch (caught) {
      error.value = message(caught)
    }
  }

  function openMeshFromSolid() {
    solidSeedDocument.value = null
    meshSeedDocument.value = null
    showMesh()
  }

  function continueInSolid(document: DirectDocument) {
    storageSet(SOLID_DRAFT_KEY, stringifyMeshJson(document))
    showSolid(document)
  }

  function meshToSolid(doc: MeshWorkspaceDocument) {
    continueInSolid(meshDocumentToSolidDocument(doc))
  }

  // Solid is the resting workspace: with no Code tab there is nothing else to show.
  directModelerOpen.value = true

  return {
    directModelerOpen,
    meshModelerOpen,
    solidSeedDocument,
    meshSeedDocument,
    editorOpen,
    workspaceMode,
    solidBuilding,
    solidAppendBodies,
    groupEdit,
    cancelSolidBuild,
    openGroupEditor,
    openWorkspaceMode,
    buildSolidGroup,
    buildSolidFromSource,
    bringCodeToMesh,
    bringCodeToSolid,
    openMeshFromSolid,
    meshToSolid,
    continueMainEditInSolid: continueInSolid,
  }
}
