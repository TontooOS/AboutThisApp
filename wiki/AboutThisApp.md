# AboutThisApp

AboutThisApp renders a macOS-style About window for any TontooOS `.app`
bundle path passed as the single CLI argument (e.g.
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
| `icon_path` | `Option<PathBuf>` | Raw extracted icon file, when the bundle ships one (`None` = use `fallback_icon`) |

### Rules

- Accepts both the zipped bundle (`Finder.app` file, as built by TBuild)
  and an already extracted `<Name>.app` directory.
- Reads `Info.tontoo` (`bundle_id`, localized `name` map, `version`).
- Name lookup order: requested locale (`en_us` or `de_de`), then any of
  `en_us`/`de_de`, then a plain string `name`, then the file stem.
- Missing `Info.tontoo` is not fatal: name falls back to the file stem and
  version stays empty (the version line is then hidden).
- Icon lookup order mirrors TBuild: `App/icon.png` first, then
  `Resources/icon.png`.
- ZIP icons are extracted to `std::env::temp_dir()` as
  `about-this-app-icon-<AppName>.png`.
- Every icon is finished through CoreIcon (see `## Icon Pipeline`).
- Returns `Err` when the path does not exist or the file is not a valid
  ZIP bundle.

## Icon Pipeline

Every bundle icon — and the fallback tile for bundles without an icon —
goes through CoreIcon (`CoreIcon::generator`), so a plain PNG is always
shown as a proper Apple-style app icon.

```rust
pub fn beautify_icon(raw: &Path, app_name: &str, dark: bool) -> Option<PathBuf>
pub fn fallback_icon(app_name: &str) -> Option<PathBuf>
```

| Function | Input | Output |
|---|---|---|
| `beautify_icon` | Raw bundle PNG | `about-this-app-icon-<AppName>-glass.png`: 1024x1024 squircle with the Liquid Glass depth finish (ambient + artwork shadow, inner bevel, specular rim, top gloss, vibrancy, bottom shade) |
| `fallback_icon` | App name only | `about-this-app-icon-<AppName>-fallback.png`: white/black gradient squircle with the Liquid Glass finish |

- `dark` selects the dark-mode background (`#1d1d1d`, artwork colors
  preserved); light mode keeps the original background.
- Both functions return `None` on failure: a failed beautify falls back to
  the raw PNG (shown with a CSS corner radius), a failed fallback falls
  back to a plain CSS gradient tile.
- `main` tracks whether the displayed file is CoreIcon-finished in a local
  `icon_glass` flag (already rounded with transparency, no CSS rounding
  needed) and passes it to the root view.

## Window

The window is a fixed, non-resizable 340x460 card with no scroll container
and no system decoration bar. `force_size` pins the exact size so oversized
content can never stretch the card.

```rust
let mut app = App::with_delegate(window_title, 340, 460, delegate);
app.auto_color_scheme();
app.no_window_bar();
app.fixed_size();
app.no_scroll();
app.force_size(340, 460);
app.run();
```

### Layout

| Element | Rule |
|---|---|
| `TitleBar` | Custom bar with `title("About {name}")` and `without_maximize()`, so only close and minimize show; both keep working via `__close` / `__minimize` |
| `Icon` | Centered 120px `GtkPicture` with `hexpand`/`vexpand` disabled so it can never stretch to fill the card; CoreIcon squircles shown as-is, raw PNGs with a 28px CSS corner radius |
| `Name` | Bold SF Pro Display 15px, centered |
| `Version` | SF Pro Display 12px in secondary gray, `Version {version}`; hidden entirely when the version is empty |

### Colors

| Mode | Background | Foreground | Secondary |
|---|---|---|---|
| Dark | `#1d1d1d` | `#F5F5F7` | `#A1A1A6` |
| Light | `#ececec` | `#1E1E1E` | `#6E6E73` |

- The scheme is read live via `current_color_scheme()` with
  `ColorScheme::detect_system()` as the fallback.
- All text uses `SF Pro Display` (loaded from the system font paths
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

Only `en_us` and `de_de` exist; the active locale follows `LANGUAGE`,
`LC_ALL`, `LANG` and `/etc/locale.conf`.

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
  "name": "AboutThisApp",
  "about.title": "About {name}",
  "about.version": "Version {version}",
  "about.usage": "Usage: about-this-app <path-to-app.app>"
}
```

## Usage / Example

```bash
cargo run -- "/Users/paul/Applications/Finder.app"
```

Shows `About Finder` with the Finder icon and `Version 26.1.0`.

## Cross References

- [MAIN.md](MAIN.md) – project overview and quick start
- [RULE.md](RULE.md) – wiki design system and repo rules
