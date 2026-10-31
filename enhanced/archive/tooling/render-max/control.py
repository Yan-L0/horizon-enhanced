import subprocess,json,sys
from pathlib import Path
c=json.loads(Path('work/render-max/test-state.json').read_text());ctl=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/horizon-ctl'
r=subprocess.run([str(ctl),'--socket',c['socket'],'--timeout','30',*sys.argv[1:]],capture_output=True,text=True)
print(r.stdout or r.stderr,end='');raise SystemExit(r.returncode)
