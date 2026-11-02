// Install raw-source hooks before linking the TypeScript geometry graph.
import '../src/mcp/registerRawSourceImports.mjs'
import {register} from 'tsx/esm/api'
register()
await import('./export-scalar-sweep-step.mts')
