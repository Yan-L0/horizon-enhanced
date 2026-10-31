from pathlib import Path
exec(Path('work/render-bugs/fixture.py').read_text().split('deadline=time.monotonic()')[0])
label=sys.argv[1] if len(sys.argv)>1 else 'fixed'
command('gamemode survival');command('clear');command('setblock 0 96 -13 minecraft:oak_log');command('tp @s 0 96 -16 0 0');call('wait','5');call('look-at-block','0','96','-13');call('wait','5')
shot(label+'-outline')
call('mouse','left','down');call('wait','10')
progress=call('state');assert progress.get('mining') and progress['mining']['progress'] > 0,progress
shot(label+'-cracks');call('mouse','left','up')
command('gamemode creative');command('fill -48 94 -24 -25 94 24 minecraft:stone');command('fill -48 95 -24 -25 95 24 minecraft:grass_block');command('tp @s -8 96 -18 0 0');call('look-at','-8','96','3');call('wait','5')
positions=[]
call('key','d','down')
for i in range(3):
 call('screenshot',str((out/(label+'-moving-'+str(i)+'.png')).resolve()));time.sleep(.1);positions.append(call('wait-ready').get('player'))
call('key','d','up')
(out/(label+'-movement.json')).write_text(json.dumps({'positions':positions,'mining_snapshot':progress.get('mining'),'initial_player':progress.get('player')},indent=2))
command('execute in minecraft:the_end run tp @s -10 82 -12')
for i in range(80):
 state=call('wait-ready')
 if state.get('dimension')=='minecraft:the_end':break
 time.sleep(.2)
else:raise RuntimeError('End transition failed')
for s in ['fill -20 79 -20 60 79 20 minecraft:end_stone','fill -14 80 -16 -6 88 -8 minecraft:air','fill 10 80 0 12 87 2 minecraft:end_stone','fill 30 80 0 32 87 2 minecraft:end_stone','fill 50 80 0 52 87 2 minecraft:end_stone']:
 command(s)
call('look-at','30','82','1');call('wait','10');shot(label+'-end')
log=(Path(c['root'])/'process.log').read_text();(out/(label+'-runtime.log')).write_text(log)
print(json.dumps({'label':label,'movement_positions':positions,'end':state.get('dimension'),'mining':progress.get('mining')}),flush=True)
