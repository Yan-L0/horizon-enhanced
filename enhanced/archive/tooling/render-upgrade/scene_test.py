import json,subprocess,time
from pathlib import Path
c=json.loads(Path('work/render-upgrade/test-state.json').read_text())
ctl=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/horizon-ctl'
out=Path('outputs/horizon-render-upgrade').resolve()
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
 'tp @s -16 102 -18 0 0',
]:command(cmd)
call('look-at','0','98','2');call('wait','10')
assert call('ui')['screen']=='playing',call('ui')
call('key','f1','tap')
command('time set 6000');shot('noon')
command('time set 12000');shot('low-sun')
command('time set 18000');shot('night')
command('time set 6000');command('tp @s -12 93 -3 0 0')
# A two-block deep pocket gives the camera an actual water volume.
command('fill -16 92 -6 -8 95 4 minecraft:water');command('tp @s -12 93 -3 0 0');shot('underwater')
command('tp @s -16 102 -18 0 0');call('look-at','0','98','2');call('wait','10')
state=call('wait-ready');ui=call('ui')
log=(Path(c['root'])/'game.log').read_text()
assert 'terrain casts shadows with shadow' in log
assert 'translucent terrain draws with gbuffers_water' in log
assert 'texture.noise:' not in log,log
assert 'turned off:' not in log and 'panicked at' not in log,log
result={'renderer':state.get('renderer'),'fps_snapshot':state.get('fps'),'post_programs':9,'scene':['noon','low-sun','night','underwater'],'full_shader_loaded':True,'log_path':str(Path(c['root'])/'game.log')}
(out/'validation-scene.json').write_text(json.dumps(result,indent=2));(out/'runtime.log').write_text(log)
print(json.dumps(result),flush=True)
