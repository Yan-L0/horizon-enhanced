# Upgrader — private Horizon port

Target: Horizon Nightly 2026-10-30, commit `1da354b67a509812c4141b67de2affd6de65c342`, Minecraft resources 26.3, macOS Apple Silicon.

This is a separate Horizon client build with the Rust gameplay adapter built in, plus the content-only `.hmod` containing the original Upgrader item, texture, names and crafting recipe. The content `.hmod` alone does **not** implement the upgrade mechanic. Forge `.jar` files are not used by the client.

## Run

Open **Horizon Upgrader.app**. It reads the resources already prepared by your existing Horizon installation. Its worlds, options, screenshots and config are kept separately in `UserData/` beside the app. It does not replace the installed Horizon or open your existing worlds automatically.

Craft `upgrader:upgrader` using the original recipe:

```
G D G
D A D
G D G
```

G = gold ingot, D = diamond, A = anvil.

Right-click with the item, press **U**, or type `/upgrader` to open the menu. Opening the menu with U or the command still requires the crafted item in your inventory to upgrade. The settings screen can be viewed separately with `/upgrader config`.

- Click an inventory stack to stake it; right-click to stake half. The chosen stack stays reserved by reference to its server inventory slot until the attempt. Clearing the input or closing the menu keeps it in the inventory.
- Click the target card to open the searchable item catalog. Set the target amount, then press Upgrade.
- Settings contains all eight global settings. Item Prices includes fixed prices, multipliers, blacklist, search, block/item/changed filters and sorting. Empty per-item fields remove the override. Save applies the settings on the server and persists JSON.
- Single-player owners can edit settings. On a shared server only its operators can edit them.
- Invalid or changed source stacks are rejected by the server. Winning/losing and item mutation happen atomically on the server at the start of the wheel animation; closing/disconnecting does not cancel or repeat the result. The result is displayed after the configured animation duration.

## Multiplayer

Host and friend both need this matching client build and content package. The host uses the rebuilt engine, including its integrated server. This is not compatible with an unmodified Java/Forge server or an unmodified Horizon server for this mod's gameplay. The LAN/server transport uses `upgrader:main` custom payloads; it never trusts a client-supplied probability, result or item price.

## Source and validation

`src/core.rs`: settings, prices, odds, durability, limit windows, wheel angle and wire data.

`src/engine.rs`: authoritative inventory operations, network transport, catalog, menus and wheel.

`prepare_source.py SOURCE`: apply the adapter to the pinned pristine Horizon source. It preserves existing engine logic and adds the isolated adapter module and entry points. Do not apply it to a different revision.

Core tests: `cargo test`. Full client: apply the source, then `cargo build --locked --profile fast -p horizon-client --bin horizon-client` in that source workspace. Copy the binary into the app's `Contents/MacOS/horizon-client`.

This is a Rust reimplementation of the supplied mod, not a byte-for-byte conversion. The menu uses Horizon's renderer, retaining the original main layout and feature categories. The original Java random loot sampling is re-expressed with Horizon's loot engine; sampled prices can vary, as in the original.

See `VALIDATION.md` for what was actually tested; do not infer a tested platform from an available code path. Settings are intended to be opened inside a world.
