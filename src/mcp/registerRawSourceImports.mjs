import * as module from 'node:module'
import {load} from './rawSourceLoader.mjs'
// Node 22 synchronous hooks compose with tsx; Node 20 uses the loader thread.
if(typeof module.registerHooks==='function')module.registerHooks({load})
else module.register(new URL('./rawSourceLoader.mjs',import.meta.url),import.meta.url)
