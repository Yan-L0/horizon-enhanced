import json,subprocess,time,sys,statistics
from pathlib import Path
c=json.loads(Path('work/render-bugs/test-state.json').read_text())
ctl=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/horizon-ctl'
out=Path('outputs/horizon-render-optimized').resolve()
def call(*args):
 r=subprocess.run([str(ctl),'--socket',c['socket'],'--timeout','30',*args],capture_output=True,text=True)
 if r.returncode:raise RuntimeError((args,r.stdout,r.stderr))
 result=json.loads(r.stdout)
 if 'error' in result:raise RuntimeError((args,result))
 return result
def command(text):return call('command',text)
def shot(name):
 time.sleep(1);return call('screenshot',str(out/(name+'.png')))
deadline=time.monotonic()+30
while not Path(c['socket']).exists():
 if time.monotonic()>deadline:raise RuntimeError('No test control socket')
 time.sleep(.2)
ready=call('wait-ready');call('options-set','pauseOnLostFocus','false')
call('options-set','chatVisibility','2')
if call('ui')['screen']=='paused':call('click','back-to-game')
label=sys.argv[1] if len(sys.argv)>1 else 'test'
command('gamerule minecraft:advance_time false');command('weather clear')
for cmd in [
 'fill -24 94 -24 24 94 24 minecraft:stone',
 'fill -24 95 -24 24 95 24 minecraft:grass_block',
 'fill -24 96 -24 24 108 24 minecraft:air',
 'fill -18 95 -8 -3 95 12 minecraft:water',
 'fill 4 96 -4 6 103 -2 minecraft:stone_bricks',
 'fill 10 96 1 12 101 3 minecraft:oak_log',
 'fill 8 102 -1 14 104 5 minecraft:oak_leaves',
 'fill -1 96 10 6 100 10 minecraft:red_stained_glass',
 'setblock 0 96 -5 minecraft:sea_lantern',
 'setblock 0 97 -5 minecraft:torch',
 'setblock -6 96 -12 minecraft:poppy',
 'setblock -4 96 -12 minecraft:dandelion',
 'setblock -2 96 -12 minecraft:short_grass',
 'setblock 0 96 -12 minecraft:fern',
 'setblock 2 96 -12 minecraft:dead_bush',
 'setblock 4 95 -12 minecraft:netherrack',
 'setblock 4 96 -12 minecraft:fire',
 'setblock 0 96 -13 minecraft:stone',
 'tp @s -10 96 -20 0 0',
]:command(cmd)
call('look-at','-3','96.8','-12');call('wait','10')
command('time set 8000');shot(label+'-plants')
time.sleep(12)
samples=[]
for i in range(30):
 state=call('wait-ready');samples.append(state.get('fps'));time.sleep(.5)
result={'label':label,'fps_median':statistics.median(samples),'fps_samples':samples,'renderer':state.get('renderer'),'pack':c['pack']}
(out/(label+'-performance.json')).write_text(json.dumps(result,indent=2));print(json.dumps(result),flush=True)

import shutil
shutil.copy2(Path(c['root'])/'process.log',out/(label+'-runtime.log'))
call('quit')
