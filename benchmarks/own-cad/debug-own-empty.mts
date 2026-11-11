import {evaluateOpenSCADViaManifoldPlanForQualification as run} from '../../src/kernel/manifoldPlanEvaluator'
for(const s of ['polygon(points=[]);','polygon(points=[],paths=[]);'])try{console.log(await run(s))}catch(e){console.dir(e,{depth:6})}
