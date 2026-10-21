# Native200: original branch germs and leading-ray cyclic ports, isolated

Previous goalturn PROGRESS: exact sourceparameter order andglobalisolated atlas.
This turn PROGRESS: actualrational firstnonzero branchgerms, exactcrossprimitive
rationalvalueorder, originalcyclicdistinct-leadingray contactports withfullsource
binding. Goalactive/unbounded; main198/200, N0030/N0173planned. NoGit/agents.
Memory895read. Neverinflatehelpercounts. Newgeometry/proofimplementation isRust.

NEXT RUST EDITS candidate /private/tmp/nurbs-native-200-next-6v12adcx.
Latestcumulative16filepatch supersedesold6/11patches, don'tapplyboth. Main3parents
stillmatchbaseline andnew13files absentmain. Sourcecopybaseline3365files and63
read-onlycompilefixtures retained. SharedRust/release snapshot unchanged.
Incomingcoordination notesstableV5snapshot/WASMsaved, futureworkseparate;
resources high: futureCARGO_BUILD_JOBS=2,RUST_TEST_THREADS=2, explicit--jobs1.
DoNOTinterruptotherjobs/goals ormessageotherchat withouthuman authorization.

New arithmetic/branch_germ.rs evaluatesACTUALhomogeneous originalcell jet
H(delta)=X(s+direction*delta)W(s)-X(s)W(s+direction*delta), similarlyY. Derivative
numerators(order m scaled m!) composedmodactualprimitiveF. Firstnonzero actual
selectedroot vector m<=originaldegree, signsfromGCDzero_at pluscheckedQinterval
refinement (new event_coordinates::sign_at); denominatorpositive. No arbitrary
unit tangent/zerovelocity replacement. leadingorder andrays proofONLY, nothigher
intrinsicbranchangle. OriginalBasis.validate disallows interior multiplicity
>degree soactualsourcecontinuous acrossknots (verifiedinexistingcorecurve.rs).

New event_nodes/branches.rs LeadingBranchRay andOwnedEventBranch. Fresh full
originaleventnode/sourcepaircensus first. ExactnaturalP==knot viaGCD determines
incomingPREVIOUScell/outgoingRIGHTcell. Endpointoutside direction refuses;
try_own_event_branch returnsNone ONLYafter exactoutside-domain proof, ALLproof/
budgetfailuresErr. Cusp firstnonzero m2 bothsidesleading+X distinctgerms; cubic
stationary m3 incoming/outopposite; rationalnonunitclock/knots actualtests pass.

Extended arithmetic/parameter_order.rs rationalcomparison N(alpha)/D(alpha).
SelectedDen0 refuses. Removegcd(F,D) factors awayfromselectedroot beforevalue
elimination, retainingactualroot; no illegal inverse inreduciblequotient.
Compute Res_alpha(F,value*D-N), squarefree commonvaluepolys, homogeneous
G(N/D)*D^degree checkedviaactualselectedroot GCD, uniquecommonroot inexactjoint
fractionbounds grantsEqual. NegativeDen interval arithmetic exact. Constant/
sameprimitive/singleton exactfastpaths; resultantsbudget refusal canstillprove
strictorder viaoriginalrootrefinement. Sameexisting explicitdegree/bit/matrix/
refinement limits. Syntheticmathunits verify unrelatedpole atalpha=-1 doesn't
break validalpha=1 and equivalentfractions acrossdifferentrealextensions.
No syntheticfixture isclaimed asfullgeometryqualification.

New event_nodes/port_order.rs consumeswholeOriginalContactAtlas pointgroup and
allorderedsourceparamgroups. Perparam deriveexistingincoming/outgerms, omitonly
provenoutside sides. Leadingquarter + exactY/X slope comparison givesCCW order
(noatan2/sample). SourceCurve/selfparamrefs retained, sourceCensus replayfresh.
EQUALleadingrays explicitlyREFUSEpendingintrinsic tangentorder. Portlimit/
comparisonlimit failures don'tpublishpartialports. Cachedreceiptcomparesmetadata,
ray/cell/ref AND actualbranchowner.sources+census toAtlasfreshbuilder; otherwise
translatedunrelatedbranches withidenticalrays coulddetach. Privateaudit verifies
that genuineidenticalray translatedsource substitution refuses.

Tests:6ports ineach tripleintersection,4ports atselfintersection with2distinct
naturalparams,1port atoriginalstationaryendpoint m2. Rational higherdegreegerms,
sourceknots corners andstationarym3/cuspm2 explicit. Tangentparabolapair touching
atirrationalroots andcusp SAMEleadingrays refuse (notfinished!). Optionalbranch
nearmaxheight y=3.next_down withwide1e-3rootboxes: zero signrefinements yieldsErr,
neverNone;192refinements succeeds. Fullproofsource/caches/ordering/budgets audited.

Alljobs terminal/consumed: finalregions92957, WASM73641; profiles58980 terminal76
passed (afterruntimecodefreeze, beforeonlyadditionalintegrationtestfixture).
Earlier64364,95030,7029terminal. Final146nativepass0failed0ignored: profile76,
16integrationbinaries70. WASMpass, map186check, explicit13newfilefmtpass. All16
changedRusthashes stable acrossfinalintegration/WASMgates. Prior72candidate and
1647main historical/overlap; doNOTsum. Currentlogs/native-tests/patch authoritative.

NEXT ACTUAL TANGENT ORDER PATH (avoidsame-delta/Puiseux shortcut):
Use a COMMON EXACT RATIONAL LONGITUDINAL SLICE nearcontact, independentofsource
parameterization. Forports withsameleadingray chooseaxis whereleadingcomponent
nonzero andits sign. Eachoriginalgerm canbeproved localstrictmonotone graph of
thatcoordinate using fullscalar derivative critical-rootcensus and actualfirst
nonzerojet. Cutlocal parameter BEFORE nearestotherAtlas orderedcontact AND
before othercoordinatecriticalroot, withinactualselectedsourcecell. Allcuts and
sourceevaluations Q, no floatCurve extraction. This providescontact-free branch
neighbourhoods fromexactsourcecensus, not sample residual.

Detailed construction: letcontactnaturalparameter P(alpha), originalcell [a,b],
alphaactualisolatingroot. Forout direction startcut=b, repeatedly choosecut
betweenupperexactPbound andpreviouscut; forin startcut=a betweenlowerPbound and
previouscut. Refinealpha simultaneously asneeded; clipPbox toactualcelldomain.
Checkcut precedes next/previousAtlas sourceparamgroup via exactsign of neighbour
P(beta)-cut (ornewparameter comparison); nootherfullsourcecontact beforecut.
Foraxis derivative g=X'W-XW' (actualexactpowercoeffs), provecompleteclosed root
count on rationalhull(contactPbox,cut) is0 ifg(P)!=0 or1 ifg(P)=0. In lattercase
selectedrootGCD provesonly root iscontact. Require g(cut)sign=leadingAxisSign*
direction. Ifneeded shrinkcut/refinebox toexcludeother scalarcriticalroots.
Thenbranchcoordinate isstrictmonotone alltheway fromcontact tocut (endpointzero
jetsallowed). Closedsource/knot incoming cellsalready handledbyOwnedBranch.

EvaluateoriginalC(cut) EXACTQ. Refinecontactcoordinate rationalfunctionbounds
untilchooseqx strictly BETWEENcontact andBOTH branchcut axiscoordinates, with
correctaxis sign; qxQ maynonbinary. Eachbranch hasunique originalsource scalar
root X(u)-qxW(u)=0 beforecut; isolatecomplete roots, selectby exactsourceparam
bounds/ordering, no roundedinverse. EvaluatetransverseY atthatroot as rational
algebraicvalue; compare viaNEWfractionorder. Pair fullAtlas/contact-free cut
provesnootherintersection betweencontact andslice => transverseorder intrinsic
andstable tocontact. IfslicecompareEqual, refuseoverlap/incompleteproof; don't
mergeports. No sourceparameter speeds/Puiseuxroot extension needed.

Order orientation: iflongaxisX, transverseY comparison normal forpositiveX,
REVERSE fornegativeX. IflongaxisY, transverseX comparison REVERSE forpositiveY,
normal fornegativeY. At+Xleadingray cycliccut, branchesabove/below maystraddle
0/2pi: intrinsicgrouporder below→axis→above iscyclic-valid butcanonicalpositiveX
start shouldrotate below+Xprefix toend. Determine transverse firstnonzero germ
sign EXACTLY (continueoriginaljet component pastleadingvectororder); yident0
meansactualhorizontalbranch. Otherquadrantboundary groups canstaylocalordered.
Currentdistinct-leadingray implementation iscyclicvalid, tangenttie phase must
handlethiscut/rotation, no fakeabsolute angle.

Afterintrinsicportorder: addordinarysourceendpoints, ownedatomiccarrier cuts,
weightedDCELfaceboundary selection usingexactoriginalpointwinding, overlapbranches,
source-boundregion/physicalface topology. N0030 automaticmixedrootatlas and
ownedphysicalbranch/source trimtopology also stillopen. Goalnotcomplete.
