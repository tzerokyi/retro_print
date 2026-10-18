# RetroPrint

RetroPrint is a compact, offline, cross-platform Rust desktop app that turns photographs and renders into two-color, irregular stipple/retro-print artwork. It is inspired by analog grain, halftone, threshold, and duotone workflows.

![RetroPrint showing the original image and its two-color print preview](assets/retroprint-preview.png)

Version 0.3.0 introduces two selectable render engines: the familiar clustered **Organic** grain and detail-preserving **Fine Stipple** with edge-aware smoothing, local contrast, and public-domain blue-noise dots. Direct ink/paper spectrum pickers, precise HEX controls, and automatic lossless indexed-PNG compression remain available.

The UI uses the official [Lucide](https://lucide.dev/) vector icon set for buttons, controls, import affordances, and status indicators. Icons are bundled into the executable; no separate font or installation is required.

The [GitHub Actions workflow](.github/workflows/rust.yml) checks formatting, linting, and tests on pushes/PRs, then builds a Linux x86-64 ZIP and Flatpak bundle. Push a `v*` tag to publish both as GitHub Release assets.

## Features

- Drag-and-drop or open a PNG, JPEG, or still WebP. Click the empty preview area to browse; drops also accept readable file handles when a direct path is unavailable.
- Side-by-side original and processed previews.
- **Organic** texture mode for the established analog clumping, or **Fine Stipple** for more even dot spacing and better small-detail/edge separation; Organic remains the default so existing looks are unchanged.
- Five controls: Grain Size, Grain Amount, Contrast, Threshold, Irregularity.
- Independently customizable ink and paper colors, with 2D spectrum pickers, `#RRGGBB` inputs, and six one-click palettes.
- Transparent Paper option for print-ready cutouts.
- Deterministic seed, Reseed, and Reset.
- Save original-resolution PNG in a background thread, automatically using 1-bit indexed PNG for two-color images; preserve all input alpha/color values when more palette entries are needed.
- No cloud, telemetry, Python, Photoshop, OpenCV, or FFmpeg.

## Build and run

Install an up-to-date stable Rust toolchain, plus the usual native Linux development dependencies for egui/winit. On an Arch Linux KDE desktop, make sure the Wayland/X11 development environment, D-Bus, and the XDG desktop portal are present.

**Dolphin drag-and-drop on KDE Plasma Wayland:** RetroPrint prefers the X11/XWayland window backend on Linux whenever `DISPLAY` is set. This routes Dolphin-to-window drops through X11 DnD events. XWayland is typically available in Plasma; install `xorg-xwayland` if needed. If `DISPLAY` is absent, the app keeps native Wayland as a startup fallback. Dropped files are read by path or by file bytes when the portal provides them. This behavior still needs an interactive KDE test.

```bash
cargo run --release
```

To open an image immediately (also useful for GUI smoke tests):

```bash
cargo run --release -- /path/to/image.png
```

Produce an executable without running:

```bash
cargo build --release
```

The binary will be `target/release/retroprint` on Linux. The same Cargo project can be built for Windows/macOS with their respective Rust toolchains and build dependencies.

The GitHub Actions artifact `retroprint-linux-x86_64.zip` includes a prebuilt Linux x86-64 executable named `retroprint` at the archive root. On a compatible glibc-based Linux desktop, extract the ZIP, then run:

```bash
chmod +x retroprint
./retroprint
```

That executable is a convenience build, not an AppImage or a statically linked universal Linux package. If it does not start because of graphics/session/runtime compatibility, compile locally with Cargo instead.

### Flatpak bundle

The release also includes `retroprint-linux-x86_64.flatpak`. With Flatpak installed, download the bundle and run:

```bash
flatpak install --user ./retroprint-linux-x86_64.flatpak
flatpak run io.github.tzerokyi.RetroPrint
```

Flatpak installs the executable with its launcher, so there is no separate `chmod` step. The first installation may download the Freedesktop 26.08 runtime from Flathub. This bundle uses X11/XWayland for the desktop window and the file chooser portal for file access; it does not request network access. A desktop with XWayland support is required. The bundle is distributed through GitHub Releases, not Flathub's app catalog.

The standalone bundle is a direct download, not a software-store listing or an automatic update source. Software centers may show limited publisher details or omit screenshots when inspecting the bundle. The project metadata includes a developer name and a screenshot URL for a future public Flatpak repository; the screenshot URL becomes accessible when this GitHub repository is public.

## Usage

1. Drag an image into the window or click **Open Image**.
2. Choose **Organic** or **Fine Stipple** in **Texture Mode**. Fine Stipple reduces unwanted random clusters in midtones and holds silhouettes/details more clearly; Organic retains softer analog clumping.
3. Use **Grain Size** to choose the scale and **Grain Amount** for texture strength. These settings remain available in both modes; Fine Stipple uses Grain Size for subtle secondary organic structure rather than the primary dot spacing.
4. Tune **Contrast**, **Threshold**, and **Irregularity** to preserve shape and create ink breakup.
5. Choose the **Ink** and **Paper** colors independently: click either swatch for its hue/saturation spectrum or type a six-digit HEX color (`#D45A36`); alternatively select a quick palette.
6. Optionally enable **Transparent Paper** to export only the printed ink. This is **not** semantic background removal: highlights and unprinted areas also become transparent.
7. Click **Export PNG** and select a file.

The app preserves original input alpha. The saved PNG uses the original image dimensions; previews are intentionally reduced for responsiveness. On export, the app checks how many distinct RGBA colors are actually used. It automatically encodes up to 256 unique colors as a lossless 1-, 2-, 4-, or 8-bit indexed PNG (typically only **1 bit per pixel** for opaque two-color prints); images with more than 256 RGBA colors use full RGBA PNG. This is pixel-for-pixel reversible, without color reduction, resampling, or manual RGB conversion. File sizes vary with image dimensions and texture complexity.

## Tests

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## Source layout

| File | Purpose |
| --- | --- |
| `src/main.rs` | GUI, asynchronous preview/export workers, drag-and-drop, file dialogs |
| `src/ui.rs` | Full-window UI layout, theme, embedded Lucide icons, preview rendering |
| `src/effect.rs` | CPU image processing and unit tests |
| `src/image_io.rs` | Import, previews, and PNG export |
| `src/presets.rs` | Default parameters and colors |
| `assets/icons/` | Official Lucide SVG icons and their upstream license notice |
| `assets/noise/` | Christoph Peters's CC0 blue-noise mask and license notice |
| `assets/retroprint-preview.png` | Screenshot of the running app |
| `flatpak/` | Flatpak manifest, launcher, icon, metadata, and offline Cargo sources |

## Limitations

- This is an original stipple approximation rather than a Photoshop filter-clone.
- No automatic object segmentation or background removal.
- No ICC color-management pipeline or guaranteed exact print color reproduction.
- Very large images can consume considerable memory before input limits are evaluated.
- Preview and output use the same normalized grain scale, but resampling means individual grain dots will not match pixel-for-pixel.

## License

The RetroPrint source is MIT-licensed (see `LICENSE`). The bundled Lucide icons retain their upstream ISC/MIT licensing and attribution; see `assets/icons/LUCIDE_LICENSE.txt`. The blue-noise mask is by Christoph Peters under CC0 1.0; see `assets/noise/LICENSE.txt`.
