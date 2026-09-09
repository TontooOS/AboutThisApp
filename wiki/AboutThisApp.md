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
| `icon_path` | `Option<PathBuf>` | Extracted icon file, when the bundle ships one |

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
- Returns `Err` when the path does not exist or the file is not a valid
  ZIP bundle.

## Window

The window is a fixed, non-resizable 340x460 card with no scroll container
and no system decoration bar.

```rust
let mut app = App::with_delegate(window_title, 340, 460, delegate);
app.auto_color_scheme();
app.no_window_bar();
app.fixed_size();
app.no_scroll();
app.run();
```

### Layout

| Element | Rule |
|---|---|
| `TitleBar` | Custom bar with `title("About {name}")` and `without_maximize()`, so only close and minimize show; both keep working via `__close` / `__minimize` |
| `Icon` | Centered 120px icon with 28px corners and a soft shadow; white/black gradient tile when the bundle has no icon |
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
