import { ref, shallowRef } from 'vue'
import type { MainSolidJob, MainSolidProgress, MainSolidResults } from '../services/mainSolidProtocol'
import type { MainSolidWorkerClient } from '../services/mainSolidWorkerClient'

/** Shared UI lifecycle for cancellable CAD jobs. Only the current generation may
 * publish progress, commit a result, or report an error. */
export function useMainSolidTask(worker: MainSolidWorkerClient) {
  const pending = ref(false)
  const progress = shallowRef<MainSolidProgress | null>(null)
  const kind = ref<MainSolidJob['kind'] | null>(null)
  let generation = 0

  async function run<K extends MainSolidJob['kind']>(
    job: Extract<MainSolidJob, {kind: K}>,
    accept: (result: MainSolidResults[K]) => void,
    reject: (error: unknown) => void,
  ): Promise<boolean> {
    if (pending.value) return false
    const current = ++generation
    kind.value = job.kind
    pending.value = true
    progress.value = null
    try {
      const result = await worker.run(job, {
        onProgress: value => { if (current === generation) progress.value = value },
      })
      if (current !== generation) return false
      accept(result)
      return true
    } catch (error) {
      if (current !== generation) return false
      reject(error)
      return false
    } finally {
      if (current === generation) {
        pending.value = false
        progress.value = null
      }
    }
  }

  function cancel() {
    if (!pending.value) return
    generation++
    worker.cancel()
    pending.value = false
    progress.value = null
  }

  function dispose() {
    generation++
    pending.value = false
    progress.value = null
    worker.dispose()
  }

  return {pending, progress, kind, run, cancel, dispose}
}
