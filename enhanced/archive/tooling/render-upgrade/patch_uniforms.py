from pathlib import Path
import shutil
root=Path('work/upgrader-port/horizon-1da354b67a509812c4141b67de2affd6de65c342')
def backup(rel):
 p=root/rel;b=Path('work/render-upgrade/before')/rel;b.parent.mkdir(parents=True,exist_ok=True)
 if not b.exists():shutil.copy2(p,b)
 return p
p=backup('crates/horizon-client-render/src/gfx/pack_uniforms.rs');s=p.read_text()
s=s.replace('    pub held_light: [i32; 2],','''    pub held_light: [i32; 2],
    /// Camera minus the player head position, in world axes (third-person included).
    pub relative_eye: [f32; 3],
    pub cloud_height: f32,
    pub bedrock_level: i32,
    pub right_handed: bool,''')
s=s.replace('    "previousCameraPosition",','''    "previousCameraPosition",
    "cameraPositionFract",
    "cameraPositionInt",
    "previousCameraPositionFract",
    "previousCameraPositionInt",
    "relativeEyePosition",
    "cloudHeight",
    "bedrockLevel",
    "isRightHanded",''',1)
s=s.replace('previous: Option<(Vec3, Mat4, Mat4)>','previous: Option<(DVec3, Mat4, Mat4)>')
s=s.replace('''        let (previous_camera, previous_view, previous_projection) =
            self.previous
                .unwrap_or((Vec3::ZERO, Mat4::ZERO, Mat4::ZERO));
        self.previous = Some((camera, view, projection));''','''        let (previous_camera, previous_view, previous_projection) =
            self.advance_camera(frame.camera, view, projection);''')
s=s.replace('"previousCameraPosition" => v3(previous_camera),','''"previousCameraPosition" => v3(previous_camera.as_vec3()),
                "cameraPositionFract" => v3(camera_fraction(frame.camera)),
                "cameraPositionInt" => Value::i(&frame.camera.floor().as_ivec3().to_array()),
                "previousCameraPositionFract" => v3(camera_fraction(previous_camera)),
                "previousCameraPositionInt" => Value::i(&previous_camera.floor().as_ivec3().to_array()),
                "relativeEyePosition" => Value::f(&inputs.relative_eye),
                "cloudHeight" => Value::f(&[inputs.cloud_height]),
                "bedrockLevel" => Value::i(&[inputs.bedrock_level]),
                "isRightHanded" => Value::i(&[i32::from(inputs.right_handed)]),''')
s=s.replace('''impl FrameUniforms {
    /// This frame''','''/// Split in f64 before casting: negative coordinates must have a fraction in [0, 1),
/// and sub-block precision must survive beyond f32's exact integer range.
fn camera_fraction(camera: DVec3) -> Vec3 {
    (camera - camera.floor()).as_vec3()
}

impl FrameUniforms {
    /// On the first frame history is the current camera, never a zero matrix.
    /// Invalid history would make temporal reprojection collapse toward the screen centre.
    fn advance_camera(&mut self, camera: DVec3, view: Mat4, projection: Mat4) -> (DVec3, Mat4, Mat4) {
        self.previous.replace((camera, view, projection)).unwrap_or((camera, view, projection))
    }

    /// This frame''',1)
s=s.replace('''        | "previousCameraPosition"
        | "upPosition"''','''        | "previousCameraPosition"
        | "cameraPositionFract"
        | "cameraPositionInt"
        | "previousCameraPositionFract"
        | "previousCameraPositionInt"
        | "relativeEyePosition"
        | "upPosition"''')
s=s.replace('''    #[test]
    fn values_follow''','''    #[test]
    fn camera_fraction_survives_negative_and_large_coordinates() {
        assert_eq!(camera_fraction(DVec3::new(-0.25, -64.75, 30_000_000.125)), Vec3::new(0.75, 0.25, 0.125));
        assert_eq!(components("cameraPositionFract"), 3);
        assert_eq!(components("relativeEyePosition"), 3);
    }

    #[test]
    fn temporal_history_starts_at_the_current_camera() {
        let mut uniforms = FrameUniforms::default();
        let camera = DVec3::new(30_000_000.125, 80.5, -0.75);
        let view = Mat4::from_rotation_y(0.8);
        let projection = Mat4::perspective_rh_gl(1.1, 1.5, 0.05, 512.0);
        assert_eq!(uniforms.advance_camera(camera, view, projection), (camera, view, projection));
        assert_eq!(uniforms.advance_camera(camera + DVec3::X, Mat4::IDENTITY, Mat4::IDENTITY), (camera, view, projection));
    }

    #[test]
    fn values_follow''',1)
p.write_text(s)
p=backup('crates/horizon-client/src/game/shader_packs.rs');s=p.read_text().replace('''            held_items,
            held_light,
        }''','''            held_items,
            held_light,
            relative_eye: self.eye_position().map_or([0.0; 3], |eye| {
                (camera - glam::DVec3::from_array(eye)).as_vec3().to_array()
            }),
            cloud_height: 192.33,
            bedrock_level: self.world.bottom_y(),
            right_handed: !self.settings.main_hand_left,
        }''',1);p.write_text(s)
