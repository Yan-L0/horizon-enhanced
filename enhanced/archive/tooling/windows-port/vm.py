import sys,base64,subprocess
from pathlib import Path
code="$ProgressPreference='SilentlyContinue'\n"+Path(sys.argv[1]).read_text()
encoded=base64.b64encode(code.encode('utf-16le')).decode()
r=subprocess.run(['/usr/local/bin/prlctl','exec','<VM_UUID>','--current-user','powershell.exe','-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand',encoded],capture_output=True,timeout=180)
sys.stdout.buffer.write(r.stdout);sys.stderr.buffer.write(r.stderr);sys.exit(r.returncode)
