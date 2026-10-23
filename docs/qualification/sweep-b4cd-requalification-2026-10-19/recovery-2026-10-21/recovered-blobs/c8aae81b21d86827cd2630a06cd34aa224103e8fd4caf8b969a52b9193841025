import {meshTransferables} from '../core/mesh'
import {buildBrepSemanticScene} from './brepSemanticScene'
import {lowerOpenSCADToSemanticProgram} from './semanticProgramLowerer'
import {diagnosticEnvelope, diagnosticError, isBrepDiagnosticMessage, isBrepDiagnosticRequest,
  type BrepDiagnosticMessage} from './brepDiagnosticProtocol'

/** Source is lowered inside the disposable realm; trusted WeakSet artifacts never cross IPC. */
export function diagnosticWorkerRuntime(post: (message: BrepDiagnosticMessage, transfer: ArrayBuffer[]) => void, close: () => void) {
  let admitted = false
  return (value: unknown) => {
    if (admitted) return
    if (!isBrepDiagnosticRequest(value)) {close(); return}
    admitted = true
    const request = value, envelope = diagnosticEnvelope(request)
    post({...envelope, status: 'started'}, [])
    void (async () => {
      let terminal: BrepDiagnosticMessage
      try {
        const input = lowerOpenSCADToSemanticProgram(request.source, {quality: request.policy.quality})
        const scene = await buildBrepSemanticScene(input, request.policy)
        terminal = {...envelope, status: 'succeeded', scene, programJson: JSON.stringify(input.program)}
        if (!isBrepDiagnosticMessage(terminal, request)) throw new RangeError('Diagnostic scene exceeds its bounded transport contract')
      } catch (error) {terminal = {...envelope, status: 'failed', error: diagnosticError(error)}}
      try {
        post(terminal, terminal.status === 'succeeded' ? meshTransferables(terminal.scene.result.meshes) : [])
      } catch (error) {post({...envelope, status: 'failed', error: diagnosticError(error)}, [])}
      finally {close()}
    })()
  }
}
