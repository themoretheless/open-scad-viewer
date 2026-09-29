import {expect,it} from 'vitest'
import {connectedEdgeChain} from '../src/services/edgeSelection'
it('extends an open chain in both directions',()=>expect(connectedEdgeChain([{a:0,b:1},{a:1,b:2},{a:2,b:3}],[1])).toEqual([0,1,2]))
it('stops at branches and does not select a disconnected component',()=>expect(connectedEdgeChain([{a:0,b:1},{a:1,b:2},{a:2,b:3},{a:2,b:4},{a:5,b:6}],[0])).toEqual([0,1]))
it('terminates on a closed loop and ignores invalid seeds',()=>expect(connectedEdgeChain([{a:0,b:1},{a:1,b:2},{a:2,b:0}],[-1,1,99])).toEqual([0,1,2]))
