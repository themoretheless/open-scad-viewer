// Installed in the page before workers are created; Playwright serializes this function.
export function installSweepWorkerProbe(){
    const NativeWorker=window.Worker
    window.sweepHeld=[];window.sweepTerminated=[];window.sweepTerminateCalls=[];window.sweepHoldBuild=false;window.sweepHoldExact=false
    window.Worker=class extends NativeWorker {
     postMessage(message,...args){
      const mode=message?.type==='build'?'build':message?.kind==='exact-solid'?'exact':null
      if(mode)this.sweepNativeMode=mode
      if(mode&&(mode==='build'?window.sweepHoldBuild:window.sweepHoldExact)){
       this.sweepMode=mode;window.sweepHeld.push(mode);return
      }
      return super.postMessage(message,...args)
     }
     terminate(){window.sweepTerminateCalls.push({mode:this.sweepNativeMode,atEpochMs:Date.now()});if(this.sweepMode)window.sweepTerminated.push(this.sweepMode);return super.terminate()}
    }
   }
