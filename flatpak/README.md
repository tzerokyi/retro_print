# Flatpak packaging

The manifest builds RetroPrint from source with the Freedesktop 26.08 SDK and its Rust extension. Cargo dependencies are downloaded by Flatpak Builder from checksummed entries in `cargo-sources.json`, then compiled offline inside the build sandbox.

## Local bundle

Install Flatpak and enable Flathub, then install the builder and SDK components:

```bash
flatpak install --user flathub org.flatpak.Builder org.freedesktop.Sdk//26.08 org.freedesktop.Sdk.Extension.rust-stable//26.08
```

From the repository root, build and bundle:

```bash
flatpak run org.flatpak.Builder --user --force-clean --install-deps-from=flathub --repo=dist/flatpak-repo dist/flatpak-build flatpak/io.github.tzerokyi.RetroPrint.json
flatpak build-bundle dist/flatpak-repo dist/retroprint-linux-x86_64.flatpak io.github.tzerokyi.RetroPrint --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
```

Install and run the bundle:

```bash
flatpak install --user ./dist/retroprint-linux-x86_64.flatpak
flatpak run io.github.tzerokyi.RetroPrint
```

The application has X11/XWayland and GPU access. File dialogs use the desktop portal. The application has no network permission.

A single-file bundle does not include the repository AppStream catalog used by software centers. The application MetaInfo includes the developer and a screenshot URL, but a complete store page and automatic updates require a published Flatpak repository or Flathub. The screenshot URL will work after the GitHub repository becomes public.

## Updating Cargo dependencies

When `Cargo.lock` changes, regenerate `cargo-sources.json` with the [Flatpak Cargo generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo):

```bash
python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
```

Keep the generated file in the repository so release builds can run without Cargo network access inside the sandbox.
