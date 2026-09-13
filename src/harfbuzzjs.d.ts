declare module 'harfbuzzjs/hb.js' {
  interface HarfBuzzModuleOptions {
    readonly locateFile?: (path: string) => string
    readonly wasmBinary?: Uint8Array<ArrayBuffer>
  }

  const createHarfBuzzModule: (options?: HarfBuzzModuleOptions) => Promise<unknown>
  export default createHarfBuzzModule
}

declare module 'harfbuzzjs/hbjs.js' {
  const bindHarfBuzz: (module: unknown) => unknown
  export default bindHarfBuzz
}
