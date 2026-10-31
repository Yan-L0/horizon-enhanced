# Windows water material fix

The Windows screenshots show Intel Iris Xe, Vulkan and native multi-draw terrain. The source enables flat terrain by default on Metal (and a specific Intel ANV workaround), while native terrain previously substituted `mc_Entity=(-1,-1,0,0)` and a constant upward normal/tangent. BSL recognizes water through material ID 20000. Its existing sprite table already assigns that ID correctly; the native vertex path never read the table.

Both terrain paths now bind the existing material-ID and arena-vertex tables. Native rendering binds the correct page before each existing multi-draw run, and reconstructs the same face basis as flat rendering. No shader preset reduction, forced flat rendering, or extra terrain draw calls are introduced.

Validation: 29 `gfx::pack_` tests passed on Apple M2/Metal with `HORIZON_REQUIRE_GPU=1`. The arena storage-binding limit regression also passed; pages are capped to the adapter storage limit. The added GPU regression translates BSL-style material classification and verifies material/fluid IDs, face normal and tangent for native and flat inputs, including nonzero base-vertex and instance offsets. This tests the formerly broken native vertex path on Metal; it is not a visual test on the user's Windows GPU. The remote connection could not be controlled, and no Windows runtime log was supplied. Other backend-specific issues remain possible.

For a real-PC check, fully extract the 0.4.1 Windows archive, close the old Enhanced client, and start the new `Launch-Horizon.cmd`. The existing Enhanced data folder and chosen shader pack remain in use. Compare daytime water from an oblique angle with Quality r2 enabled. If still incorrect, retain the latest client stderr log from the Enhanced logs folder for diagnosis.
