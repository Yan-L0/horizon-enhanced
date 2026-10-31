# Validation — Upgrader Horizon 0.1.0

Environment: Apple M2, macOS 26.3, Rust 1.99.1, Horizon source `1da354b67a509812c4141b67de2affd6de65c342`. The packaged binary is the final tested build; its SHA-256 is in `PORT-METADATA.json`.

## Passed

- Optimized `fast` build of the complete Horizon client with the adapter.
- Eight Rust unit tests: original default odds and durability, rejecting invalid/non-finite config, attempt-window boundaries, winning/losing wheel sectors, recipe-derived costs and fixed overrides, sampled-loot floors, built-in deny list, partial-stack visible remainder.
- `rustfmt --check` on the port's Rust files.
- Applying the source adapter to freshly extracted pristine pinned sources reproduces the exact module used by the build. Input source files are checked by SHA-256.
- Content package loads in the actual game with the Upgrader item and its original texture.
- Click-driven UI test: select inventory input, open catalog, search, select diamond, press Upgrade. On the authoritative integrated server, 10 iron ingots were removed and one diamond was awarded.
- Settings UI: change spin duration to 40 ticks and save; server JSON contains 40.
- Item Prices UI: search iron ingot, set its fixed price to 123, save; server JSON contains 123.
- Select half after an inventory change with the menu already open: stake 5 of 10 iron ingots; a winning attempt leaves the other 5 and awards one diamond.
- Observed losing attempt removes its stake and gives no target item.
- Attempt limiter disables a new attempt and preserves a newly selected input item.
- Two independent clients on local LAN: a guest upgrades iron to diamond; the host's inventory is unchanged. The guest's settings page has no editable setting fields or Save button. Both processes used the final packaged binary.
- Attempts to select the built-in forbidden bedrock target do not change inventory.

The tests use fresh isolated homes and worlds, not the player's existing saves. Forced 100% odds were used in win tests; very low odds were used for the observed losing test. Those test configurations are **not** included in the production UserData. Default odds/settings remain the original 0.9 house edge, 0.001 minimum chance, 0.9 maximum chance and 70 spin ticks.

Screenshots are from test worlds and show test values, not necessarily defaults.

## Scope and differences

This is a first experimental Rust reimplementation, packaged as a separate client with a content `.hmod`. It is not an ordinary Forge JAR or a drop-in `.hmod` containing all gameplay. The UI uses Horizon's renderer and retains the main layout and feature categories, rather than duplicating Java's renderer pixel for pixel.

The selected stake stays in a server inventory slot by reference until the attempt. Results settle atomically at the start of the wheel animation; the animation then reveals the result. This intentionally differs from the original delayed container mutation and avoids temporary-item escrow on disconnect.

The full original Java-mod parity has not been established for every possible datapack, item component, loot table or rare UI sequence. The original crafting recipe is included, but the end-to-end test obtained the item with a creative/server give command rather than crafting it. Settings are intended to be opened after entering a world. Full LAN transport was tested; Play with Friends relay, a separately managed dedicated-server deployment, Windows and Linux builds were not tested.

The toolchain emitted a non-fatal debug-symbol stripping warning from rust-objcopy. Compilation, launch, UI and inventory/network tests completed successfully.
