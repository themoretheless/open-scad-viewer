import { register } from 'tsx/esm/api'
import * as module from 'node:module'
import {load} from './raw-import-loader.mjs'

// Install in the actual worker before its shipped entry imports WGSL. Keep
// Node20's async loader fallback; newer Node supports synchronous hooks.
if(typeof module.registerHooks==='function')module.registerHooks({load})
else module.register(new URL('./raw-import-loader.mjs',import.meta.url),import.meta.url)
register()
await import('./web-worker-node-harness.ts')
