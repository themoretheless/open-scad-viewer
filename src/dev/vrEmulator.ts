import { XRDevice, metaQuest3, XRReferenceSpaceType } from 'iwer'
import { DevUI } from '@iwer/devui'

/** Explicit, development-only WebXR runtime; never included in production. */
export function installVrEmulator(): void {
  const device = new XRDevice(metaQuest3, { stereoEnabled: true })
  device.installRuntime({ forceInstall: true })
  device.installDevUI(DevUI)
  // DevUI defaults to zero IPD for its overlay. Restore real stereo separation.
  device.ipd = 0.063

  // Deterministic controls complement the emulator's free-movement UI, making
  // the same smoke test repeatable without a headset or browser extension.
  const panel = document.createElement('aside')
  panel.setAttribute('aria-label', 'WebXR emulator test controls')
  panel.style.cssText = 'position:fixed;bottom:32px;left:12px;z-index:10000;background:#17202e;color:white;padding:10px;border:1px solid #62748d;border-radius:8px;font:12px sans-serif;max-width:660px'
  const status = document.createElement('div')
  panel.append(status)
  const evidence = document.createElement('output')
  evidence.style.display = 'block'
  const initial = device.position.clone()
  let yaw = 0
  function button(label: string, action: () => void) {
    const button = document.createElement('button')
    button.textContent = label
    button.style.cssText = 'margin:6px 4px 0 0;padding:5px;color:white;background:#334155;border:1px solid #64748b;border-radius:4px'
    button.onclick = () => {
      device.controlMode = 'programmatic'
      action()
      device.notifyStateChange()
    }
    panel.append(button)
  }
  button('Head +30 cm', () => { device.position.x += 0.3 })
  button('Turn 20°', () => {
    yaw += Math.PI / 9
    device.quaternion.set(0, Math.sin(yaw / 2), 0, Math.cos(yaw / 2))
  })
  button('Reset head', () => { device.position.copy(initial); device.quaternion.set(0, 0, 0, 1); yaw = 0 })
  button('Controller trigger', () => {
    device.controllers.right?.updateButtonValue('trigger', 1)
    window.setTimeout(() => device.controllers.right?.updateButtonValue('trigger', 0), 150)
  })
  button('Inspect stereo', () => {
    const session = device.activeSession
    if (!session) { evidence.textContent = 'No active session'; return }
    void session.requestReferenceSpace(XRReferenceSpaceType.Local).then(space => {
      session.requestAnimationFrame((_time, frame) => {
        const views = frame.getViewerPose(space)?.views ?? []
        const positions = views.map(view => view.transform.position)
        const distance = positions.length === 2 ? Math.hypot(positions[0].x - positions[1].x, positions[0].y - positions[1].y, positions[0].z - positions[1].z) : 0
        const gl = device.appCanvas?.getContext('webgl2')
        evidence.textContent = `Views: ${views.map(view => view.eye).join(', ')} · eye distance ${(distance * 1000).toFixed(1)} mm · WebGL error ${gl?.getError() ?? 'unavailable'}`
      })
    }).catch(error => { evidence.textContent = String(error) })
  })
  button('Free movement', () => { device.controlMode = 'manual' })
  button('End XR session', () => { void device.activeSession?.end() })
  panel.append(evidence)
  document.body.append(panel)
  const timer = window.setInterval(() => {
    status.textContent = `Meta Quest 3 emulator · ${device.activeSession ? 'session active' : 'session inactive'} · stereo · head x=${device.position.x.toFixed(2)} m · yaw=${Math.round(yaw * 180 / Math.PI)}°`
  }, 200)
  import.meta.hot?.dispose(() => {
    clearInterval(timer)
    void device.activeSession?.end()
    device.uninstallRuntime()
    panel.remove()
  })
}
