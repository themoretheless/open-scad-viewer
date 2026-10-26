import { computed, onUnmounted, ref, shallowRef, type ComputedRef, type Ref, type ShallowRef } from 'vue'
import type { DirectBody, DirectDocument, DirectSketch, Point2 } from '../../services/directModeling'
import type { MainSolidResults } from '../../services/mainSolidProtocol'
import type { NurbsCurve } from '../../services/nurbsCurve'
import { SolidCurveDisplayQueue } from '../../services/solidCurveDisplayQueue'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { SolidProfileDisplayQueue } from '../../services/solidProfileDisplayQueue'
import { SolidSketchSnapPreparation } from '../../services/solidSketchSnapPreparation'
import { SolidSnapPreparation } from '../../services/solidSnapPreparation'
import { SolidSurfaceDisplayQueue } from '../../services/solidSurfaceDisplayQueue'

type SolidTopology = MainSolidResults['topology']

export interface UseDisplayAndSnapQueuesOptions {
  props: { open: boolean }
  kernelReady: Ref<boolean>
  document: ShallowRef<DirectDocument>
  snapDocument: ShallowRef<DirectDocument>
  selectedBody: ComputedRef<DirectBody | undefined>
  workplaneOutline: Ref<Point2[][]>
  snap: Ref<boolean>
  geometrySnap: Ref<boolean>
  objectInView: (id: string) => boolean
  getAdvancedPreviewDocument: () => DirectDocument | null | undefined
  onSnapCacheInvalidated: () => void
  label: (ru: string, en: string) => string
  error: Ref<string>
}

export function useDisplayAndSnapQueues(options: UseDisplayAndSnapQueuesOptions) {
  const snapPreparationWorker = createSolidPreviewWorker()
  const snapPreparation = new SolidSnapPreparation(snapPreparationWorker)
  const snapPreparationPending = ref(false)
  const snapPreparationVersion = ref(0)
  const snapPreparationErrors = ref<{ id: string; message: string }[]>([])
  let snapPreparationGeneration = 0
  function cancelSnapPreparation() {
    snapPreparationGeneration++
    snapPreparation.cancel()
    snapPreparationPending.value = false
  }
  onUnmounted(() => {
    cancelSnapPreparation()
    snapPreparationWorker.dispose()
    snapPreparation.clear()
  })

  const sketchSnapWorker = createSolidPreviewWorker()
  const sketchSnapPreparation = new SolidSketchSnapPreparation(sketchSnapWorker)
  const sketchSnapPending = ref(false)
  const sketchSnapVersion = ref(0)
  const sketchSnapErrors = ref<{ id: string; message: string }[]>([])
  let sketchSnapGeneration = 0
  function cancelSketchSnaps() {
    sketchSnapGeneration++
    sketchSnapPreparation.cancel()
    sketchSnapPending.value = false
  }
  onUnmounted(() => {
    cancelSketchSnaps()
    sketchSnapWorker.dispose()
    sketchSnapPreparation.clear()
  })

  const topology = shallowRef<SolidTopology>({ faces: [], edges: [] })
  const topologyPending = ref(false)
  const topologyRetryVisible = ref(false)
  const topologyWorker = createSolidPreviewWorker()
  let topologyPick = 0
  let topologyGeneration = 0
  let topologyBody: DirectBody | undefined
  let topologyRequest: Promise<SolidTopology | null> | null = null

  function isTopologyReadyFor(body: DirectBody): boolean {
    return topologyBody === body && !topologyPending.value && topologyRequest !== null
  }

  function nextTopologyPick(): number {
    return ++topologyPick
  }

  function getTopologyPick(): number {
    return topologyPick
  }

  function cancelTopology(clear = false) {
    topologyRetryVisible.value = false
    topologyPick++
    topologyGeneration++
    topologyWorker.cancel()
    if (clear || topologyPending.value) {
      topologyBody = undefined
      topologyRequest = null
      topology.value = { faces: [], edges: [] }
    }
    topologyPending.value = false
  }

  function prepareTopology(body: DirectBody): Promise<SolidTopology | null> {
    if (topologyBody === body && topologyRequest) return topologyRequest
    cancelTopology(true)
    const generation = topologyGeneration
    const source = options.document.value
    topologyBody = body
    topologyPending.value = true
    topologyRequest = topologyWorker
      .run({ kind: 'topology', mesh: body.mesh })
      .then(result => {
        if (generation !== topologyGeneration || options.document.value !== source || options.selectedBody.value !== body) return null
        topology.value = result
        return result
      })
      .catch(() => {
        if (generation === topologyGeneration && options.document.value === source && options.selectedBody.value === body) {
          topologyBody = undefined
          topologyRequest = null
          topologyRetryVisible.value = true
          options.error.value =
            body.name +
            ': ' +
            options.label(
              'Не удалось подготовить выбор граней. Нажмите «Повторить выбор граней».',
              'Could not prepare face selection. Choose Retry face selection.',
            )
        }
        return null
      })
      .finally(() => {
        if (generation === topologyGeneration) topologyPending.value = false
      })
    return topologyRequest
  }
  onUnmounted(() => {
    cancelTopology()
    topologyWorker.dispose()
  })

  const curveDisplayWorker = createSolidPreviewWorker()
  const curveDisplayQueue = new SolidCurveDisplayQueue(curveDisplayWorker)
  const curveDisplayPending = ref(false)
  const curveDisplayVersion = ref(0)
  const curveDisplayErrors = ref<{ id: string; message: string }[]>([])
  let curveDisplayGeneration = 0
  function cancelCurveDisplay() {
    curveDisplayGeneration++
    curveDisplayQueue.cancel()
    curveDisplayPending.value = false
  }
  onUnmounted(() => {
    cancelCurveDisplay()
    curveDisplayWorker.dispose()
    curveDisplayQueue.clear()
  })
  function curvePoints(curve: NurbsCurve): number[][] {
    void curveDisplayVersion.value
    return curveDisplayQueue.get({ id: '', curve }) ?? []
  }

  const profileDisplayWorker = createSolidPreviewWorker()
  const profileDisplayQueue = new SolidProfileDisplayQueue(profileDisplayWorker)
  const profileDisplayPending = ref(false)
  const profileDisplayVersion = ref(0)
  const profileDisplayErrors = ref<{ id: string; message: string }[]>([])
  let profileDisplayGeneration = 0
  function cancelProfileDisplay() {
    profileDisplayGeneration++
    profileDisplayQueue.cancel()
    profileDisplayPending.value = false
  }
  onUnmounted(() => {
    cancelProfileDisplay()
    profileDisplayWorker.dispose()
    profileDisplayQueue.clear()
  })
  function profileLoops(sketch: DirectSketch): Point2[][] {
    void profileDisplayVersion.value
    return sketch.retainedProfile ? profileDisplayQueue.get({ id: sketch.id, profile: sketch.retainedProfile }) ?? [] : [sketch.points]
  }

  const surfaceDisplayWorker = createSolidPreviewWorker()
  const surfaceDisplayQueue = new SolidSurfaceDisplayQueue(surfaceDisplayWorker)
  const surfaceDisplayPending = ref(false)
  const surfaceDisplayVersion = ref(0)
  const surfaceDisplayErrors = ref<{ id: string; message: string }[]>([])
  let surfaceDisplayGeneration = 0
  function cancelSurfaceDisplay() {
    surfaceDisplayGeneration++
    surfaceDisplayQueue.cancel()
    surfaceDisplayPending.value = false
  }
  const surfaceDisplayNeedsRetry = computed(() => {
    void surfaceDisplayVersion.value
    return (
      !surfaceDisplayPending.value &&
      [...(options.document.value.surfaces ?? []), ...(options.getAdvancedPreviewDocument()?.surfaces ?? [])].some(
        item => !surfaceDisplayQueue.get(item),
      )
    )
  })
  onUnmounted(() => {
    cancelSurfaceDisplay()
    surfaceDisplayWorker.dispose()
    surfaceDisplayQueue.clear()
  })

  const supportSnapSketches = computed<DirectSketch[]>(() =>
    options.workplaneOutline.value.map((points, i) => ({
      id: `support-${i}`,
      name: 'Support',
      closed: true,
      points: points.map(p => [p[0], p[1]] as Point2),
    })),
  )
  const snapSketches = computed(() => [
    ...options.snapDocument.value.sketches.filter(s => options.objectInView(s.id)),
    ...supportSnapSketches.value,
  ])
  const sketchSnapsNeedRetry = computed(() => {
    void sketchSnapVersion.value
    return (
      options.props.open &&
      options.snap.value &&
      options.geometrySnap.value &&
      !sketchSnapPending.value &&
      snapSketches.value.some(s => !sketchSnapPreparation.get(s))
    )
  })
  async function refreshSketchSnaps() {
    cancelSketchSnaps()
    sketchSnapErrors.value = []
    options.onSnapCacheInvalidated()
    if (!options.props.open || !options.kernelReady.value || !options.snap.value || !options.geometrySnap.value) return
    const generation = sketchSnapGeneration
    sketchSnapPending.value = snapSketches.value.length > 0
    const result = await sketchSnapPreparation.prepare(snapSketches.value)
    if (generation !== sketchSnapGeneration) return
    sketchSnapVersion.value++
    options.onSnapCacheInvalidated()
    sketchSnapErrors.value = result.errors
    sketchSnapPending.value = false
  }

  const snapBodies = computed(() => options.snapDocument.value.bodies.filter(b => options.objectInView(b.id)))
  const snapPreparationNeedsRetry = computed(() => {
    void snapPreparationVersion.value
    return (
      options.props.open &&
      options.snap.value &&
      options.geometrySnap.value &&
      !snapPreparationPending.value &&
      snapBodies.value.some(b => !snapPreparation.get(b))
    )
  })
  async function refreshSnapPreparation() {
    cancelSnapPreparation()
    options.onSnapCacheInvalidated()
    snapPreparationErrors.value = []
    if (!options.props.open || !options.kernelReady.value || !options.snap.value || !options.geometrySnap.value) return
    const generation = snapPreparationGeneration
    snapPreparationPending.value = snapBodies.value.length > 0
    const result = await snapPreparation.prepare(snapBodies.value)
    if (generation !== snapPreparationGeneration) return
    snapPreparationVersion.value++
    options.onSnapCacheInvalidated()
    snapPreparationErrors.value = result.errors
    snapPreparationPending.value = false
  }

  const displayCurves = computed(() => [
    ...(options.document.value.curves ?? []),
    ...(options.getAdvancedPreviewDocument()?.curves ?? []),
  ])
  const curveDisplayNeedsRetry = computed(() => {
    void curveDisplayVersion.value
    return !curveDisplayPending.value && displayCurves.value.some(item => !curveDisplayQueue.get(item))
  })
  async function refreshCurveDisplay() {
    cancelCurveDisplay()
    curveDisplayErrors.value = []
    if (!options.props.open || !options.kernelReady.value) return
    const generation = curveDisplayGeneration
    curveDisplayPending.value = displayCurves.value.length > 0
    const result = await curveDisplayQueue.prepare(displayCurves.value)
    if (generation !== curveDisplayGeneration) return
    if (result.changed) curveDisplayVersion.value++
    curveDisplayErrors.value = result.errors
    curveDisplayPending.value = false
  }

  const displayProfiles = computed(() =>
    [...options.document.value.sketches, ...(options.getAdvancedPreviewDocument()?.sketches ?? [])].flatMap(s =>
      s.retainedProfile ? [{ id: s.id, profile: s.retainedProfile }] : [],
    ),
  )
  const profileDisplayNeedsRetry = computed(() => {
    void profileDisplayVersion.value
    return !profileDisplayPending.value && displayProfiles.value.some(item => !profileDisplayQueue.get(item))
  })
  async function refreshProfileDisplay() {
    cancelProfileDisplay()
    profileDisplayErrors.value = []
    if (!options.props.open || !options.kernelReady.value) return
    const generation = profileDisplayGeneration
    profileDisplayPending.value = displayProfiles.value.length > 0
    const result = await profileDisplayQueue.prepare(displayProfiles.value)
    if (generation !== profileDisplayGeneration) return
    if (result.changed) profileDisplayVersion.value++
    profileDisplayErrors.value = result.errors
    profileDisplayPending.value = false
  }

  async function refreshSurfaceDisplay() {
    cancelSurfaceDisplay()
    surfaceDisplayErrors.value = []
    if (!options.props.open || !options.kernelReady.value) return
    const generation = surfaceDisplayGeneration
    const items = [...(options.document.value.surfaces ?? []), ...(options.getAdvancedPreviewDocument()?.surfaces ?? [])]
    surfaceDisplayPending.value = items.length > 0
    const result = await surfaceDisplayQueue.prepare(items)
    if (generation !== surfaceDisplayGeneration) return
    if (result.changed) surfaceDisplayVersion.value++
    surfaceDisplayErrors.value = result.errors
    surfaceDisplayPending.value = false
  }

  return {
    snapPreparation,
    snapPreparationPending,
    snapPreparationVersion,
    snapPreparationErrors,
    snapBodies,
    snapPreparationNeedsRetry,
    cancelSnapPreparation,
    refreshSnapPreparation,
    sketchSnapPreparation,
    sketchSnapPending,
    sketchSnapVersion,
    sketchSnapErrors,
    supportSnapSketches,
    snapSketches,
    sketchSnapsNeedRetry,
    cancelSketchSnaps,
    refreshSketchSnaps,
    topology,
    topologyPending,
    topologyRetryVisible,
    isTopologyReadyFor,
    nextTopologyPick,
    getTopologyPick,
    cancelTopology,
    prepareTopology,
    curveDisplayQueue,
    curveDisplayPending,
    curveDisplayVersion,
    curveDisplayErrors,
    displayCurves,
    curveDisplayNeedsRetry,
    cancelCurveDisplay,
    refreshCurveDisplay,
    curvePoints,
    profileDisplayQueue,
    profileDisplayPending,
    profileDisplayVersion,
    profileDisplayErrors,
    displayProfiles,
    profileDisplayNeedsRetry,
    cancelProfileDisplay,
    refreshProfileDisplay,
    profileLoops,
    surfaceDisplayQueue,
    surfaceDisplayPending,
    surfaceDisplayVersion,
    surfaceDisplayErrors,
    surfaceDisplayNeedsRetry,
    cancelSurfaceDisplay,
    refreshSurfaceDisplay,
  }
}
