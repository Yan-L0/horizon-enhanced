#!/usr/bin/env python3
"""Apply this private Upgrader port to the exact pristine Horizon source revision."""
from pathlib import Path
import argparse, json, hashlib
p=argparse.ArgumentParser();p.add_argument('source',type=Path);a=p.parse_args()
root=a.source.resolve();here=Path(__file__).resolve().parent
marker=root/'.upgrader-port'
if marker.exists():
 print('Already patched; updating mod module only')
else:
 for rel,digest in json.loads((here/"expected_sources.json").read_text()).items():
  if hashlib.sha256((root/rel).read_bytes()).hexdigest()!=digest: raise SystemExit("Wrong Horizon source revision: "+rel)
 edits={
 'crates/horizon-client/src/game.rs':[
 ('mod native_hooks;', 'mod native_hooks;\nmod upgrader;'),
 ('fn hud(&mut self, size: [u32; 2]) -> HudBatch {','fn hud(&mut self, size: [u32; 2]) -> HudBatch {\n        if let Some(batch) = upgrader::draw(self, size) { return batch; }'),
 ('fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, mut event: WindowEvent) {','fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, mut event: WindowEvent) {\n        if self.window.as_ref().is_some_and(|w| w.id() == id) && upgrader::event(self, &event) { return; }'),
 ],
 'crates/horizon-client/src/game/ui_control.rs':[
 ('pub(super) fn ui_nodes(&self) -> Vec<Node> {','pub(super) fn ui_nodes(&self) -> Vec<Node> {\n        if let Some(nodes) = super::upgrader::nodes() { return nodes; }')],
 'crates/horizon-client/src/game/control.rs':[
 ("// A server's dialog, in configuration too.","if super::upgrader::typed(self, text) { return ok(); }\n                // A server's dialog, in configuration too.")],
 'crates/horizon-client/src/game/lan_actions.rs':[
 ('pub(super) fn lan_guest_packet(&mut self, host: &mut LanHost, conn: u32, packet: S) {','pub(super) fn lan_guest_packet(&mut self, host: &mut LanHost, conn: u32, packet: S) {\n        if super::upgrader::guest(self, host, conn, &packet) { return; }')],
 'crates/horizon-client/src/game/remote.rs':[
 ('fn handle_packet(&mut self, packet: C) -> Result<()> {','fn handle_packet(&mut self, packet: C) -> Result<()> {\n        if super::upgrader::received(&packet) { return Ok(()); }')],
 'crates/horizon-client/src/game/native_hooks.rs':[
 ('pub(super) fn native_command(&mut self, command: &str, ran: &mut bool) {','pub(super) fn native_command(&mut self, command: &str, ran: &mut bool) {\n        if super::upgrader::command(self, command) { *ran = true; }'),
 ('pub(super) fn native_mods(&mut self, mods: &mut Vec<NativeMod>) {','pub(super) fn native_mods(&mut self, mods: &mut Vec<NativeMod>) {\n        mods.push(NativeMod { id: "upgrader".into(), title: "Upgrader 0.1.0 (Rust port)".into(), description: "Item upgrade station. Use the crafted item or /upgrader.".into(), keys: vec![] });'),
 ('pub(super) fn native_settings(&mut self, mod_id: &str, buttons: &mut Vec<String>) {','pub(super) fn native_settings(&mut self, mod_id: &str, buttons: &mut Vec<String>) {\n        if mod_id == "upgrader" { buttons.push("Открыть настройки Upgrader".into()); }'),
 ('pub(super) fn native_settings_click(&mut self, mod_id: &str, button: usize) {','pub(super) fn native_settings_click(&mut self, mod_id: &str, button: usize) {\n        if mod_id == "upgrader" && button == 0 { super::upgrader::settings_open(self); }')]
 }
 for rel,replacements in edits.items():
  f=root/rel;s=f.read_text()
  for old,new in replacements:
   if old not in s: raise SystemExit(f'Expected source not found: {rel}: {old}')
   s=s.replace(old,new,1)
  f.write_text(s)
 marker.write_text('1da354b67a509812c4141b67de2affd6de65c342\n')
core=(here/'src/core.rs').read_text();engine=(here/'src/engine.rs').read_text()
(root/'crates/horizon-client/src/game/upgrader.rs').write_text('pub(crate) mod core {\n'+core+'\n}\n'+engine)
print('Upgrader source ready:',root)
