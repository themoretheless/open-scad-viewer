from pathlib import Path
import time,subprocess,json
out=Path(__file__).resolve().parent
while not (out/'finalnorm-main-original31.json').exists():time.sleep(5)
j=json.loads((out/'finalnorm-main-original31.json').read_text());print('Focused31 result:',j['numPassedTests'],j['numFailedTests'],flush=True)
result=subprocess.run(['python3',str(out/'build-main-final-source-refresh-staged.py')]);raise SystemExit(result.returncode)
