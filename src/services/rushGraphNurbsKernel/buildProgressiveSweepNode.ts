import {progressiveSweepNurbsProfiles} from '../nurbsConstructors'
import type {NurbsCurve} from '../nurbsCurve'
import type {SweepArguments,SweepResolver} from '../rushGraphNurbsKernel'
import type {compileRushGraphNurbs} from '../rushGraphNurbsCompiler'
type Node=Extract<ReturnType<typeof compileRushGraphNurbs>['resolved_document']['nodes'][number],{op:'progressive_sweep'}>
export function buildProgressiveSweepNode(n:Node,needCurve:(id:string)=>NurbsCurve,resolveSweep?:SweepResolver) {
 const key=n.id
                    if(n.orientation==='authored'&&(!n.frame_axis||!n.frame_normal))throw new Error('Authored progressive sweep requires frame_axis and frame_normal');
                    if((n.contact_parameter!==undefined&&!n.orientation_guide)||(n.contact_profile!==undefined&&n.contact_parameter===undefined))throw new Error('Contact anchor requires orientation_guide and contact_parameter');
                    const frameOptions=n.orientation==='authored'?{orientation:'authored' as const,frameAxis:n.frame_axis!,frameNormal:n.frame_normal!}:{orientation:n.orientation};
                    const sweep=(...args:SweepArguments)=>resolveSweep?resolveSweep(key,args):progressiveSweepNurbsProfiles(...args);
                    const builtSweep=sweep(n.inputs.slice(0,-1).map(needCurve),needCurve(n.inputs[n.inputs.length-1]!),n.scale,n.twist,{...(n.orientation_guide?{orientationGuide:needCurve(n.orientation_guide),...(n.contact_parameter!==undefined?{contactAnchor:{profileIndex:n.contact_profile??0,parameter:n.contact_parameter}}:{})}:{}),axisScale:n.axis_scale,centerLaw:n.center_law,normal:n.normal,...frameOptions,spacing:n.spacing,initialSections:n.initial_sections,maxSections:n.max_sections,maxDeviation:n.max_deviation,lengthTolerance:n.length_tolerance,lengthMaxCells:n.length_max_cells,rmfTransportSteps:n.rmf_transport_steps,errorMaxCells:n.error_max_cells,errorMaxProducts:n.error_max_products});
                    if(!builtSweep.report.accepted||!builtSweep.patches){
                        const report=builtSweep.report;
                        if(n.rmf_transport_steps!==undefined&&!report.continuousBound)throw new Error("Progressive sweep continuous retained-patch error is unproved at the configured RMF work budget");
                        const error=report.continuousBound||report.knownProfileErrorUpper!=null
                            ?`continuous retained-patch error ${report.continuousErrorUpper??report.knownProfileErrorUpper}`
                            :`continuous retained-patch error or refinement ${report.sampledControlDeviation}`;
                        throw new Error(`Progressive sweep ${error}mm exceeds ${report.budget}mm at ${report.sections} sections`);
                    }
 return builtSweep
}
