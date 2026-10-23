from pathlib import Path
source = Path(__file__).resolve().parent.parent / 'qualify-template.py'
exec(compile(source.read_text(), str(source), 'exec'))
