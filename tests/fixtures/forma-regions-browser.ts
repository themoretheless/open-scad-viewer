import {createApp,h,reactive} from 'svet'
import FormaRegion from '../../src/components/FormaRegion.svt'
import topbar from '../../src/ui/Topbar.ui?raw'
import toolbar from '../../src/ui/EditorToolbar.ui?raw'
import viewer from '../../src/ui/ViewerToolbar.ui?raw'
import selection from '../../src/ui/SelectionModes.ui?raw'
import dock from '../../src/ui/DockRail.ui?raw'
import status from '../../src/ui/Statusbar.ui?raw'
import stats from '../../src/ui/EditorStats.ui?raw'
import compute from '../../src/ui/ComputeInspector.ui?raw'
import workspace from '../../src/ui/WorkspaceActionBar.ui?raw'
import '../../src/styles/app-shell.css'
const style=document.createElement('style');style.textContent=`
:root{--bg:#1e1d1b;--surface:#292825;--surface-raised:#34332f;--text:#eeeae2;--text-dim:#aaa69d;--accent:#f3a544;--hairline:#47453e;--border:#5a574e;--hover:#39372f;--danger:#ff8585;--radius:5px;--font-ui:system-ui;--font-mono:monospace;--topbar-h:48px;--statusbar-h:30px}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:13px system-ui}.fixture-stage{position:relative;min-height:300px;background:linear-gradient(145deg,#252520,#191916);border-block:1px solid var(--hairline)}.fixture-label{position:absolute;inset:48% 0 auto;text-align:center;color:var(--text-dim)}.fixture-tools{display:flex;flex-wrap:wrap;gap:12px;padding:12px}.fixture-tools>.compute-inspector{flex:1;min-width:240px}.fixture-tools .dock-rail{position:relative;inset:auto;width:auto;min-height:180px}.fixture-stage .selection-hud{white-space:normal}
`;document.head.append(style)
const state=reactive({cubeLabel:'Cube',cylinderLabel:'Cylinder',sphereLabel:'Sphere',booleanLabel:'Difference',workspaceMode:'solid',modes:[{id:'solid',label:'Solid'},{id:'mesh',label:'Mesh'},{id:'code',label:'Code'}],workspaceModeGroupLabel:'Workspace',editorOpen:false,sourceLabel:'Source',commandsLabel:'Commands',commandHelp:'Commands',exportLabel:'Export',exportTitle:'Export model',workspaceConflict:false,persistenceError:false,
 rendering:false,renderLabel:'Build',cancelBuildLabel:'Cancel build',autoRender:true,autoLabel:'Auto',solidBuilding:false,
 toolbarClass:'viewer-toolbar',fitLabel:'Fit',resetLabel:'Reset',canPreviousView:false,previousViewLabel:'Previous view',previousViewTitle:'Previous view',isOrthographic:false,projectionLabel:'Perspective',projectionTitle:'Projection',gridVisible:true,gridLabel:'Grid',displayLabel:'Display',displayMode:'shaded',shadedLabel:'Shaded',edgesLabel:'Edges',xrayLabel:'X-ray',
 selectionModeLabel:'Selection',selectionMode:'face',pointLabel:'Point',pointTitle:'Point · 1',faceLabel:'Face',faceTitle:'Face · 3',bodyLabel:'Body',bodyTitle:'Body · 4',showHud:true,hasSelectedMesh:true,selectedSummary:'Selected: fixture view model',focusLabel:'Focus',isolateLabel:'Isolate',isolateTitle:'Isolate',isolated:false,deselectLabel:'Deselect',
 sidebarLabel:'Sidebar',dockTab:'scene',tabs:[{id:'scene',domId:'dock-tab-scene',label:'Scene',iconPath:'M4 4h16v16H4z'},{id:'inspector',domId:'dock-tab-inspector',label:'Inspector',iconPath:'M4 8h16M4 16h16'}],closeSidebarLabel:'Close sidebar',
 meshesLabel:'Meshes',meshesValue:'1',trianglesLabel:'Triangles',trianglesValue:'12',hasMeshes:true,volumeLabel:'Volume',volumeValue:'8',areaLabel:'Area',areaValue:'24',statusClass:'status',statusSummary:'Fixture state',
 panelTitle:'Geometry computation',description:'Finite UI view-model fixture',canRunCompute:true,computeButtonLabel:'Compute',computeBusy:false,cancelLabel:'Cancel',hasResult:false,areaPrefix:'Area: ',backendSummary:'CPU fixture',timingSummary:'',showFallback:false,fallbackNote:'Fallback state',verificationNote:'',computeError:'',
 statusAriaLabel:'Status',gpuDotClass:'off',backendLabel:'CPU',showSortedTransparency:false,showPersistence:true,persistenceLabel:'Saved',conversionPending:false,conversionLabel:'Exporting',conversionMax:2,conversionValue:0,hintLabel:'Orbit: drag'})
const events:{name:string,args:unknown[]}[]=[]
function action(name:string,args:unknown[]){events.push({name,args});switch(name){case'openWorkspaceMode':state.workspaceMode=String(args[0]);break;case'toggleEditor':state.editorOpen=!state.editorOpen;break;case'renderFull':state.rendering=true;break;case'cancelGeometryBuild':state.rendering=false;break;case'setDisplayMode':state.displayMode=String(args[0]);break;case'setSelectionMode':state.selectionMode=String(args[0]);break;case'toggleIsolate':state.isolated=!state.isolated;break;case'selectDockTab':state.dockTab=String(args[0]);break;case'runGeometryAnalysis':state.computeBusy=true;state.canRunCompute=false;break;case'cancelGeometryAnalysis':state.computeBusy=false;state.canRunCompute=true;break}}
const region=(source:string,slots?:Record<string,()=>unknown>)=>h(FormaRegion,{source,state,onAction:action,onBinding:(key:string,value:unknown)=>{if(key==='autoRender')state.autoRender=Boolean(value)}},slots)
createApp({render:()=>h('div',{},[
 h('nav',{class:'topbar'},region(topbar,{fileMenu:()=>h('details',{class:'menu file-menu'},h('summary',{'aria-label':'File',class:'file-chip'},'model.scad')),settingsMenu:()=>h('button',{'aria-label':'Settings',class:'icon-btn'},'⚙')})),
 region(toolbar,{vr:()=>h('span',{},'VR'),functionReference:()=>h('button',{'aria-label':'Reference',class:'icon-btn'},'?'),moreMenu:()=>h('button',{'aria-label':'More',class:'icon-btn'},'⋯')}),
 h('section',{class:'fixture-stage'},[region(viewer,{gridAndMaterial:()=>h('button',{'aria-label':'Material',class:'btn view-btn'},'Material')}),region(selection),h('div',{class:'fixture-label'},'UI contract viewport')]),
 region(stats),h('section',{class:'fixture-tools'},[region(dock),region(compute),region(workspace)]),region(status)
])}).mount('#app')
Object.assign(window,{__formaFixture:{state,events}})
