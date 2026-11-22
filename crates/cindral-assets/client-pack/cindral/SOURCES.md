# Cindral client pack — sources

Every file in this directory is embedded into the client by
`cindral_assets::client_pack` (ADR-0056). Each binary file is also listed in
the `assets-guard` allowlist of `.github/workflows/ci.yml`.

| File | Author | License | Notes |
|---|---|---|---|
| `textures/ui/logo.png` | Cindral contributors | MIT OR Apache-2.0 | Original Cindral logo (an ember in a C), white on transparent, 359x354, drawn by `gen_art.py`. |
| `textures/ui/panorama/panorama_0..5.png` | Cindral contributors | MIT OR Apache-2.0 | Original twilight panorama, 1024x1024 per face, computed by `gen_art.py` from the view direction of each pixel (seamless). Replaces the third-party panorama of the upstream project, whose license was never recorded. |
| `gen_art.py` | Cindral contributors | MIT OR Apache-2.0 | Regenerates the logo and the panorama: `python gen_art.py OUT_DIR` (NumPy, Pillow). |
| `textures/ui/settings.png` | Cindral contributors | MIT OR Apache-2.0 | Original 64x64 white icon, drawn as SVG and rasterized with ImageMagick. |
| `font/Inter-SemiBold.ttf` | The Inter Project Authors | SIL OFL 1.1 (`font/OFL.txt`) | Inter 4.1, `extras/ttf`. |
| `font/hd/{16,24,32}.png` | Cindral project owner | MIT OR Apache-2.0 | HD pixel font used by the Ember and Amethyst themes (ADR-0078): a 16x16 glyph grid in the `ascii.png` layout, cells of 16, 24 and 32 px. The owner confirmed on 2026-11-17 that they own the files. Advances are derived from the pixels, not shipped. |
| `lang/*.lang` | Cindral contributors | MIT OR Apache-2.0 | `cindral.*` keys only. |
