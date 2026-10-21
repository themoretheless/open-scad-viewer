import {bezierCurve,bezierLine} from './bezier-algebraic-cases'
export const irrationalQuartic=bezierCurve([[0,1],[1,-1],[2,1],[3,-1],[4,1]])
export const irrationalContactLine=bezierLine([0,.25],[4,.25])
export const nondyadicTangency=bezierCurve([[0,1],[1.5,-2],[3,4]])
export const nondyadicContactLine=bezierLine([0,0],[3,0])
export const irrationalLoop=bezierCurve([[3,-1.5],[0,2.25],[-1,0],[0,-2.25],[3,1.5]])
export const irrationalCusp=bezierCurve([[0,0],[3,0],[-2,1],[-7,-5],[-4,-2]])
export const multipleFiberFirst=bezierLine([0,0],[0,1])
export const multipleFiberSecond=bezierCurve([[.75,.5],[-1.25,.5],[.75,.5]])
export const multispanFirst={degree:2,knots:[0,0,0,.5,.5,1,1,1],controlPoints:[[0,0],[12,0],[24,1],[36,2],[48,4]],weights:[1,1,1,1,1],periodic:false}
export const multispanSecond=bezierCurve([[0,0],[0,0],[8,0],[24,0],[48,4]])
export const regionBox=(x=0)=>[bezierLine([x,0],[x+4,0]),bezierLine([x+4,0],[x+4,4]),bezierLine([x+4,4],[x,4]),bezierLine([x,4],[x,0])]
export const curvedRegion=[bezierLine([0,0],[4,0]),bezierLine([4,0],[4,4]),bezierCurve([[4,4],[2,6],[0,4]]),bezierLine([0,4],[0,0])]
