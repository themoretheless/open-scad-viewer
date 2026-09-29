import { afterEach, describe, expect, it, vi } from 'vitest'
import { VrPresentation, type VrFrame, type VrSession, type VrSystem } from '../src/services/vrSession'

const identity = () => new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1])
function setup() {
  let callback: (time: number, frame: VrFrame) => void = () => {}
  const session = Object.assign(new EventTarget(), {
    end: vi.fn(async () => { session.dispatchEvent(new Event('end')) }),
    requestReferenceSpace: vi.fn(async () => ({})), updateRenderState: vi.fn(),
    requestAnimationFrame: vi.fn((cb: typeof callback) => { callback = cb; return 1 }), cancelAnimationFrame: vi.fn(),
  }) satisfies VrSession
  const system = Object.assign(new EventTarget(), { requestSession: vi.fn(async () => session), isSessionSupported: vi.fn(async () => true) }) satisfies VrSystem
  const gl = Object.fromEntries(['shaderSource','compileShader','attachShader','linkProgram','useProgram','bindBuffer','bufferData','enable','clearColor','bindFramebuffer','clear','uniformMatrix4fv','viewport','enableVertexAttribArray','vertexAttribPointer','uniform3fv','drawElements','deleteBuffer','deleteShader','deleteProgram'].map(name => [name, vi.fn()]))
  Object.assign(gl, {
    makeXRCompatible: vi.fn(async () => {}), createProgram: () => ({}), createShader: () => ({}), createBuffer: () => ({}),
    getShaderParameter: () => true, getProgramParameter: () => true, getAttribLocation: () => 0, getUniformLocation: (_p: unknown, name: string) => name,
    getExtension: () => null,
  })
  const canvas = Object.assign(new EventTarget(), { getContext: () => gl })
  vi.stubGlobal('document', { createElement: () => canvas })
  vi.stubGlobal('XRWebGLLayer', class {
    framebuffer = {}
    getViewport(view: { eye: number }) { return { x: view.eye * 100, y: 0, width: 100, height: 100 } }
  })
  const changed = vi.fn(), failed = vi.fn()
  const presentation = new VrPresentation(changed, failed)
  const meshes = [{ positions: new Float32Array(9), indices: new Uint32Array([0,1,2]), color: [1,0,0,1] }]
  const frame = () => callback(0, { getViewerPose: () => ({ transform: { matrix: identity() }, views: [0,1].map(eye => ({ eye, projectionMatrix: identity(), transform: { inverse: { matrix: identity() } } })) }) })
  return { session, system, gl, presentation, changed, failed, meshes, frame }
}
afterEach(() => vi.unstubAllGlobals())
describe('VR presentation lifecycle', () => {
  it('requests VR on entry, renders both eyes, recenters and frees resources on exit', async () => {
    const s = setup()
    await s.presentation.start(s.system, s.meshes)
    expect(s.system.requestSession).toHaveBeenCalledWith('immersive-vr')
    expect(s.changed).toHaveBeenLastCalledWith(true)
    s.frame()
    expect(s.gl.drawElements).toHaveBeenCalledTimes(2)
    expect(s.gl.viewport).toHaveBeenNthCalledWith(2, 100,0,100,100)
    const anchor = s.gl.uniformMatrix4fv.mock.calls.find((call: unknown[]) => call[0] === 'anchor')[2]
    expect(anchor[14]).toBeCloseTo(-1.2)
    s.session.dispatchEvent(new Event('select')); s.frame()
    await s.presentation.stop()
    expect(s.gl.deleteBuffer).toHaveBeenCalledTimes(2)
    expect(s.gl.deleteShader).toHaveBeenCalledTimes(2)
    expect(s.gl.deleteProgram).toHaveBeenCalledTimes(1)
    expect(s.changed).toHaveBeenLastCalledWith(false)
    s.frame()
    expect(s.gl.drawElements).toHaveBeenCalledTimes(4)
  })
  it('ends the session and cleans up after setup failure', async () => {
    const s = setup()
    s.session.requestReferenceSpace.mockRejectedValue(new Error('space unavailable'))
    await expect(s.presentation.start(s.system, s.meshes)).rejects.toThrow('space unavailable')
    expect(s.session.end).toHaveBeenCalledOnce()
    expect(s.changed).toHaveBeenLastCalledWith(false)
  })
  it('ends a pending session if the component unmounts before approval', async () => {
    const s = setup()
    let approve!: (session: VrSession) => void
    s.system.requestSession.mockImplementation(() => new Promise(resolve => { approve = resolve as typeof approve }))
    const started = s.presentation.start(s.system, s.meshes)
    s.presentation.dispose()
    approve(s.session)
    await started
    expect(s.session.end).toHaveBeenCalledOnce()
    expect(s.gl.makeXRCompatible).not.toHaveBeenCalled()
  })
  it('handles headset-initiated exit exactly once', async () => {
    const s = setup()
    await s.presentation.start(s.system, s.meshes)
    s.session.dispatchEvent(new Event('end'))
    await s.presentation.stop()
    expect(s.gl.deleteProgram).toHaveBeenCalledOnce()
    expect(s.changed.mock.calls).toEqual([[true], [false]])
  })
})
