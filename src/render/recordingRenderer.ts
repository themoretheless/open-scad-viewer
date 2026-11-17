/**
 * CPU-only stand-in for the WebGPU viewport renderer, used by browser smoke
 * tests in headless Chromium without a GPU. It keeps the CPU-side state the
 * UI reads back (camera, camera history, selection, visibility, display
 * settings) and records every call and frame instead of drawing.
 *
 * `implements` over the public keys of WebGPURenderer makes the compiler
 * reject any drift between the two public surfaces.
 */
import type { MeshData } from '../core/mesh'
import { DEFAULT_DISPLAY_BUDGET, sameDisplayBudget, type DisplayBudget } from './displayScheduler'
import { carryMeshPublication, deltaMeshFromMeshData, diffSceneEntities } from './sceneDelta'
import { countOperation } from '../dev/operationCounters'
import { InstancedSceneBuffers, instancePickId, resolveInstancePick, type InstancingMetrics } from './instancedScene'
import { CameraHistory, cameraStatesEqual, type CameraHistorySnapshot, type CameraState } from './cameraHistory'
import { clampGestureDistance, computeOrbitUpdate, computePanUpdate, computeWheelDistance } from './cameraGestures'
import type { KeyboardCameraStep } from './keyboardCamera'
import {
  DEFAULT_THEME,
  getThemePreset,
  type CameraChangeHandler,
  type CameraHistoryChangeHandler,
  type DisplayMode,
  type DistanceMeasurement,
  type HoverChangeHandler,
  type MeasurementChangeHandler,
  type PickHit,
  type RendererLifecycleEvent,
  type RendererStatusChangeHandler,
  type RenderTheme,
  type SceneUploadMetrics,
  type SelectionChangeHandler,
  type SelectionMode,
  type SetMeshesOptions,
  type ShadingModel,
} from './rendererContracts'
import type { WebGPUFrameDiagnostics } from './types'
import { ISO_PITCH, ISO_YAW, standardViewOrientation, type ProjectionMode, type StandardView } from './viewportModel'
import type { GpuBufferArena, GpuCsgTree, GpuWboitRenderer, SlicerResult, WebGPURenderer } from './webgpuRenderer'

type PublicRenderer = Pick<WebGPURenderer, keyof WebGPURenderer>

export interface RecordedCall {
  readonly method: string
  readonly at: number
}

export interface RecordingSceneState {
  readonly status: RendererLifecycleEvent['status']
  readonly meshCount: number
  readonly triangleCount: number
  readonly visibleCount: number
  readonly selectedIndex: number | null
  readonly isolated: boolean
  readonly displayMode: DisplayMode
  readonly selectionMode: SelectionMode
  readonly gridVisible: boolean
  readonly camera: CameraState
  readonly cameraHistory: number
  readonly framesRequested: number
  readonly framesDrawn: number
  readonly sceneUploads: number
  readonly measuring: boolean
  readonly displayBudget: DisplayBudget
  readonly calls: Readonly<Record<string, number>>
}

const CALL_LOG_LIMIT = 512

/** Buffer-less device: the recorder stages instances only to count uploads. */
const NULL_DEVICE = {
  createBuffer: () => ({ destroy() {} }) as unknown as GPUBuffer,
  queue: { writeBuffer() {} } as unknown as GPUQueue,
}

export class RecordingRenderer implements PublicRenderer {
  onFrameSubmitted: ((token: number, submittedAt: number) => void) | null = null
  onSelectionChange: SelectionChangeHandler | null = null
  onHoverChange: HoverChangeHandler | null = null
  onMeasurementChange: MeasurementChangeHandler | null = null
  onCameraHistoryChange: CameraHistoryChangeHandler | null = null
  onCameraChange: CameraChangeHandler | null = null
  onStatusChange: RendererStatusChangeHandler | null = null

  readonly callLog: RecordedCall[] = []
  private readonly callCounts = new Map<string, number>()
  private status: RendererLifecycleEvent = { status: 'idle' }
  private canvas: HTMLCanvasElement | null = null
  private meshes: MeshData[] = []
  private visibility: boolean[] = []
  private selected: number | null = null
  private selectedHit: PickHit | null = null
  private isolated = false
  private displayMode: DisplayMode = 'shaded'
  private selectionMode: SelectionMode = 'object'
  private shadingModel: ShadingModel = 'phong'
  private theme: RenderTheme = DEFAULT_THEME
  private matcap = 'procedural'
  private env = 'none'
  private shadows = false
  private gridVisible = true
  private transparency: 'sorted' | 'wboit' = 'sorted'
  private displayBudget: DisplayBudget = DEFAULT_DISPLAY_BUDGET
  private csgTree: GpuCsgTree | null = null
  private measurement: DistanceMeasurement | null = null
  private camera: CameraState = {
    yaw: ISO_YAW,
    pitch: ISO_PITCH,
    distance: 100,
    target: [0, 0, 0],
    projection: 'perspective',
  }
  private readonly history = new CameraHistory()
  private raf = 0
  private pendingFrameToken: number | null = null
  private framesRequested = 0
  private framesDrawn = 0
  private sceneUploads = 0
  private uploadMetrics: SceneUploadMetrics = { geometryUploadBytes: 0, geometryBuffersCreated: 0, reusedEntities: 0 }
  private readonly instanced = new InstancedSceneBuffers(NULL_DEVICE)

  private record(method: string) {
    this.callCounts.set(method, (this.callCounts.get(method) ?? 0) + 1)
    if (this.callLog.length >= CALL_LOG_LIMIT) this.callLog.shift()
    this.callLog.push({ method, at: performance.now() })
  }

  private setStatus(event: RendererLifecycleEvent) {
    this.status = event
    try {
      this.onStatusChange?.(event)
    } catch {
      /* UI callbacks must not break the recorder. */
    }
  }

  private emitCamera() {
    try {
      this.onCameraChange?.(this.getCameraState())
    } catch {
      /* ignored */
    }
  }

  private emitHistory() {
    try {
      this.onCameraHistoryChange?.(this.canGoToPreviousView)
    } catch {
      /* ignored */
    }
  }

  private emitSelection() {
    try {
      this.onSelectionChange?.(this.selected, this.isolated, this.selectedHit)
    } catch {
      /* ignored */
    }
  }

  private commitCamera(next: CameraState): boolean {
    const before = this.getCameraState()
    const clamped: CameraState = { ...next, distance: clampGestureDistance(next.distance), target: [...next.target] }
    if (cameraStatesEqual(before, clamped)) return false
    if (this.history.record(before)) this.emitHistory()
    this.camera = clamped
    this.emitCamera()
    this.requestRender()
    return true
  }

  snapshot(): RecordingSceneState {
    return {
      status: this.status.status,
      meshCount: this.meshes.length,
      triangleCount: this.meshes.reduce((sum, mesh) => sum + mesh.indices.length / 3, 0),
      visibleCount: this.meshes.filter((_, index) => this.visibility[index] !== false).length,
      selectedIndex: this.selected,
      isolated: this.isolated,
      displayMode: this.displayMode,
      selectionMode: this.selectionMode,
      gridVisible: this.gridVisible,
      camera: this.getCameraState(),
      cameraHistory: this.history.size,
      framesRequested: this.framesRequested,
      framesDrawn: this.framesDrawn,
      sceneUploads: this.sceneUploads,
      measuring: this.measurement !== null,
      displayBudget: this.displayBudget,
      calls: Object.fromEntries(this.callCounts),
    }
  }

  async init(canvas: HTMLCanvasElement): Promise<boolean> {
    this.record('init')
    this.canvas = canvas
    this.setStatus({ status: 'initializing' })
    await Promise.resolve()
    if (this.canvas !== canvas) return false
    this.setStatus({ status: 'ready' })
    return true
  }

  destroy() {
    this.record('destroy')
    if (this.raf) cancelAnimationFrame(this.raf)
    this.raf = 0
    this.canvas = null
    this.instanced.destroy()
    this.setStatus({ status: 'destroyed' })
  }

  get currentStatus(): RendererLifecycleEvent {
    return this.status
  }
  // Public camera fields of WebGPURenderer, mapped onto the recorded camera.
  get yaw() {
    return this.camera.yaw
  }
  set yaw(value: number) {
    this.camera = { ...this.camera, yaw: value }
  }
  get pitch() {
    return this.camera.pitch
  }
  set pitch(value: number) {
    this.camera = { ...this.camera, pitch: value }
  }
  get dist() {
    return this.camera.distance
  }
  set dist(value: number) {
    this.camera = { ...this.camera, distance: value }
  }
  get tx() {
    return this.camera.target[0]
  }
  set tx(value: number) {
    this.camera = { ...this.camera, target: [value, this.camera.target[1], this.camera.target[2]] }
  }
  get ty() {
    return this.camera.target[1]
  }
  set ty(value: number) {
    this.camera = { ...this.camera, target: [this.camera.target[0], value, this.camera.target[2]] }
  }
  get tz() {
    return this.camera.target[2]
  }
  set tz(value: number) {
    this.camera = { ...this.camera, target: [this.camera.target[0], this.camera.target[1], value] }
  }
  get sceneUploadMetrics(): SceneUploadMetrics {
    return { ...this.uploadMetrics }
  }
  /** Instanced staging: one geometry upload per new asset, one draw per batch, edges on demand. */
  get instancingMetrics(): InstancingMetrics {
    return { ...this.instanced.metrics }
  }
  /** Mesh index for the pick id an instanced draw writes, or null. */
  resolveInstancePick(pickId: number): number | null {
    return resolveInstancePick(this.instanced.plan, pickId)
  }
  instancePickId(meshIndex: number): number | null {
    return instancePickId(this.instanced.plan, meshIndex)
  }
  get currentDisplayMode(): DisplayMode {
    return this.displayMode
  }
  get selectedIndex(): number | null {
    return this.selected
  }
  get currentHit(): PickHit | null {
    return this.selectedHit
  }
  get isIsolated(): boolean {
    return this.isolated
  }
  get canGoToPreviousView(): boolean {
    return this.history.size > 0
  }
  get currentTheme(): RenderTheme {
    return this.theme
  }
  get currentDefaultShadingModel(): ShadingModel {
    return this.shadingModel
  }
  get currentMatcap(): string {
    return this.matcap
  }
  get currentEnv(): string {
    return this.env
  }
  get shadowsOn(): boolean {
    return this.shadows
  }
  get wboitRenderer(): GpuWboitRenderer {
    throw new Error('WBOIT requires WebGPU; the recording renderer has no device.')
  }

  requestRender(_resetFrameRetry = true) {
    this.framesRequested++
    if (this.raf || this.status.status !== 'ready' || typeof requestAnimationFrame !== 'function') return
    this.raf = requestAnimationFrame(() => {
      this.raf = 0
      if (this.status.status !== 'ready') return
      this.framesDrawn++
      countOperation('viewportFrames')
      const token = this.pendingFrameToken
      this.pendingFrameToken = null
      if (token !== null) {
        try {
          this.onFrameSubmitted?.(token, performance.now())
        } catch {
          /* ignored */
        }
      }
    })
  }

  setMeshes(meshes: MeshData[], options: SetMeshesOptions = {}) {
    this.record('setMeshes')
    if (this.status.status !== 'ready') return
    const { selected, selectedHit, isolated, measurement } = this
    const previous = this.meshes
    const kept = carryMeshPublication(previous, meshes, { selected, selectedHit, isolated, measurement }, options)
    this.meshes = meshes
    this.visibility = meshes.map(() => true)
    this.selected = kept.selected
    this.selectedHit = kept.selectedHit
    this.isolated = kept.isolated
    this.measurement = kept.measurement
    this.sceneUploads++
    countOperation('viewportSceneUploads')
    // Mirror the WebGPU delta: only added or geometry-changed entities upload.
    const delta = diffSceneEntities(previous.map(deltaMeshFromMeshData), meshes.map(deltaMeshFromMeshData))
    const uploaded = [...delta.added, ...delta.geometryChanged]
    const bytes = uploaded.reduce(
      (sum, index) => sum + meshes[index]!.vertices.byteLength + meshes[index]!.indices.byteLength,
      0,
    )
    this.uploadMetrics = {
      geometryUploadBytes: bytes,
      geometryBuffersCreated: uploaded.length * 2,
      reusedEntities: meshes.length - uploaded.length,
    }
    this.instanced.stage(meshes)
    if (this.displayMode === 'edges') this.ensureInspectionEdges()
    this.pendingFrameToken = options.frameToken ?? null
    this.requestRender()
  }

  /** Inspection edges are built only when a mode needs them, once per asset. */
  private ensureInspectionEdges() {
    this.instanced.plan.batches.forEach((_, index) => this.instanced.ensureEdges(index))
  }

  setMeshVisibility(index: number, visible: boolean) {
    this.record('setMeshVisibility')
    if (index < 0 || index >= this.meshes.length) return
    this.visibility[index] = visible
    this.requestRender()
  }

  setMeshVisibilityBatch(visibility: readonly boolean[]): boolean {
    this.record('setMeshVisibilityBatch')
    if (visibility.length !== this.meshes.length) return false
    this.visibility = [...visibility]
    this.requestRender()
    return true
  }

  selectMesh(index: number | null) {
    this.record('selectMesh')
    const next = index !== null && index >= 0 && index < this.meshes.length ? index : null
    this.selected = next
    this.selectedHit = null
    this.emitSelection()
    this.requestRender()
  }

  clearSelection() {
    this.record('clearSelection')
    this.selected = null
    this.selectedHit = null
    this.isolated = false
    this.emitSelection()
    this.requestRender()
  }

  restoreNativeFaceSelection(_previous: MeshData, _hit: PickHit, _index: number): boolean {
    this.record('restoreNativeFaceSelection')
    return false
  }

  toggleIsolateSelection(): boolean {
    this.record('toggleIsolateSelection')
    if (this.selected === null) return false
    this.isolated = !this.isolated
    this.emitSelection()
    this.requestRender()
    return true
  }

  setPreselection(_index: number | null) {
    this.record('setPreselection')
  }
  setSourceHighlight(_sourceId: number | null) {
    this.record('setSourceHighlight')
  }

  fitView() {
    this.record('fitView')
    this.commitCamera({ ...this.camera, target: [0, 0, 0] })
  }

  fitSelection(): boolean {
    this.record('fitSelection')
    return this.selected !== null
  }

  resetView() {
    this.record('resetView')
    this.commitCamera({
      yaw: ISO_YAW,
      pitch: ISO_PITCH,
      distance: this.camera.distance,
      target: [0, 0, 0],
      projection: this.camera.projection,
    })
  }

  setView(view: StandardView) {
    this.record('setView')
    const [yaw, pitch] = standardViewOrientation(view)
    this.commitCamera({ ...this.camera, yaw, pitch })
  }

  setCameraPreset(view: StandardView, projection: ProjectionMode) {
    this.record('setCameraPreset')
    const [yaw, pitch] = standardViewOrientation(view)
    this.commitCamera({ ...this.camera, yaw, pitch, projection })
  }

  setProjection(projection: ProjectionMode) {
    this.record('setProjection')
    this.commitCamera({ ...this.camera, projection })
  }

  setOrbitAngles(yaw: number, pitch: number): boolean {
    this.record('setOrbitAngles')
    return this.commitCamera({ ...this.camera, yaw, pitch })
  }

  setPerspectiveFieldOfView(_radians: number) {
    this.record('setPerspectiveFieldOfView')
    this.requestRender()
  }

  applyKeyboardCameraStep(step: KeyboardCameraStep): boolean {
    this.record('applyKeyboardCameraStep')
    const {
      yaw,
      pitch,
      distance: dist,
      target: [tx, ty, tz],
    } = this.camera
    if (step.kind === 'zoom')
      return this.commitCamera({ ...this.camera, distance: computeWheelDistance(dist, step.delta) })
    const state = { yaw, pitch, dist, tx, ty, tz }
    const next =
      step.kind === 'pan'
        ? computePanUpdate(state, step.dx, step.dy, this.canvas?.clientHeight || 1)
        : computeOrbitUpdate(state, step.dx, step.dy)
    return this.commitCamera({
      yaw: next.yaw,
      pitch: next.pitch,
      distance: next.dist,
      target: [next.tx, next.ty, next.tz],
      projection: this.camera.projection,
    })
  }

  getCameraState(): CameraState {
    return { ...this.camera, target: [...this.camera.target] }
  }

  getCameraHistorySnapshot(): CameraHistorySnapshot {
    return this.history.snapshot()
  }

  restoreCameraHistory(snapshot: CameraHistorySnapshot): boolean {
    const was = this.canGoToPreviousView
    if (!this.history.restore(snapshot)) return false
    if (was !== this.canGoToPreviousView) this.emitHistory()
    return true
  }

  restoreCameraState(state: CameraState): boolean {
    this.record('restoreCameraState')
    if (cameraStatesEqual(this.camera, state)) return false
    this.camera = { ...state, target: [...state.target] }
    this.requestRender()
    return true
  }

  previousView(): CameraState | null {
    this.record('previousView')
    const state = this.history.previous()
    if (!state) return null
    this.camera = { ...state, target: [...state.target] }
    this.emitHistory()
    this.emitCamera()
    this.requestRender()
    return state
  }

  setDisplayMode(mode: DisplayMode) {
    this.record('setDisplayMode')
    this.displayMode = mode
    if (mode === 'edges') this.ensureInspectionEdges()
    this.requestRender()
  }
  setBackgroundColor(_color: readonly [number, number, number]) {
    this.record('setBackgroundColor')
    this.requestRender()
  }
  setTheme(themeId: string) {
    this.record('setTheme')
    this.theme = getThemePreset(themeId) ?? DEFAULT_THEME
    this.requestRender()
  }
  setDefaultShadingModel(model: ShadingModel) {
    this.record('setDefaultShadingModel')
    this.shadingModel = model
    this.requestRender()
  }
  setDefaultMaterial(_defaults: Parameters<WebGPURenderer['setDefaultMaterial']>[0]) {
    this.record('setDefaultMaterial')
    this.requestRender()
  }
  setMatcapTexture(id: string): Promise<void> {
    this.record('setMatcapTexture')
    this.matcap = id
    return Promise.resolve()
  }
  setEnvMap(id: string): Promise<void> {
    this.record('setEnvMap')
    this.env = id
    return Promise.resolve()
  }
  setShadowsEnabled(enabled: boolean) {
    this.record('setShadowsEnabled')
    this.shadows = enabled
    this.requestRender()
  }
  setGridVisible(visible: boolean) {
    this.record('setGridVisible')
    this.gridVisible = visible
    this.requestRender()
  }
  setGridStep(_step: number) {
    this.record('setGridStep')
    this.requestRender()
  }
  setSelectionMode(mode: SelectionMode) {
    this.record('setSelectionMode')
    this.selectionMode = mode
    this.requestRender()
  }
  setSection(_enabled: boolean, _normal: readonly [number, number, number], _offset: number) {
    this.record('setSection')
    this.requestRender()
  }
  setDisplayBudget(budget: DisplayBudget) {
    this.record('setDisplayBudget')
    if (sameDisplayBudget(this.displayBudget, budget)) return
    this.displayBudget = budget
    this.requestRender()
  }
  getDisplayBudget(): DisplayBudget {
    return this.displayBudget
  }
  private instancing = false
  isInstancingEnabled(): boolean {
    return this.instancing
  }
  setInstancingEnabled(enabled: boolean) {
    this.record('setInstancingEnabled')
    this.instancing = enabled
    this.requestRender()
  }
  getTransparencyMode(): 'sorted' | 'wboit' {
    return this.transparency
  }
  setTransparencyMode(mode: 'sorted' | 'wboit') {
    this.record('setTransparencyMode')
    this.transparency = mode
    this.requestRender()
  }

  setMeasureMode(_active: boolean) {
    this.record('setMeasureMode')
  }
  clearMeasurement() {
    this.record('clearMeasurement')
    this.measurement = null
    try {
      this.onMeasurementChange?.(null, false)
    } catch {
      /* ignored */
    }
  }
  restoreMeasurement(measurement: DistanceMeasurement | null, _active = false): boolean {
    this.record('restoreMeasurement')
    this.measurement = measurement
    return true
  }

  setCsgPreviewTree(tree: GpuCsgTree | null) {
    this.record('setCsgPreviewTree')
    this.csgTree = tree
    this.requestRender()
  }
  getCsgPreviewTree(): GpuCsgTree | null {
    return this.csgTree
  }
  previewCsgSource(_source: string) {
    this.record('previewCsgSource')
  }

  async gpuPickAt(_clientX: number, _clientY: number): Promise<number | null> {
    this.record('gpuPickAt')
    return null
  }
  getBufferArena(): GpuBufferArena | null {
    return null
  }
  async sliceSceneMeshesGpu(
    _options: Parameters<WebGPURenderer['sliceSceneMeshesGpu']>[0],
  ): Promise<Array<{ meshIndex: number; slice: SlicerResult }>> {
    this.record('sliceSceneMeshesGpu')
    throw new Error('GPU slicing requires WebGPU; the recording renderer has no device.')
  }

  resize() {
    this.record('resize')
    this.requestRender()
  }
  projectWorldPoint(_point: readonly number[]): [number, number] | null {
    return null
  }
  worldRay(_clientX: number, _clientY: number): ReturnType<WebGPURenderer['worldRay']> {
    return null
  }

  getFrameDiagnostics(): WebGPUFrameDiagnostics {
    return {
      opaqueMeshCount: this.meshes.length,
      transparentMeshCount: 0,
      edgeMeshCount: 0,
      totalMeshCount: this.meshes.length,
      deviceLost: false,
    }
  }
}
