# AboutThisApp – Wiki

AboutThisApp is the TontooOS About window: a fixed 340x460 TontooUI card
that inspects any `.app` bundle (TBuild ZIP) and shows its icon, localized
name and version, macOS-style. It follows the live system color scheme and
loads `en_us`/`de_de` strings from `lang/`.

- Repository: https://github.com/TontooOS/TontooOS
- License: TCL v26.1
- Version: 0.1.0

## Feature Index

| Feature | File | Description |
|---|---|---|
| Main index | [MAIN.md](MAIN.md) | This page |
| Rules | [RULE.md](RULE.md) | Development and usage rules |
| AboutThisApp | [AboutThisApp.md](AboutThisApp.md) | Bundle inspection, About card layout and localization |

## Quick Start

Show the About window for any `.app` bundle from the repository root:

```bash
cargo run -- "/Users/paul/Applications/Finder.app"
```

The window follows the GNOME system theme live (Dark `#1d1d1d`, Light
`#ececec`) and picks German strings when `LANG` starts with `de`.

See [AboutThisApp.md](AboutThisApp.md) for details.

## Changelog

- 2026-09-09: Initial AboutThisApp (bundle inspection + fixed About card + `lang/en_us.json` and `lang/de_de.json`).
