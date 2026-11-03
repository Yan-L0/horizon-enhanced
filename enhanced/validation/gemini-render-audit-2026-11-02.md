# Renderer audit with Gemini, 2026-11-02

Baseline source: `c663bd731f50c5e74570af2b00786f2d0cb5df9a`. Target measured locally: Apple M2 / Metal, BSL Quality r2 (unchanged ZIP), six chunks, render scale 75%, 1710×977 logical / 3420×1954 physical window. This is an existing comparison configuration, not a quality reduction. Each run uses a fresh copy of the same local fixture world and isolated game/config/launcher homes. No original player data are written.

## Independent consultation

The user explicitly authorized transmitting `pack_post.rs` to Gemini API. Two calls used `gemini-3.5-flash-lite`, with that source file only; no worlds, credentials or game resources were transmitted. API metadata: 74,281 input tokens, 2,461 output tokens, 76,742 total. No claim is made about saved Codex subscription quota or monetary cost.

Gemini proposed guarding the depth2 copy; rejected because `copy_depth_snapshot` already returns immediately when its optional snapshot is absent. Its second response claimed read-only depth attachments were invalid; rejected based on the pinned local wgpu implementation and an actual GPU test. No Gemini-proposed patch was accepted directly. Its contribution was independent scrutiny and candidate discussion, not an autonomous code change or demonstrated speedup.

## Rejected depth experiment

A local candidate marked non-writing sky attachments explicitly read-only, with conservative fragment-depth/shadow guards. Thirty shader-pack GPU tests passed, including preserved depth values and foreground occlusion. Backend inspection showed Metal still uses Load+Store for read-only depth, so removing depth-store traffic was not a valid claim.

Initial repeated runs were too variable and one initial surface camera differed; those runs are exploratory, not evidence of improvement. Controlled camera resets and 30-second warmups gave median three-sample FPS: baseline 50.25 cave / 43.05 surface, depth candidate 34.88 / 29.05. These single alternating runs do not isolate thermal/host variance, but clearly fail to establish a speedup. The depth optimization was removed from final production code.

Live world-region image differences for the depth experiment: cave MAE 0.025/255, surface 1.148/255. Animated/temporal surface output prevents those captures from proving exact equality. Deterministic GPU checks, not live image similarity alone, establish depth behavior.

## Final candidate

Completed on 2026-11-03. Source and regression tests are preserved in [`perf/ubo-upload-experiment`](https://github.com/Yan-L0/horizon-enhanced/tree/perf/ubo-upload-experiment), commit `ccaed52`. The experiment is not enabled in `main` and was not packaged as a new performance release.

The candidate conservatively identifies active UBO fields across both shader stages and skips bit-identical uploads, without changing layout, offsets or shader settings. Unknown pointer uses retain complete serialization. Unused compatibility-fog initialization was removed only from stages with no `gl_Fog` token. Among 23 loaded program entries, active members fell from 616 declared to 421 retained (member counts, not a measured CPU-time percentage; not every loaded program runs every frame). Examples: `composite4` 3/7, `composite6` 2/7, `final` 2/7.

All 51 shader-library tests and 33 renderer-pack tests passed, including exact GPU pixel equality between full and filtered uploads, changing active values, ignoring unused time, nested matrix/array members, whole-block fallback and host-header behavior. macOS arm64 and Windows x64 MSVC clients compiled successfully. Native Windows GPU behavior was not measured.

Final alternating runs used fixed camera resets before every sample, checked unchanged camera position/yaw/pitch, 30-second warmups and three 8-second samples per scene. Median FPS across each run's three samples:

| Run | Cave | Surface |
|---|---:|---:|
| Baseline A1 | 55.88 | 47.07 |
| Candidate B1 | 55.04 | 41.55 |
| Baseline A2 | 56.46 | 11.68 |
| Candidate B2 | 50.56 | 43.73 |

All camera checks passed. CPU/GPU and frame-time percentiles are in the sanitized [JSON report](ubo-experiment-2026-11-03.json). The baseline surface run itself varied severely; host/background/thermal and streaming conditions were not sufficiently controlled to assign causality. Cave results show no gain and a possible regression. **There is no demonstrated FPS improvement, so the candidate was not promoted to main.** Do not quote the surface recovery from 11.68 to 43.73 as an optimization gain.

Final live image comparison (world ROI, excluding HUD/toasts) A1 vs B1: cave MAE 0.0089/255, maximum channel difference 7; surface MAE 1.4281/255, maximum 165. Surface animation and temporal differences prevent an exact-equivalence claim from live images. The deterministic GPU regression did produce byte-identical output for its controlled cases. Images and raw personal logs remain outside Git.

The main-branch deliverable is a reusable isolated comparison tool and this honest record of accepted/rejected candidates. The stable renderer is unchanged. No maximum attainable FPS, energy savings, native Windows gains or Codex quota savings are claimed.
