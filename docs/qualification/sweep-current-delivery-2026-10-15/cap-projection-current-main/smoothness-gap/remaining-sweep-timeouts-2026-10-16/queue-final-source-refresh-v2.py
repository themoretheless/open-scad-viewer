from pathlib import Path
import json,time,subprocess,hashlib
root=Path.cwd();out=Path(__file__).resolve().parent;sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
while json.loads((out/'original-five-finalnorm-focus-receipt.json').read_text())['state'] not in ['complete','failed']:time.sleep(5)
script=(out/'build-main-final-source-refresh-v2-staged.py').read_text();exec(script[script.index('def sources():'):script.index('before=sources();')]);previous=sources();stable=time.monotonic()
while time.monotonic()-stable<60:
 time.sleep(10);current=sources()
 if current!=previous:previous=current;stable=time.monotonic()
print('Native source quiet; rebuilding latest main without changing live assets.',flush=True)
result=subprocess.run(['python3',str(out/'build-main-final-source-refresh-v2-staged.py')]);raise SystemExit(result.returncode)
