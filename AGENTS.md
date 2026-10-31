# Horizon Enhanced working instructions

This is Yan-L0's private independent Horizon Enhanced repository. The user explicitly requested that subsequent development be committed and pushed here. Use this checkout as the active source for future authorized Horizon work.

- Preserve the upstream MIT license and attribution. The pinned import is 1da354b67a509812c4141b67de2affd6de65c342. Upstream guidance is preserved in enhanced/docs/AGENTS-UPSTREAM.md and CONTRIBUTING.md; claims/issues/comments in the upstream project are not part of this private workflow.
- After completing an authorized change, run appropriate checks, create meaningful Git commits, and push origin/main. Do not require another permission question for these previously authorized commits/pushes. Do not add AI co-author trailers or AI signatures; the user explicitly requested their removal.
- Never commit Mojang jars/assets/prepared resources, original mod jars, player worlds, accounts/tokens/keys, raw personal logs, downloaded toolchains, target directories, or release binaries. Shader inputs stay outside Git; enhanced/profiles.py generates private derivatives. Put approved build archives in private GitHub Releases.
- Test with fresh HORIZON_HOME, HORIZON_LAUNCHER_HOME, XDG_DATA_HOME and XDG_CONFIG_HOME, HORIZON_LAUNCHER_NO_SELF_UPDATE=1 and HORIZON_NO_AUDIO=1. Additional launcher trash/clipboard/desktop paths also belong in the scratch home. Do not modify real worlds during tests.
- Preserve visual quality when optimizing; distinguish renderer changes from preset quality reductions. Measure comparable conditions and state unverified Windows GPU behavior honestly.
- Upgrader remains disabled in current builds; its source is archived under enhanced/archive/upgrader.
- Historical tooling under enhanced/archive/tooling documents the prior workspace and is not assumed portable. Active platform launchers live under enhanced/platform.
- Communicate with subagents in English and with the user in Russian. Subagents are authorized for this project when they improve speed or cost.
