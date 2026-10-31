# Development milestones

All dates are local project dates, October 2026. This Git history starts with a pinned upstream snapshot and imports the completed work honestly; it does not invent historical source commits for earlier binaries.

- **Upgrader 0.1.0, October 30:** experimental Rust port, content item, GUI/settings, pricing and server-authority checks. Source/content retained in archive; excluded from current builds.
- **Enhanced 0.1.1, October 31:** GLSL BSL integration, noise/uniform/depth/sky work, stable macOS Application Support bootstrap instead of a protected Documents working directory.
- **Enhanced 0.2.0:** plant/cutout transparency, dark outline and crack overlay, camera-bob reconstruction fix, End fog profiles. GPU and synthetic scene checks.
- **Enhanced 0.3.0:** reviewed newer nightly, conservative mip-generation demand analysis, Upgrader removal. Paired M2 test scene approximately 56.7 to 58.1 FPS; no universal guarantee.
- **Enhanced 0.4.0:** corrected shadow camera, safe sky-after-terrain depth rejection, r2 underground sky/fog/shaft approximation and zero-contribution early return. Copied cave scene approximately 78 to 88 FPS. 28 renderer tests passed on M2/Metal.
- **Windows 0.4.0-windows.1:** actual x86_64 MSVC executable and control tool, static CRT, PowerShell/CMD bootstrap. Windows VM launch/API/migration checks passed; resources and DX12 pipelines loaded and a temporary server accepted the player. Full shader visual/performance validation on native Windows GPU remains pending.
- **GitHub import:** complete patched source, archived Upgrader/tooling, sanitized evidence, recipe-only shader generation, current and historical binary release assets. Future authorized work is committed and pushed here.

Previous binary archives are retained as historical artifacts, not recommended alternatives to the latest build. Private BSL derivatives remain release assets rather than Git source; original game assets are never included.
