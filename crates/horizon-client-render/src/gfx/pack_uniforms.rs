//! The uniforms a shader pack's post-processing programs read once per frame (spec 5.2, class
//! F), from the renderer's `Frame` and the few values only the game knows (`PackInputs`).
//! Matrices follow GL's conventions (5.5): `gbufferProjection` maps view depth to clip z in
//! [-w, w], and the depth buffers packs read hold GL window depth.

use glam::{DMat4, DVec3, Mat4, Vec3, Vec4};
use horizon_shaders::Shadow;

use super::Frame;

/// What the game passes for the pack's uniforms, beyond what `Frame` carries.
#[derive(Clone, Copy, Debug, Default)]
pub struct PackInputs {
    /// `Level.getDayTime`.
    pub day_time: i64,
    /// The interpolated rain and thunder levels, 0..1.
    pub rain: f32,
    pub thunder: f32,
    /// The fluid at the camera: 0 none, 1 water, 2 lava, 3 powder snow.
    pub eye_in: i32,
    pub blindness: f32,
    /// Block and sky light at the camera, 0..15.
    pub eye_light: [i32; 2],
    pub hide_gui: bool,
    /// The brightness option, 0..1.
    pub brightness: f32,
    /// The biome at the camera (3.4): registry id, `CAT_*`, `PPT_*`, temperature, downfall.
    pub biome: i32,
    pub biome_category: i32,
    pub biome_precipitation: i32,
    pub temperature: f32,
    pub rainfall: f32,
    /// `heldItemId`, `heldItemId2` (item.properties, 5.3) and `heldBlockLightValue(2)`.
    pub held_items: [i32; 2],
    pub held_light: [i32; 2],
    /// Camera minus the player head position, in world axes (third-person included).
    pub relative_eye: [f32; 3],
    pub cloud_height: f32,
    pub bedrock_level: i32,
    pub right_handed: bool,
    /// Intensity and world-space position (distance 100) of the End sky flash.
    pub end_flash: (f32, [f32; 3]),
}

/// One uniform's value: up to 16 floats or 3 integers.
#[derive(Clone, Copy, Debug)]
pub enum Value {
    F([f32; 16]),
    I([i32; 3]),
}

impl Value {
    fn f(v: &[f32]) -> Self {
        let mut a = [0.0; 16];
        a[..v.len()].copy_from_slice(v);
        Value::F(a)
    }
    fn i(v: &[i32]) -> Self {
        let mut a = [0; 3];
        a[..v.len()].copy_from_slice(v);
        Value::I(a)
    }

    /// Writes the value as the type the pack declared, at `offset` of a std140 block.
    pub fn write(&self, ty: &str, out: &mut [u8], offset: usize) {
        let ints = matches!(ty, "int" | "bool" | "uint") || ty.starts_with(['i', 'u']);
        let (floats, integers): ([f32; 16], [i32; 16]) = match *self {
            Value::F(f) => (f, f.map(|v| v as i32)),
            Value::I(i) => {
                let mut f = [0.0; 16];
                let mut v = [0; 16];
                for k in 0..3 {
                    f[k] = i[k] as f32;
                    v[k] = i[k];
                }
                (f, v)
            }
        };
        let count = match ty {
            "mat4" => 16,
            "mat3" => {
                // std140: three columns, each padded to a vec4, from the mat4's upper left.
                for c in 0..3 {
                    let column: [f32; 3] = std::array::from_fn(|r| floats[c * 4 + r]);
                    put(out, offset + c * 16, bytemuck::cast_slice(&column));
                }
                return;
            }
            t => match t.as_bytes().last() {
                Some(n @ b'2'..=b'4') if t.contains("vec") => (n - b'0') as usize,
                _ => 1,
            },
        };
        if ints {
            put(out, offset, bytemuck::cast_slice(&integers[..count]));
        } else {
            put(out, offset, bytemuck::cast_slice(&floats[..count]));
        }
    }
}

fn put(out: &mut [u8], offset: usize, bytes: &[u8]) {
    if let Some(dst) = out.get_mut(offset..offset + bytes.len()) {
        dst.copy_from_slice(bytes);
    }
}

/// The uniforms provided, in the order of [`FrameUniforms::values`].
pub const NAMES: &[&str] = &[
    "cameraPosition",
    "previousCameraPosition",
    "cameraPositionFract",
    "cameraPositionInt",
    "previousCameraPositionFract",
    "previousCameraPositionInt",
    "relativeEyePosition",
    "cloudHeight",
    "bedrockLevel",
    "isRightHanded",
    "endFlashIntensity",
    "endFlashPosition",
    "eyeAltitude",
    "upPosition",
    "sunPosition",
    "moonPosition",
    "shadowLightPosition",
    "sunAngle",
    "shadowAngle",
    "worldTime",
    "worldDay",
    "moonPhase",
    "frameCounter",
    "frameTime",
    "frameTimeCounter",
    "viewWidth",
    "viewHeight",
    "aspectRatio",
    "near",
    "far",
    "rainStrength",
    "wetness",
    "thunderStrength",
    "fogColor",
    "skyColor",
    "fogMode",
    "fogStart",
    "fogEnd",
    "fogDensity",
    "fogShape",
    "isEyeInWater",
    "blindness",
    "nightVision",
    "darknessFactor",
    "darknessLightFactor",
    "screenBrightness",
    "hideGUI",
    "eyeBrightness",
    "eyeBrightnessSmooth",
    "heldItemId",
    "heldItemId2",
    "heldBlockLightValue",
    "heldBlockLightValue2",
    "renderStage",
    "biome",
    "biome_category",
    "biome_precipitation",
    "temperature",
    "rainfall",
    "gbufferModelView",
    "gbufferModelViewInverse",
    "gbufferProjection",
    "gbufferProjectionInverse",
    "gbufferPreviousModelView",
    "gbufferPreviousProjection",
    "atlasSize",
    "shadowModelView",
    "shadowModelViewInverse",
    "shadowProjection",
    "shadowProjectionInverse",
];

/// Values carried from frame to frame.
pub struct FrameUniforms {
    pub values: Vec<Value>,
    /// The block atlas's size (`atlasSize`), 0 without one.
    pub atlas: [u32; 2],
    start: std::time::Instant,
    last: Option<std::time::Instant>,
    frame: u32,
    previous: Option<(DVec3, Mat4, Mat4)>,
    wetness: f32,
    eye_smooth: [f32; 2],
    /// The pack's shadow settings (7.1).
    pub shadow: Shadow,
}

impl Default for FrameUniforms {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            atlas: [0; 2],
            start: std::time::Instant::now(),
            last: None,
            frame: 0,
            previous: None,
            wetness: 0.0,
            eye_smooth: [0.0; 2],
            shadow: Shadow::default(),
        }
    }
}

/// `(0, 100, 0)` on the frame the sky draws a body on: Y -90°, then X by its angle in degrees;
/// `sunPathRotation` (`tilt`, degrees) then turns the path about the world's X axis (7.1, OQ-6).
fn body(angle: f32, tilt: f32) -> Vec3 {
    (tilt_matrix(tilt)
        * Mat4::from_rotation_y((-90.0f32).to_radians())
        * Mat4::from_rotation_x(angle.to_radians()))
    .transform_vector3(Vec3::new(0.0, 100.0, 0.0))
}

/// `sunPathRotation`'s turn of the sky's bodies: `tilt` degrees about the world's X axis, so a
/// positive angle moves the noon sun toward -Z (north), matching pack coordinates.
pub fn tilt_matrix(tilt: f32) -> Mat4 {
    Mat4::from_rotation_x(-tilt.to_radians())
}

/// The world direction of the shadow light (7.1): the sun by day, the moon by night.
pub fn shadow_light(frame: &Frame, shadow: &Shadow) -> Vec3 {
    let sun_angle = (frame.sky.sun_angle / 360.0 + 0.25).rem_euclid(1.0);
    let angle = if sun_angle < 0.5 {
        frame.sky.sun_angle
    } else {
        frame.sky.moon_angle
    };
    body(angle, shadow.sun_path_rotation).normalize()
}

/// `shadow_camera`'s view and projection in wgpu's clip space (depth 0..1), for culling.
pub fn shadow_culling(frame: &Frame, shadow: &Shadow) -> Mat4 {
    let (view, gl) = shadow_camera(shadow_light(frame, shadow), frame.camera, shadow);
    // GL's depth -w..w to wgpu's 0..w.
    let depth = Mat4::from_cols(
        Vec4::X,
        Vec4::Y,
        Vec4::new(0.0, 0.0, 0.5, 0.0),
        Vec4::new(0.0, 0.0, 0.5, 1.0),
    );
    depth * gl * view
}

/// How far toward the light the shadow projection starts, in blocks: part of the projection,
/// not of `shadowModelView` (Iris's matrices, measured).
const SHADOW_EYE: f32 = 100.0;

/// The shadow camera as Iris places it (7.1; measured from the outside): its view (player
/// space to shadow view space) and GL projection. The view undoes `sunPathRotation`, turns the
/// light's path about Z by the light's angle from the zenith, then X by 90 degrees, so the
/// light lies on the view's +Z. Its origin is `shadowIntervalSize / 2 - camera % interval`
/// (the remainder takes the camera's sign) from the player on each world axis, so the map moves
/// in whole steps. The orthographic projection covers `shadowDistance` each side and depths
/// from `-near` to `far`, counted from `SHADOW_EYE` blocks toward the light; or perspective
/// with `shadowMapFov`.
pub fn shadow_camera(light: Vec3, camera: DVec3, shadow: &Shadow) -> (Mat4, Mat4) {
    let tilt = f64::from(shadow.sun_path_rotation).to_radians();
    let untilted = DMat4::from_rotation_x(tilt).transform_vector3(light.as_dvec3());
    let zenith = untilted.x.atan2(untilted.y);
    let rotation = DMat4::from_rotation_x(std::f64::consts::FRAC_PI_2)
        * DMat4::from_rotation_z(zenith)
        * DMat4::from_rotation_x(tilt);
    let step = f64::from(shadow.interval);
    let origin = if step > 0.0 {
        DVec3::splat(step / 2.0) - camera % step
    } else {
        DVec3::ZERO
    };
    let view = (rotation * DMat4::from_translation(-origin)).as_mat4();
    let (near, far) = (shadow.near, shadow.far);
    let eye = Mat4::from_translation(Vec3::new(0.0, 0.0, -SHADOW_EYE));
    let projection = match shadow.fov {
        Some(fov) => Mat4::perspective_rh_gl(fov.to_radians(), 1.0, near, far) * eye,
        None => {
            let d = shadow.distance;
            Mat4::orthographic_rh_gl(-d, d, -d, d, -near, far) * eye
        }
    };
    (view, projection)
}

/// `value` moved toward `target` with a half-life of `half_life` seconds over `dt` (5.6).
fn smooth(value: f32, target: f32, half_life: f32, dt: f32) -> f32 {
    target + (value - target) * 0.5f32.powf(dt / half_life)
}

/// Split in f64 before casting: negative coordinates must have a fraction in [0, 1),
/// and sub-block precision must survive beyond f32's exact integer range.
fn camera_fraction(camera: DVec3) -> Vec3 {
    (camera - camera.floor()).as_vec3()
}

/// A shader pack's projection must remain a normal perspective lens. Many packs
/// reconstruct view-space positions from only its diagonal and depth terms; putting
/// bobbing into that matrix makes those positions jump while the player walks.
/// The complete clip transform remains exactly P * pose * camera_rotation.
fn camera_matrices(rotation: Mat4, fov: f32, aspect: f32, far: f32, pose: Mat4) -> (Mat4, Mat4) {
    let view = pose * rotation;
    let projection = Mat4::perspective_rh_gl(fov.to_radians(), aspect, 0.05, far);
    (view, projection)
}

impl FrameUniforms {
    /// On the first frame history is the current camera, never a zero matrix.
    /// Invalid history would make temporal reprojection collapse toward the screen centre.
    fn advance_camera(
        &mut self,
        camera: DVec3,
        view: Mat4,
        projection: Mat4,
    ) -> (DVec3, Mat4, Mat4) {
        self.previous
            .replace((camera, view, projection))
            .unwrap_or((camera, view, projection))
    }

    /// This frame's values for a render size of `size` pixels.
    pub fn update(&mut self, frame: &Frame, inputs: &PackInputs, size: [u32; 2]) {
        let now = std::time::Instant::now();
        let dt = self.last.map_or(0.0, |l| (now - l).as_secs_f32());
        self.last = Some(now);
        let aspect = size[0] as f32 / size[1].max(1) as f32;
        let (view, projection) = camera_matrices(
            crate::camera::view_matrix(frame.yaw, frame.pitch),
            frame.fov_degrees,
            aspect,
            frame.depth_far,
            frame.bob * frame.spin,
        );
        let camera = frame.camera.as_vec3();
        let (previous_camera, previous_view, previous_projection) =
            self.advance_camera(frame.camera, view, projection);

        let to_view = |v: Vec3| view.transform_vector3(v);
        let sun_angle = (frame.sky.sun_angle / 360.0 + 0.25).rem_euclid(1.0);
        let day = sun_angle < 0.5;
        let tilt = self.shadow.sun_path_rotation;
        let sun = to_view(body(frame.sky.sun_angle, tilt));
        let moon = to_view(body(frame.sky.moon_angle, tilt));
        // Half-lives are given in ticks (7.2); smoothing runs per frame in seconds (OQ-26).
        let half_life = if inputs.rain > self.wetness {
            600.0
        } else {
            200.0
        };
        self.wetness = smooth(self.wetness, inputs.rain, half_life / 20.0, dt);
        let eye = inputs.eye_light.map(|l| (l * 16) as f32);
        for (smoothed, target) in self.eye_smooth.iter_mut().zip(eye) {
            *smoothed = smooth(*smoothed, target, 10.0 / 20.0, dt);
        }
        // Vanilla fades by the stronger of two linear fogs; a pack gets one: the pair (start
        // and end together) of the one that is complete nearer (OQ-36). Mixing the nearer start
        // of one with the nearer end of the other made fog begin at the camera on clear days.
        let (fog_color, fog_start, fog_end) = match &frame.fog {
            Some(f) => {
                let (start, end) = if f.environmental_end < f.render_distance_end {
                    (f.environmental_start, f.environmental_end)
                } else {
                    (f.render_distance_start, f.render_distance_end)
                };
                ([f.color[0], f.color[1], f.color[2]], start, end)
            }
            None => (frame.sky.fog_color, frame.distance * 0.9, frame.distance),
        };
        let light = shadow_light(frame, &self.shadow);
        let (shadow_view, shadow_projection) = shadow_camera(light, frame.camera, &self.shadow);
        let day_time = inputs.day_time.rem_euclid(24000) as i32;
        let world_day = inputs.day_time.div_euclid(24000);
        let seconds = (now - self.start).as_secs_f32() % 3600.0;

        let value = |name: &str| -> Value {
            let v3 = |v: Vec3| Value::f(&v.to_array());
            let m4 = |m: Mat4| Value::f(&m.to_cols_array());
            match name {
                "cameraPosition" => v3(camera),
                "previousCameraPosition" => v3(previous_camera.as_vec3()),
                "cameraPositionFract" => v3(camera_fraction(frame.camera)),
                "cameraPositionInt" => Value::i(&frame.camera.floor().as_ivec3().to_array()),
                "previousCameraPositionFract" => v3(camera_fraction(previous_camera)),
                "previousCameraPositionInt" => {
                    Value::i(&previous_camera.floor().as_ivec3().to_array())
                }
                "relativeEyePosition" => Value::f(&inputs.relative_eye),
                "cloudHeight" => Value::f(&[inputs.cloud_height]),
                "bedrockLevel" => Value::i(&[inputs.bedrock_level]),
                "isRightHanded" => Value::i(&[i32::from(inputs.right_handed)]),
                "endFlashIntensity" => Value::f(&[inputs.end_flash.0]),
                "endFlashPosition" => v3(to_view(Vec3::from_array(inputs.end_flash.1))),
                "eyeAltitude" => Value::f(&[frame.camera.y as f32]),
                "upPosition" => v3(to_view(Vec3::new(0.0, 100.0, 0.0))),
                "sunPosition" => v3(sun),
                "moonPosition" => v3(moon),
                "shadowLightPosition" => v3(if day { sun } else { moon }),
                "sunAngle" => Value::f(&[sun_angle]),
                "shadowAngle" => Value::f(&[if day { sun_angle } else { sun_angle - 0.5 }]),
                "worldTime" => Value::i(&[day_time]),
                "worldDay" => Value::i(&[world_day as i32]),
                "moonPhase" => Value::i(&[world_day.rem_euclid(8) as i32]),
                "frameCounter" => Value::i(&[self.frame as i32]),
                "shadowModelView" => m4(shadow_view),
                "shadowModelViewInverse" => m4(shadow_view.inverse()),
                "shadowProjection" => m4(shadow_projection),
                "shadowProjectionInverse" => m4(shadow_projection.inverse()),
                "atlasSize" => Value::i(&[self.atlas[0] as i32, self.atlas[1] as i32]),
                "frameTime" => Value::f(&[dt]),
                "frameTimeCounter" => Value::f(&[seconds]),
                "viewWidth" => Value::f(&[size[0] as f32]),
                "viewHeight" => Value::f(&[size[1] as f32]),
                "aspectRatio" => Value::f(&[aspect]),
                "near" => Value::f(&[0.05]),
                "far" => Value::f(&[frame.distance]),
                "rainStrength" => Value::f(&[inputs.rain]),
                "wetness" => Value::f(&[self.wetness]),
                "thunderStrength" => Value::f(&[inputs.thunder]),
                "fogColor" => Value::f(&fog_color),
                "skyColor" => Value::f(&frame.sky.sky_color),
                // GL_LINEAR: vanilla's fog is linear.
                "fogMode" => Value::i(&[9729]),
                "fogStart" => Value::f(&[fog_start]),
                "fogEnd" => Value::f(&[fog_end]),
                "fogDensity" => Value::f(&[1.0]),
                "fogShape" => Value::i(&[0]),
                "isEyeInWater" => Value::i(&[inputs.eye_in]),
                "blindness" => Value::f(&[inputs.blindness]),
                "nightVision" => Value::f(&[frame.night_vision]),
                "darknessFactor" => Value::f(&[frame.darkness.0]),
                "darknessLightFactor" => Value::f(&[frame.darkness.1]),
                "screenBrightness" => Value::f(&[inputs.brightness]),
                "hideGUI" => Value::i(&[inputs.hide_gui as i32]),
                "eyeBrightness" => Value::i(&eye.map(|v| v as i32)),
                "biome" => Value::i(&[inputs.biome]),
                "biome_category" => Value::i(&[inputs.biome_category]),
                "biome_precipitation" => Value::i(&[inputs.biome_precipitation]),
                "temperature" => Value::f(&[inputs.temperature]),
                "rainfall" => Value::f(&[inputs.rainfall]),
                "eyeBrightnessSmooth" => Value::i(&self.eye_smooth.map(|v| v.round() as i32)),
                "gbufferModelView" => m4(view),
                "gbufferModelViewInverse" => m4(view.inverse()),
                "gbufferProjection" => m4(projection),
                "gbufferProjectionInverse" => m4(projection.inverse()),
                "gbufferPreviousModelView" => m4(previous_view),
                "gbufferPreviousProjection" => m4(previous_projection),
                "heldItemId" => Value::i(&[inputs.held_items[0]]),
                "heldItemId2" => Value::i(&[inputs.held_items[1]]),
                "heldBlockLightValue" => Value::i(&[inputs.held_light[0]]),
                "heldBlockLightValue2" => Value::i(&[inputs.held_light[1]]),
                // renderStage is NONE outside gbuffers draws.
                _ => Value::i(&[0]),
            }
        };
        self.values.clear();
        self.values.extend(NAMES.iter().map(|n| value(n)));
        self.frame = (self.frame + 1) % 720_720;
    }

    /// `hz_fog_color` and `hz_fog` of the translator's block (the compatibility `gl_Fog`).
    pub fn fog(&self) -> [Vec4; 2] {
        let get = |name| match index(name).and_then(|i| self.values.get(i)) {
            Some(Value::F(f)) => *f,
            _ => [0.0; 16],
        };
        let (color, start, end) = (get("fogColor"), get("fogStart")[0], get("fogEnd")[0]);
        [
            Vec4::new(color[0], color[1], color[2], 1.0),
            Vec4::new(1.0, start, end, 1.0 / (end - start).max(1e-3)),
        ]
    }
}

impl FrameUniforms {
    /// Evaluates the pack's custom uniforms with this frame's values and puts them after
    /// `NAMES`' values, in `custom.uniforms()` order.
    pub fn eval_custom(&mut self, custom: &mut horizon_shaders::Custom) {
        self.values.truncate(NAMES.len());
        let values = &self.values;
        let builtin = |name: &str| {
            let v = values.get(index(name)?)?;
            let n = components(name);
            Some(match *v {
                Value::F(f) => horizon_shaders::Val::vector(&f.map(f64::from)[..n]),
                Value::I(i) => horizon_shaders::Val::vector(&i.map(f64::from)[..n.min(3)]),
            })
        };
        let dt = match index("frameTime").and_then(|i| values.get(i)) {
            Some(Value::F(f)) => f64::from(f[0]),
            _ => 0.0,
        };
        let out = custom.eval(dt, &builtin);
        self.values
            .extend(out.iter().map(|v| Value::f(&v.v.map(|x| x as f32)[..v.n])));
    }
}

/// How many components the uniform `name` has (16 for a matrix).
fn components(name: &str) -> usize {
    match name {
        "cameraPosition"
        | "previousCameraPosition"
        | "cameraPositionFract"
        | "cameraPositionInt"
        | "previousCameraPositionFract"
        | "previousCameraPositionInt"
        | "relativeEyePosition"
        | "endFlashPosition"
        | "upPosition"
        | "sunPosition"
        | "moonPosition"
        | "shadowLightPosition"
        | "fogColor"
        | "skyColor" => 3,
        "eyeBrightness" | "eyeBrightnessSmooth" | "atlasSize" => 2,
        n if n.starts_with("gbuffer") || n.starts_with("shadow") && n != "shadowAngle" => 16,
        _ => 1,
    }
}

/// The position of `name` in [`NAMES`].
pub fn index(name: &str) -> Option<usize> {
    NAMES.iter().position(|n| *n == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_fraction_survives_negative_and_large_coordinates() {
        assert_eq!(
            camera_fraction(DVec3::new(-0.25, -64.75, 30_000_000.125)),
            Vec3::new(0.75, 0.25, 0.125)
        );
        assert_eq!(components("cameraPositionFract"), 3);
        assert_eq!(components("relativeEyePosition"), 3);
        assert_eq!(components("endFlashPosition"), 3);
    }

    #[test]
    fn temporal_history_starts_at_the_current_camera() {
        let mut uniforms = FrameUniforms::default();
        let camera = DVec3::new(30_000_000.125, 80.5, -0.75);
        let view = Mat4::from_rotation_y(0.8);
        let projection = Mat4::perspective_rh_gl(1.1, 1.5, 0.05, 512.0);
        assert_eq!(
            uniforms.advance_camera(camera, view, projection),
            (camera, view, projection)
        );
        assert_eq!(
            uniforms.advance_camera(camera + DVec3::X, Mat4::IDENTITY, Mat4::IDENTITY),
            (camera, view, projection)
        );
    }

    /// A typical shader-pack fast depth reconstruction: only the inverse lens's
    /// diagonal and final column are used. It is exact for a perspective lens,
    /// but was wrong when the lens also contained walking bob/roll/translation.
    fn reconstruct_relative(ndc: Vec3, view: Mat4, projection: Mat4) -> Vec3 {
        let inverse = projection.inverse();
        let diagonal = Vec4::new(
            inverse.x_axis.x,
            inverse.y_axis.y,
            inverse.z_axis.z,
            inverse.z_axis.w,
        );
        let position = diagonal * Vec4::new(ndc.x, ndc.y, ndc.z, ndc.z) + inverse.w_axis;
        view.inverse()
            .transform_point3(position.truncate() / position.w)
    }

    #[test]
    fn walking_pose_preserves_clip_transform_and_depth_reconstruction() {
        let world = DVec3::new(1005.25, 68.0, -8.0);
        for i in 0..32 {
            let t = i as f32 / 31.0;
            let camera = DVec3::new(1000.0 + f64::from(t), 70.0, -20.0);
            let rotation = crate::camera::view_matrix(10.0 + t * 4.0, 12.0);
            let pose = Mat4::from_translation(Vec3::new(
                0.025 * (t * 12.0).sin(),
                -0.035 * (t * 12.0).cos(),
                0.0,
            )) * Mat4::from_rotation_z(0.04 * (t * 12.0).sin());
            let (view, projection) = camera_matrices(rotation, 70.0, 16.0 / 9.0, 1024.0, pose);
            let relative = (world - camera).as_vec3();
            let clip = projection * view * relative.extend(1.0);
            let old_clip = projection * pose * rotation * relative.extend(1.0);
            assert!(clip.abs_diff_eq(old_clip, 2e-6));
            let reconstructed = reconstruct_relative(clip.truncate() / clip.w, view, projection);
            assert!(
                reconstructed.abs_diff_eq(relative, 0.002),
                "{reconstructed:?} != {relative:?}"
            );
            // Reproject the reconstructed point into the shadow camera too: the
            // receiver must agree with the world geometry even during view bobbing.
            let (shadow_view, shadow_projection) = shadow_camera(
                Vec3::new(0.3, 0.8, 0.5).normalize(),
                camera,
                &Shadow::default(),
            );
            let actual = (shadow_projection * shadow_view).project_point3(relative);
            let receiver = (shadow_projection * shadow_view).project_point3(reconstructed);
            assert!(
                actual.abs_diff_eq(receiver, 2e-5),
                "shadow receiver drifted during walking"
            );
        }
    }

    #[test]
    fn rotating_the_view_does_not_move_a_fixed_shadow_receiver() {
        let camera = DVec3::new(1000.4, 70.25, -20.7);
        let relative = Vec3::new(3.5, -2.5, 12.0);
        let (shadow_view, shadow_projection) = shadow_camera(
            Vec3::new(-0.2, 0.9, 0.3).normalize(),
            camera,
            &Shadow::default(),
        );
        let shadow = shadow_projection * shadow_view;
        for yaw in [-120.0, -60.0, 0.0, 60.0, 120.0] {
            let (view, projection) = camera_matrices(
                crate::camera::view_matrix(yaw, 15.0),
                70.0,
                16.0 / 9.0,
                1024.0,
                Mat4::IDENTITY,
            );
            let clip = projection * view * relative.extend(1.0);
            let point = reconstruct_relative(clip.truncate() / clip.w, view, projection);
            assert!(
                shadow
                    .project_point3(point)
                    .abs_diff_eq(shadow.project_point3(relative), 2e-5)
            );
        }
    }

    #[test]
    fn values_follow_the_declared_type() {
        let mut out = vec![0u8; 64];
        Value::i(&[7]).write("float", &mut out, 0);
        assert_eq!(f32::from_ne_bytes(out[0..4].try_into().unwrap()), 7.0);
        Value::f(&[1.5, 2.5]).write("ivec2", &mut out, 4);
        assert_eq!(i32::from_ne_bytes(out[8..12].try_into().unwrap()), 2);
        Value::i(&[1]).write("bool", &mut out, 16);
        assert_eq!(i32::from_ne_bytes(out[16..20].try_into().unwrap()), 1);
        Value::f(&[1.0; 16]).write("mat4", &mut out, 60);
        assert_eq!(out[60..64], [0; 4], "a value past the block is dropped");
    }

    #[test]
    fn noon_is_straight_up_and_the_sun_sets_in_the_west() {
        assert!(body(0.0, 0.0).abs_diff_eq(Vec3::new(0.0, 100.0, 0.0), 1e-3));
        assert!(body(90.0, 0.0).abs_diff_eq(Vec3::new(-100.0, 0.0, 0.0), 1e-3));
        // sunPathRotation tilts noon toward -Z and leaves sunset on the horizon.
        assert!(body(0.0, 30.0).abs_diff_eq(Vec3::new(0.0, 86.60254, -50.0), 1e-3));
        assert!(body(90.0, 30.0).abs_diff_eq(Vec3::new(-100.0, 0.0, 0.0), 1e-3));
    }

    #[test]
    fn the_shadow_camera_looks_down_from_the_light_in_steps() {
        let shadow = Shadow::default();
        let mut camera = DVec3::new(1000.3, 70.0, -20.7);
        // Noon: the light is straight up.
        let (view, projection) = shadow_camera(Vec3::Y, camera, &shadow);
        let at = |p: Vec3| view.transform_point3(p);
        // Straight down: a block below the player is farther from the light.
        assert!(at(Vec3::new(0.0, -1.0, 0.0)).z < at(Vec3::ZERO).z);
        // The view's origin is interval / 2 - camera % interval from the player on each axis,
        // the remainder with the camera's sign: up to a step and a half on negative axes.
        let origin = view.inverse().w_axis.truncate();
        assert!(
            origin.abs_diff_eq(Vec3::new(0.7, 1.0, 1.7), 1e-3),
            "{origin}"
        );
        // Moving within a step keeps the world where it was in the map.
        camera.x = 1000.6;
        let (moved, _) = shadow_camera(Vec3::Y, camera, &shadow);
        let world = Vec3::new(5.0, -3.0, 9.0);
        let shift = Vec3::new(0.3, 0.0, 0.0);
        assert!(
            moved
                .transform_point3(world - shift)
                .abs_diff_eq(at(world), 1e-3)
        );
        // shadowDistance blocks reach the map's edge.
        let edge = projection.project_point3(Vec3::new(160.0, 0.0, 0.0));
        assert!((edge.x.abs() - 1.0).abs() < 1e-4, "{edge}");
    }

    /// Iris's matrices at 3000 ticks with `sunPathRotation = 30` and the camera at
    /// (0.5, -58.38, 0.5), read from its uniforms by a probe pack.
    #[test]
    fn the_shadow_camera_matches_iris() {
        let shadow = Shadow {
            sun_path_rotation: 30.0,
            ..Shadow::default()
        };
        let light = Vec3::new(0.56782, 0.71287, -0.41158).normalize();
        let (view, projection) = shadow_camera(light, DVec3::new(0.5, -58.38, 0.5), &shadow);
        let iris_view = Mat4::from_cols_array_2d(&[
            [0.82315, 0.0, 0.56782, 0.0],
            [-0.49175, -0.5, 0.71287, 0.0],
            [0.28391, -0.86603, -0.41158, 0.0],
            [0.12508, 1.12301, -1.06188, 1.0],
        ]);
        assert!(view.abs_diff_eq(iris_view, 2e-4), "{view}");
        assert!(
            (projection.z_axis.z + 0.0078110).abs() < 1e-6,
            "{projection}"
        );
        assert!((projection.w_axis.z + 0.21851).abs() < 1e-4, "{projection}");
        // Elsewhere the origin keeps to the steps: Iris put it at (-0.5, 1.38, 2.5) here.
        let (view, _) = shadow_camera(light, DVec3::new(3.5, -58.38, -1.5), &shadow);
        let origin = view.inverse().w_axis.truncate();
        assert!(
            origin.abs_diff_eq(Vec3::new(-0.5, 1.38, 2.5), 1e-3),
            "{origin}"
        );
    }
    #[test]
    fn nearby_shadow_samples_are_not_on_the_invalid_depth_half() {
        let shadow = Shadow::default();
        let (view, projection) = shadow_camera(Vec3::Y, DVec3::ZERO, &shadow);
        // Pack light shafts commonly accept only the near half of shadow clip depth.
        // The old 128-block offset placed the eye at that boundary, so even a
        // nearby sample below the eye was rejected and treated as unshadowed.
        for below in [0.0, 1.0, 8.0, 16.0] {
            let clip = (projection * view).project_point3(Vec3::new(0.0, -below, 0.0));
            let packed_depth = clip.z * 0.2 * 0.5 + 0.5;
            assert!(
                packed_depth < 0.5,
                "sample {below} below eye became invalid: {packed_depth}"
            );
        }
    }
}
