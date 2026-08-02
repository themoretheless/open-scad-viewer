/** A disposable lease over one geometry-kernel runtime. */
export interface GeometryKernelSession<TModule> {
  readonly module: TModule
  dispose(): void
}

/** Kernel lifecycle boundary; language semantics remain compiler-owned. */
export interface GeometryKernel<TModule> {
  readonly implementationKey: string
  warm(): Promise<TModule>
  openSession(): Promise<GeometryKernelSession<TModule>>
}
