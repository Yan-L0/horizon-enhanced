# Upstream review before optimization

Reviewed released nightly `7aa0e28faee33841b60906e69f16b47e54681a7f` (2026-10-31), not moving main. Launcher release is 0.2.30; launcher version is separate from renderer version. Current private base is `1da354b67a509812c4141b67de2affd6de65c342` plus local rendering fixes. Read upstream AGENTS.md and CONTRIBUTING.md. No external project source was consulted; only MIT Horizon sources and public release metadata.

## Findings

- The newer GLSL pack renderer retains the same overall shadow/gbuffers/deferred/composite structure, per-frame bind-group creation and ping-pong targets. `gfx/pack_timing.rs` is byte-identical between the two trees. There is no ready-made upstream replacement that makes BSL substantially cheaper with the same settings.
- `gfx/pack_uniforms.rs` now feeds the shader uniform `far` from `frame.detail_distance` (near chunk render distance), while the projection still uses `frame.depth_far`. Current code feeds `far` from `frame.distance` (including distant terrain). This is a small, compatible correctness backport; the field already exists. It can change fog/denoise ranges and should be visually tested, not advertised as an FPS improvement.
- The shadow camera has a new construction: opposite sign for sun-path tilt, rotation based on zenith, world-axis interval origin, 100-block light offset in the projection and orthographic near sign. This fixes pack interoperability; it changes lighting placement, so is not an appearance-identical optimization. Port only as a separately tested correctness change, retain our bob-in-view and initialized temporal history fixes. Upstream itself does not have our local camera precision/history additions.
- Upstream has tight section geometry bounds for frustum culling and worker mesh scratch/capacity reuse. They can reduce geometry/CPU work without lowering visual quality, but bounds span mesher/resident/upload/culling paths. A selective backport requires the full data flow and visibility tests; copying only resident.rs is unsafe. They primarily help world/geometry load rather than costly full-screen BSL passes.
- Upstream has expanded OIT/entity paths and terrain/LOD fade encoding changes. These are broad render changes, not a minimal BSL speed fix. Existing GLSL packs keep their own transparency path.
- Current private alpha cutoff, custom noise/PBR defaults, independent depthtex2 snapshot, native crack/outline post-overlay are absent upstream. Replacing current pack files wholesale would reintroduce observed bugs.
- `horizon-prepare::OUTPUT_VERSION` increased from 2 to 3 (creative tab export change). A full rebase needs resource migration/preparation validation; it should not silently reuse the player's old prepared resources.

## Recommendation

Keep the pinned working client for this private optimization release. Selectively backport compatible corrections only after preserving local fixes. First measure BSL GPU pass times on the M2, then remove redundant renderer work or cache invariant GPU bindings, keeping pack settings and render scale unchanged. Compare the same binary baseline/optimized executable in alternating runs with a deterministic scene; use GPU timings plus FPS and before/after images. Do not call a lower-quality preset renderer optimization. Keep original BSL and Quality settings, and do not auto-select Balanced, whose previous measurement was worse.

Source: https://github.com/horizon-rust/horizon/tree/7aa0e28faee33841b60906e69f16b47e54681a7f and https://github.com/horizon-rust/horizon/releases/tag/nightly
