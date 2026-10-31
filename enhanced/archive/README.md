# Historical development archive

`upgrader/` preserves the experimental disabled mod's standalone core, integration source, content definitions and original notices. It is not referenced by the main game workspace. Its custom content texture is from the provided mod, not a Minecraft asset. The original Java JAR, decompiled sources and prepared game data are not included.

`tooling/` preserves staged development scripts, including macOS bootstraps, shader fixes, reproduction fixtures, packaging and Windows cross-build/VM probes. Local paths/device identifiers have been replaced with placeholders. These scripts document the previous scratch workspace layout and require adaptation before reuse; they are not a one-command build system for this checkout.

Active launchers are in `enhanced/platform/`, the portable recipe generator is `enhanced/profiles.py`, and the root Cargo workspace builds the current client. Do not re-enable Upgrader automatically.
