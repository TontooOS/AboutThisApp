# AboutThisApp

AboutThisApp renders a macOS-style About window for any TontooOS `.app`
container path passed as the single CLI argument (e.g.
`about-this-app "/Users/paul/Applications/Finder.app"`).

## Invocation

```rust
fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  assert!(args.len() == 1 && args[0].to_lowercase().ends_with(".app"));
}
```

- Takes exactly one positional argument ending in `.app`.
- Prints the localized `about.usage` string to stderr and exits with code
  `2` when the argument is missing or has no `.app` extension.
- Prints the IO error to stderr and exits with code `1` when the target
  cannot be read.
- Never runs without an argument; there is no self-info fallback.

## Bundle Inspection

`about::load_target` resolves the display data of the target bundle.

```rust
pub fn load_target(path: &Path, locale: &str) -> Result<TargetInfo, String>
```

### TargetInfo

| Field | Type | Description |
|---|---|---|
| `display_name` | `String` | Localized app name, or the file stem without `.app` |
| `version` | `String` | Version string, empty when unknown |
| `icon_path` | `Option<PathBuf>` | Extracted icon file (temp `.tico`/PNG), when the bundle ships one (`None` = use `fallback_icon`) |

### Rules

- Accepts both the single-file TAPP container (`Finder.app` file, as built
  by TBuild) and an already extracted `<Name>.app` directory.
- Containers are opened by index: only the footer, the central directory,
  the manifest and the icon entries are read; binary and assets are never
  touched.
- Reads the fico `Info.tontoo` (`bundle_id`, `version`, `executable`,
  `icon`, localized `name` table) with `AppManifest`; no serde usage in
  this crate.
- Name lookup order: requested locale (`en_us` or `de_de`), then any of
  `en_us`/`de_de`, then the first available name, then the file stem.
- Missing `Info.tontoo` in a directory is not fatal: name falls back to
  the file stem and version stays empty (the version line is then hidden).
- Icon lookup: the manifest `icon` entry first, then `App/icon.tico` and
  `Resources/icon.tico` under the container top prefix.
- Container icons are written to `std::env::temp_dir()` as
  `about-this-app-icon-<AppName>-<file>` for the CoreIcon pipeline.
- Every icon is finished through CoreIcon (see `## Icon Pipeline`).
- Returns `Err` when the path does not exist or the file is not a valid
  TAPP container.

## Icon Pipeline

Every bundle icon — and the fallback tile for bundles without an icon —
goes through CoreIcon, so it is always shown as a proper Apple-style app
icon.

```rust
pub fn tico_to_png(tico: &Path, app_name: &str) -> Option<PathBuf>
pub fn beautify_icon(raw: &Path, app_name: &str, dark: bool) -> Option<PathBuf>
pub fn fallback_icon(app_name: &str) -> Option<PathBuf>
```

| Function | Input | Output |
|---|---|---|
| `tico_to_png` | `.tico` container entry | `about-this-app-icon-<AppName>-tico.png`: 1024px render with the Apple app-icon finish baked in |
| `beautify_icon` | Raw artwork file (PNG, ...) | `about-this-app-icon-<AppName>-glass.png`: 1024x1024 squircle with the Liquid Glass depth finish (ambient + artwork shadow, inner bevel, specular rim, top gloss, vibrancy, bottom shade) |
| `fallback_icon` | App name only | `about-this-app-icon-<AppName>-fallback.png`: white/black gradient squircle with the Liquid Glass finish |

- `.tico` entries render straight to PNG (already finished, no beautify);
  plain files go through `beautify_icon`.
- `dark` selects the dark-mode background (artwork colors preserved);
  light mode keeps the original background. The theme is probed once from
  the settings daemon at startup.
- All functions return `None` on failure: a failed render/beautify falls
  back to the raw file (shown with the same 28px `FileImage` corner
  radius), a failed fallback falls back to the `FileImage` theme
  placeholder box.

## Window

The window is a 340x460 card rendered with TontooUI on Vello/WGPU. The
maximize light is disabled, so the size stays fixed in practice; there is
no scroll container and no system decoration bar.

```rust
let app = AboutApp::new(window_title, display_name, version_line, icon);
run(&window_title, 340, 460, app)?;
```

### Layout

| Element | Rule |
|---|---|
| `Titlebar` | `Titlebar::new("About {name}").without_maximize()`: the green light is gray and ignores clicks; close and minimize map to `WindowCommand::Close` / `WindowCommand::Minimize` |
| `Icon` | Centered 120px `FileImage` with a 28px corner radius, cover-fit; shows the CoreIcon-finished temp PNG, the raw PNG when beautify fails, or the theme placeholder when no icon exists |
| `Name` | Bold 15px (`FormattedText` bold span, `Subheadline`), centered, wraps inside 276px |
| `Version` | 12px secondary (`BasicText`, `Caption`, `TextForeground::Secondary`), `Version {version}`; hidden entirely when the version is empty |

### Colors

| Mode | Background | Foreground | Secondary |
|---|---|---|---|
| Dark | `#1B2022` | `#D8D9D9` | theme dim |
| Light | `#FFFFFF` | `#272727` | theme dim |

- The scheme follows the settings daemon live through `ThemeWatcher`
  (with focus fade); the startup icon bake probes it once via
  `poll(0.0)`.
- All text uses SF Pro (system font, loaded from the system font paths
  `/usr/share/fonts/OTF/` and `/usr/share/fonts/TTF/`).

## Bundle Resources

| Path | Description |
|---|---|
| `Resources/app-icon.png` | Own app icon, referenced by `tontoo.proj` (`"icon": "Resources/app-icon.png"`); TBuild copies it to `App/icon.png` and `Resources/icon.png` |
| `Resources/lang/en_us.json` | Bundled English strings (mirror of `lang/en_us.json`) |
| `Resources/lang/de_de.json` | Bundled German strings (mirror of `lang/de_de.json`) |

- Root `lang/` stays canonical: TBuild reads `Info.tontoo` names from
  `project/lang/`, and dev runs (`cargo run`) load it via `cwd/lang`.
- `Resources/lang/` ships inside the bundle; the runtime also checks
  `cwd/Resources/lang` and `<bundle>/Resources/lang`.
- Keep both copies in sync when strings change.

## Localization

Only `en_us` and `de_de` exist, loaded through Accessibility
(`LangFile` / `LangStore`); the active locale follows `LANGUAGE`,
`LC_ALL`, `LANG` and `/etc/locale.conf`. Files use the Accessibility
shape (`{"lang": ..., "translations": {...}}`); `{name}` and `{version}`
placeholders are replaced by the callers.

Lookup order for the `lang/` folder: `$ABOUT_THIS_APP_LANG_DIR` first
(bare binary runs outside the project dir), then `cwd/lang`,
`cwd/Resources/lang`, the binary dir and its parent (including
`<bundle>/Resources/lang` inside an installed `.app`), then
`/usr/share/about-this-app/lang`. When no file is found every lookup
returns the raw key (visible failure, e.g. `about.title` in the title
bar) instead of silently falling back.

| Key | `en_us` | `de_de` |
|---|---|---|
| `name` | `AboutThisApp` | `AboutThisApp` |
| `about.title` | `About {name}` | `Über {name}` |
| `about.version` | `Version {version}` | `Version {version}` |
| `about.usage` | `Usage: about-this-app <path-to-app.app>` | `Aufruf: about-this-app <Pfad-zur-App.app>` |

The target app name inside `about.title` is the localized bundle name, not
the `name` key above.

```json
{
  "lang": "en_us",
  "translations": {
    "name": "AboutThisApp",
    "about.title": "About {name}",
    "about.version": "Version {version}",
    "about.usage": "Usage: about-this-app <path-to-app.app>"
  }
}
```

## Usage / Example

```bash
cargo run -- "/Users/paul/Applications/Finder.app"
```

Shows `About Finder` with the Finder icon and `Version 27.0.0`.

## Cross References

- [MAIN.md](MAIN.md) – project overview and quick start
- [RULE.md](RULE.md) – wiki design system and repo rules
