declare module 'harfbuzzjs/hb.wasm?url' {
  const url: string
  export default url
}

declare module 'harfbuzzjs/hb.js' {
  interface HarfBuzzModuleOptions {
    readonly locateFile?: (path: string) => string
  }

  const createHarfBuzzModule: (options?: HarfBuzzModuleOptions) => Promise<unknown>
  export default createHarfBuzzModule
}

declare module 'harfbuzzjs/hbjs.js' {
  const bindHarfBuzz: (module: unknown) => unknown
  export default bindHarfBuzz
}
