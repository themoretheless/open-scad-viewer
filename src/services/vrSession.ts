import { prepareVrAnchor } from './vrKernel'
import type { VrMesh } from './vrScene'

// The subset of WebXR consumed by this viewer, without ambient browser globals.
export interface VrView {
  projectionMatrix: Float32Array
  transform: { inverse: { matrix: Float32Array } }
}
export interface VrFrame {
  getViewerPose(space: unknown): { views: readonly VrView[]; transform: { matrix: Float32Array } } | null
}
export interface VrSession extends EventTarget {
  end(): Promise<void>
  requestReferenceSpace(type: 'local'): Promise<unknown>
  updateRenderState(state: { baseLayer: VrLayer; depthNear: number; depthFar: number }): void
  requestAnimationFrame(callback: (time: number, frame: VrFrame) => void): number
  cancelAnimationFrame(id: number): void
}
export interface VrLayer {
  framebuffer: WebGLFramebuffer | null
  getViewport(view: VrView): { x: number; y: number; width: number; height: number } | null
}
export interface VrSystem extends EventTarget {
  isSessionSupported(mode: 'immersive-vr'): Promise<boolean>
  requestSession(mode: 'immersive-vr'): Promise<VrSession>
}
export function vrSystem(): VrSystem | undefined {
  return (navigator as Navigator & { xr?: VrSystem }).xr
}

type XrGl = WebGL2RenderingContext & { makeXRCompatible(): Promise<void> }
const vertexShader = `#version 300 es
in vec3 position;
uniform mat4 projection, view, anchor;
out vec3 worldPosition;
void main() {
  vec4 world = anchor * vec4(position, 1.0);
  worldPosition = world.xyz;
  gl_Position = projection * view * world;
}`
const fragmentShader = `#version 300 es
precision highp float;
in vec3 worldPosition;
uniform vec3 color;
out vec4 outputColor;
void main() {
  vec3 normal = normalize(cross(dFdx(worldPosition), dFdy(worldPosition)));
  float light = 0.35 + 0.65 * abs(dot(normal, normalize(vec3(0.4, 0.8, 0.6))));
  outputColor = vec4(color * light, 1.0);
}`

/** Owns all resources of one immersive session. The desktop canvas is untouched. */
export class VrPresentation {
  private session: VrSession | null = null
  private disposed = false
  private cleanup: (() => void) | null = null
  constructor(private readonly changed: (active: boolean) => void, private readonly failed: (error: unknown) => void) {}

  async start(system: VrSystem, meshes: readonly VrMesh[]): Promise<void> {
    if (this.disposed || this.session) return
    // Called directly from the click handler: preserve transient user activation.
    const session = await system.requestSession('immersive-vr')
    if (this.disposed) { await session.end(); return }
    this.session = session
    const canvas = document.createElement('canvas')
    let gl: XrGl | null = null
    const buffers: WebGLBuffer[] = [], shaders: WebGLShader[] = []
    let program: WebGLProgram | null = null, frameId = 0, ended = false
    const clean = () => {
      if (ended) return
      ended = true
      session.cancelAnimationFrame(frameId)
      session.removeEventListener('end', clean)
      session.removeEventListener('select', recenter)
      canvas.removeEventListener('webglcontextlost', lost)
      if (gl) {
        buffers.forEach(buffer => gl!.deleteBuffer(buffer))
        shaders.forEach(shader => gl!.deleteShader(shader))
        if (program) gl.deleteProgram(program)
        gl.getExtension('WEBGL_lose_context')?.loseContext()
      }
      this.session = null
      this.cleanup = null
      this.changed(false)
    }
    let anchor: Float32Array | null = null
    const recenter = () => { anchor = null }
    const lost = (event: Event) => {
      event.preventDefault()
      this.failed(new Error('VR graphics context lost'))
      void this.stop().catch(this.failed)
    }
    this.cleanup = clean
    session.addEventListener('end', clean)
    session.addEventListener('select', recenter)
    canvas.addEventListener('webglcontextlost', lost)
    try {
      gl = canvas.getContext('webgl2', { alpha: false, antialias: true }) as XrGl | null
      if (!gl) throw new Error('WebGL2 unavailable')
      await gl.makeXRCompatible()
      if (ended || this.disposed) return
      const Layer = (globalThis as unknown as { XRWebGLLayer: new (session: VrSession, gl: XrGl) => VrLayer }).XRWebGLLayer
      const layer = new Layer(session, gl)
      session.updateRenderState({ baseLayer: layer, depthNear: 0.01, depthFar: 100 })
      const space = await session.requestReferenceSpace('local')
      if (ended || this.disposed) return
      program = gl.createProgram()
      if (!program) throw new Error('VR shader allocation failed')
      for (const [type, source] of [[gl.VERTEX_SHADER, vertexShader], [gl.FRAGMENT_SHADER, fragmentShader]] as const) {
        const shader = gl.createShader(type)
        if (!shader) throw new Error('VR shader allocation failed')
        shaders.push(shader)
        gl.shaderSource(shader, source); gl.compileShader(shader)
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? 'VR shader failed')
        gl.attachShader(program, shader)
      }
      gl.linkProgram(program)
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error('VR shader link failed')
      gl.useProgram(program)
      const location = gl.getAttribLocation(program, 'position')
      const uniforms = Object.fromEntries(['projection', 'view', 'anchor', 'color'].map(name => [name, gl!.getUniformLocation(program!, name)]))
      const draws = meshes.map(mesh => {
        const upload = (target: number, data: Float32Array | Uint32Array) => {
          const buffer = gl!.createBuffer()
          if (!buffer) throw new Error('VR buffer allocation failed')
          buffers.push(buffer)
          gl!.bindBuffer(target, buffer); gl!.bufferData(target, data, gl!.STATIC_DRAW)
          return buffer
        }
        return { mesh, vertices: upload(gl!.ARRAY_BUFFER, mesh.positions), indices: upload(gl!.ELEMENT_ARRAY_BUFFER, mesh.indices) }
      })
      gl.enable(gl.DEPTH_TEST)
      gl.clearColor(0.035, 0.045, 0.07, 1)
      const frame = (_time: number, xrFrame: VrFrame) => {
        if (ended) return
        try {
          const pose = xrFrame.getViewerPose(space)
          if (pose) {
            if (!anchor) {
              // Lock the model in the room, initially 1.2 m ahead of the viewer.
              anchor = prepareVrAnchor(pose.transform.matrix)
            }
            gl!.bindFramebuffer(gl!.FRAMEBUFFER, layer.framebuffer)
            gl!.clear(gl!.COLOR_BUFFER_BIT | gl!.DEPTH_BUFFER_BIT)
            gl!.uniformMatrix4fv(uniforms.anchor, false, anchor)
            for (const view of pose.views) {
              const viewport = layer.getViewport(view)
              if (!viewport) continue
              gl!.viewport(viewport.x, viewport.y, viewport.width, viewport.height)
              gl!.uniformMatrix4fv(uniforms.projection, false, view.projectionMatrix)
              gl!.uniformMatrix4fv(uniforms.view, false, view.transform.inverse.matrix)
              for (const draw of draws) {
                gl!.bindBuffer(gl!.ARRAY_BUFFER, draw.vertices)
                gl!.enableVertexAttribArray(location)
                gl!.vertexAttribPointer(location, 3, gl!.FLOAT, false, 0, 0)
                gl!.bindBuffer(gl!.ELEMENT_ARRAY_BUFFER, draw.indices)
                gl!.uniform3fv(uniforms.color, draw.mesh.color.slice(0, 3))
                gl!.drawElements(gl!.TRIANGLES, draw.mesh.indices.length, gl!.UNSIGNED_INT, 0)
              }
            }
          }
          frameId = session.requestAnimationFrame(frame)
        } catch (error) {
          this.failed(error)
          void this.stop().catch(this.failed)
        }
      }
      frameId = session.requestAnimationFrame(frame)
      this.changed(true)
    } catch (error) {
      try { await session.end() } finally { clean() }
      throw error
    }
  }

  async stop(): Promise<void> {
    try { await this.session?.end() } finally { this.cleanup?.() }
  }
  dispose(): void {
    this.disposed = true
    void this.stop().catch(this.failed)
  }
}
