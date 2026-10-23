export type RendererRecoveryLossDecision = 'start' | 'latched' | 'exhausted'
export type RendererRecoveryCompletion = 'done' | 'retry' | 'exhausted'

/**
 * Coalesces device-loss notifications into one recovery attempt plus at most
 * one follow-up. A replacement device that is itself lost therefore cannot
 * leave the application reporting a false-ready renderer or retry forever.
 */
export class RendererRecoveryGate {
  private active = false
  private followUpPending = false
  private followUpStarted = false
  private exhausted = false

  get isActive() { return this.active }
  get mustDeferReady() { return this.followUpPending || this.exhausted }

  registerDeviceLoss(): RendererRecoveryLossDecision {
    if (!this.active) {
      this.active = true
      this.followUpPending = false
      this.followUpStarted = false
      this.exhausted = false
      return 'start'
    }

    if (!this.followUpStarted) {
      this.followUpPending = true
      return 'latched'
    }

    this.exhausted = true
    return 'exhausted'
  }

  completeAttempt(): RendererRecoveryCompletion {
    if (!this.active) return 'done'
    if (this.followUpPending && !this.followUpStarted) {
      this.followUpPending = false
      this.followUpStarted = true
      return 'retry'
    }

    const completion: RendererRecoveryCompletion = this.exhausted ? 'exhausted' : 'done'
    this.reset()
    return completion
  }

  reset() {
    this.active = false
    this.followUpPending = false
    this.followUpStarted = false
    this.exhausted = false
  }
}
