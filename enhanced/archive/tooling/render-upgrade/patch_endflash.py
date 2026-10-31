from pathlib import Path
import shutil
root=Path('work/upgrader-port/horizon-1da354b67a509812c4141b67de2affd6de65c342')
p=root/'crates/horizon-client/src/game/end.rs';backup=Path('work/render-upgrade/before/crates/horizon-client/src/game/end.rs');backup.parent.mkdir(parents=True,exist_ok=True)
if not backup.exists():shutil.copy2(p,backup)
s=p.read_text().replace('''    /// SkyRenderer.renderEndFlash, drawn after the renderer's resident sky mesh.
''','''    /// Shader-pack flash position in world axes, at the same distance/pose as the
    /// sky sprite. The renderer rotates it into view space with the other lights.
    pub(super) fn shader_end_flash(&self, partial: f32) -> (f32, [f32; 3]) {
        if self.dim != Dim::End {
            return (0.0, [0.0, 100.0, 0.0]);
        }
        let flash = &self.end.flash;
        let pose = glam::Mat4::from_rotation_y((180.0 - flash.y_angle).to_radians())
            * glam::Mat4::from_rotation_x((-90.0 - flash.x_angle).to_radians());
        let intensity = if self.settings.file.bool("hideLightningFlashes") {
            0.0
        } else {
            flash.intensity(partial)
        };
        (intensity, pose.transform_vector3(glam::Vec3::new(0.0, 100.0, 0.0)).to_array())
    }

    /// SkyRenderer.renderEndFlash, drawn after the renderer's resident sky mesh.
''',1);p.write_text(s)
p=root/'crates/horizon-client/src/game/shader_packs.rs';s=p.read_text().replace('''            right_handed: !self.settings.main_hand_left,
''','''            right_handed: !self.settings.main_hand_left,
            end_flash: self.shader_end_flash(partial),
''',1);p.write_text(s)
p=root/'crates/horizon-client-render/src/gfx/pack_uniforms.rs';s=p.read_text().replace('''    pub right_handed: bool,
''','''    pub right_handed: bool,
    /// Intensity and world-space position (distance 100) of the End sky flash.
    pub end_flash: (f32, [f32; 3]),
''',1).replace('''    "isRightHanded",
''','''    "isRightHanded",
    "endFlashIntensity",
    "endFlashPosition",
''',1).replace('''                "isRightHanded" => Value::i(&[i32::from(inputs.right_handed)]),
''','''                "isRightHanded" => Value::i(&[i32::from(inputs.right_handed)]),
                "endFlashIntensity" => Value::f(&[inputs.end_flash.0]),
                "endFlashPosition" => v3(to_view(Vec3::from_array(inputs.end_flash.1))),
''',1).replace('''        | "relativeEyePosition"
        | "upPosition"''','''        | "relativeEyePosition"
        | "endFlashPosition"
        | "upPosition"''',1).replace('''        assert_eq!(components("relativeEyePosition"), 3);''','''        assert_eq!(components("relativeEyePosition"), 3);
        assert_eq!(components("endFlashPosition"), 3);''');p.write_text(s)
