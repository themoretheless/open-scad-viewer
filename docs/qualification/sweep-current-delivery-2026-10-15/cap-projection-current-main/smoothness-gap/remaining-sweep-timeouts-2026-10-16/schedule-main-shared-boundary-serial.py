"""Finish accidental focused-run overlap, then qualify unchanged actual-main artifact serially."""
from pathlib import Path
import json,os,signal,subprocess,time
out=Path(__file__).resolve().parent
while True:
 main=json.loads((out/'main-shared-boundary-bernstein31-receipt.json').read_text())
 owned_report=out/'shared-boundary-bernstein-remaining-public.json'
 if main['state'] in ['complete','failed'] and owned_report.exists():break
 time.sleep(5)
owned=json.loads(owned_report.read_text());assert owned['numFailedTests']>0,'Do not resume a legacy pipeline which would launch duplicate follow-up matrices'
r=json.loads((out/'owned-public-scheduling.json').read_text());pid=r['driverPid'];assert 'validate-shared-boundary-bernstein-artifact.py' in subprocess.check_output(['ps','-p',str(pid),'-o','command='],text=True)
os.kill(pid,signal.SIGCONT);r['state']='resumed-after-focused-test;failed-gate-will-stop-followups';(out/'owned-public-scheduling.json').write_text(json.dumps(r,indent=2)+'\n')
while json.loads((out/'shared-boundary-bernstein-artifact-validation.json').read_text())['state']=='running':time.sleep(5)
print('Both prior focused runs finished; launching one unchanged current-main matrix.',flush=True)
result=subprocess.run(['python3',str(out/'test-main-shared-boundary-bernstein31-serial.py')],cwd=Path.cwd());raise SystemExit(result.returncode)
