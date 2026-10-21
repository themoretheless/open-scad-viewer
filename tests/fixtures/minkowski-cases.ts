import {bezierCurve,bezierLine} from './bezier-algebraic-cases'
export const polygonWire=(p:number[][])=>p.map((a,i)=>bezierLine(a,p[(i+1)%p.length]))
export const minkowskiSquare=polygonWire([[0,0],[4,0],[4,4],[0,4]])
export const minkowskiKernel=polygonWire([[-1,-1],[1,-1],[1,1],[-1,1]])
export const minkowskiL=polygonWire([[0,0],[4,0],[4,1],[1,1],[1,4],[0,4]])
export const minkowskiFrame=[polygonWire([[0,0],[10,0],[10,10],[0,10]]),polygonWire([[3,3],[3,7],[7,7],[7,3]])]
export const minkowskiNeck=polygonWire([[0,0],[3,0],[3,1.25],[5,1.25],[5,0],[8,0],[8,3],[5,3],[5,1.75],[3,1.75],[3,3],[0,3]])
export const zeroJetRegion=[bezierCurve([[0,0],[0,0],[4,0],[4,4]]),bezierLine([4,4],[0,4]),bezierLine([0,4],[0,0])]
export const retracedLine=bezierCurve([[0,0],[4,0],[-4,0],[0,0]])
export const monotoneStationary=bezierCurve([[-1,0],[2,0],[-4,0],[8,0]])
export const spanFirst={degree:1,knots:[0,0,1,2,3,3],controlPoints:[[0,0],[4,0],[4,4],[0,4]],weights:[1,1,1,1],periodic:false}
export const spanSecond={degree:1,knots:[0,0,1,2,3,3],controlPoints:[[2,0],[4,0],[4,2],[4,4]],weights:[1,1,1,1],periodic:false}
export const rationalKnotFirst={degree:1,knots:[0,0,1,2,2],controlPoints:[[0,0],[3,3],[6,0]],weights:[1,1,1],periodic:false}
export const rationalKnotSecond=bezierLine([0,1],[6,1])
export const retracingImageFirst=bezierCurve([[0,0],[6,0],[0,0]])
export const retracingImageSecond=bezierCurve([[0,0],[0,0],[8,0],[0,0],[0,0]])
export const cuspImageFirst=bezierCurve([[60,-60],[-20,60],[-20,-60],[60,60]])
const choose=(n:number,k:number)=>n<k?0:Array.from({length:k},(_,i)=>i).reduce((v,i)=>v*(n-i)/(i+1),1)
export const cuspImageSecond=bezierCurve(Array.from({length:7},(_,i)=>[60-16*choose(i,2)+16*choose(i,4),-60+24*choose(i,2)-48*choose(i,4)+480*choose(i,6)]))
