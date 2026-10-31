import json,subprocess,time,sys,statistics
from pathlib import Path
c=json.loads(Path('work/render-max/test-state.json').read_text())
ctl=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/horizon-ctl'
out=Path('outputs/horizon-render-max').resolve()
def call(*args):
 r=subprocess.run([str(ctl),'--socket',c['socket'],'--timeout','30',*args],capture_output=True,text=True)
 if r.returncode:raise RuntimeError((args,r.stdout,r.stderr))
 result=json.loads(r.stdout)
 if 'error' in result:raise RuntimeError((args,result))
 return result
def command(text):return call('command',text)
def shot(name):
 time.sleep(1);return call('screenshot',str(out/(name+'.png')))
out.mkdir(exist_ok=True)
label=sys.argv[1]
deadline=time.monotonic()+45
while not Path(c['socket']).exists():
 if time.monotonic()>deadline:raise RuntimeError('No test socket')
 time.sleep(.2)
call('wait-ready');call('options-set','pauseOnLostFocus','false');call('options-set','chatVisibility','2')
if call('ui')['screen']=='paused':call('click','back-to-game')
command('gamerule minecraft:advance_time false');command('weather clear');command('time set 8000')
command('tp @s -155.083 -11.80064 -165.083 98.3 -0.8')
time.sleep(10);shot(label+'-cave')
samples=[]
for _ in range(20):
 state=call('wait-ready');samples.append(state['fps']);time.sleep(.5)
(out/(label+'-performance.json')).write_text(json.dumps({'fps':statistics.median(samples),'samples':samples,'renderer':state['renderer']},indent=2))
import shutil
shutil.copy2(Path(c['root'])/'process.log',out/(label+'-runtime.log'))
print('fps',statistics.median(samples))
command('fill -164 -17 -174 -146 -5 -156 minecraft:stone')
command('fill -163 -16 -173 -147 -6 -157 minecraft:air')
command('tp @s -155 -15 -165 90 -5')
time.sleep(4);shot(label+'-enclosed')
command('fill -156 -6 -166 -154 -5 -164 minecraft:air')
time.sleep(4);shot(label+'-aperture')

call('quit')
