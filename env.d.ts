/// <reference types="vite/client" />
/// <reference types="@webgpu/types" />

declare module '*.svet' {
  import type { DefineComponent } from './src/forma/svet-runtime'
  const component: DefineComponent<{}, {}, any>
  export default component
}

declare module '*.ui?raw' {
  const src: string
  export default src
}

