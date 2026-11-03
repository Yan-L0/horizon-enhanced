# Shader-pack comparisons

`measure_pack.py` launches one client at a time, copies a disposable local fixture world to fresh temporary homes, fixes world time/weather and camera, warms each scene for 30 seconds, and records three 8-second frame-time samples plus PNG captures. The input world is never modified. It needs the existing cave/surface fixture coordinates in the script; it does not generate that fixture or include game assets.

Supply explicit local paths for `--client`, `--ctl`, `--resources`, `--world`, `--pack`, and `--output`. Run the same command with the before/after clients in alternating order, using separate output directories outside Git. Keep `--scale`, `--width`, `--height`, chunk distance and shader ZIP unchanged. On Retina screens the physical target is larger than the requested logical dimensions; record the actual renderer dimensions.

Do not run compilation, GPU tests, other games or GPU-heavy applications during measurements. Examine camera state, CPU/GPU percentiles and repeat variance before claiming a performance gain. Animated shader noise, weather effects, entities and temporal history prevent arbitrary live captures from being byte-identical. Compare repeated baseline captures as a noise reference and use deterministic GPU fixtures to check exact rendering behavior.

The script retains its own temporary copy and raw logs to support diagnosis. Those files, captures, worlds, assets and private machine information must stay out of Git. Only sanitized summaries belong under `enhanced/validation/`.
