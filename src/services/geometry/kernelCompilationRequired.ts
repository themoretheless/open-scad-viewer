/** Qualification Worker runtimes receive compiled native code from their
 * verified parent. They have no embedded compilation fallback or byte payload.
 */
export function compileGeometryKernelArtifact(): Promise<WebAssembly.Module> {
  return Promise.reject(new Error('Verified host geometry module bootstrap required'))
}
export function compileGeometryKernelArtifactSync(): WebAssembly.Module {
  throw new Error('Verified host geometry module bootstrap required')
}
