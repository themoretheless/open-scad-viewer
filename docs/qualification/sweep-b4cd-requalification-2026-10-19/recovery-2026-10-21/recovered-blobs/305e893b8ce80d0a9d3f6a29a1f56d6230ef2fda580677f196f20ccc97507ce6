import type { GeometryQuality } from '../core/build'

interface SchedulerOptions {
  build(quality: GeometryQuality): void
  now?: () => number
  setTimer?: (callback: () => void, delay: number) => unknown
  clearTimer?: (timer: unknown) => void
}

/** One bounded timer: trailing text debounce, throttled drag previews, final promotion. */
export class AutoBuildScheduler {
  private timer: unknown = null
  private timerGeneration = 0
  private dirty = false
  private dragging = false
  private building = false
  private deferredFull = false
  private lastPreviewAt = -Infinity
  private previewCost: number | null = null
  private readonly now: () => number
  private readonly setTimer: NonNullable<SchedulerOptions['setTimer']>
  private readonly clearTimer: NonNullable<SchedulerOptions['clearTimer']>

  constructor(private readonly options: SchedulerOptions) {
    this.now = options.now ?? (() => performance.now())
    this.setTimer = options.setTimer ?? ((callback, delay) => setTimeout(callback, delay))
    this.clearTimer = options.clearTimer ?? (timer => clearTimeout(timer as ReturnType<typeof setTimeout>))
  }

  get delays() {
    const cost = this.previewCost ?? 120
    return { textMs: Math.max(60, Math.min(450, cost)), dragMs: Math.max(100, Math.min(750, cost * 1.5)) }
  }

  observe(quality: GeometryQuality, durationMs: number) {
    if (!Number.isFinite(durationMs) || durationMs < 0) return
    // Host startup/transport are excluded; a full result seeds only the first estimate.
    if (quality === 'full' && this.previewCost !== null) return
    this.previewCost = this.previewCost === null ? durationMs : this.previewCost * 0.6 + durationMs * 0.4
  }

  edited(immediate = false) {
    this.dirty = true
    this.deferredFull = false
    if (immediate) { this.arm(0); return }
    if (this.dragging) {
      // Further pointer events keep the existing deadline instead of postponing forever.
      if (!this.building && this.timer === null) this.arm(Math.max(0, this.lastPreviewAt + this.delays.dragMs - this.now()))
    } else this.arm(this.delays.textMs)
  }

  beginGesture() { this.dragging = true }

  setBuilding(building: boolean) {
    this.building = building
    if (!building && this.dragging && this.dirty && this.timer === null) {
      this.arm(Math.max(0, this.lastPreviewAt + this.delays.dragMs - this.now()))
    }
  }

  endGesture() {
    if (!this.dragging) return
    this.dragging = false
    this.flush()
  }

  flush() {
    if (this.dirty) this.run('preview')
    else if (this.deferredFull) this.run('full')
  }

  /** Only call for a publishable current preview after scene publication finishes. */
  promote() {
    if (this.dirty) return
    if (this.dragging) this.deferredFull = true
    else this.run('full')
  }

  cancelPending() {
    this.timerGeneration++
    if (this.timer !== null) this.clearTimer(this.timer)
    this.timer = null
    this.dirty = false
    this.deferredFull = false
  }

  cancel() { this.cancelPending(); this.dragging = false }

  private run(quality: GeometryQuality) {
    this.cancelPending()
    if (quality === 'preview') this.lastPreviewAt = this.now()
    this.options.build(quality)
  }

  private arm(delay: number) {
    if (this.timer !== null) this.clearTimer(this.timer)
    const generation = ++this.timerGeneration
    this.timer = this.setTimer(() => {
      if (generation !== this.timerGeneration) return
      this.timer = null
      if (this.dirty && (!this.dragging || !this.building)) this.run('preview')
    }, delay)
  }
}
