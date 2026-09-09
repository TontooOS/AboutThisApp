# AboutThisApp

macOS-style About window for any TontooOS `.app` bundle. Run it with the
target bundle as the single argument:

```bash
cargo run -- "/Users/paul/Applications/Finder.app"
```

Shows a fixed 340x460 card (no maximize button, minimize and close keep
working) with the centered 120px app icon (white/black gradient fallback),
the bold localized app name and a `Version %version%` line read from the
bundle `Info.tontoo`. Title is `About %appname%` (`en_us` / `Über` in
`de_de`).

Wiki: [wiki/MAIN.md](wiki/MAIN.md)

## Made for TontooOS

Explore more at https://github.com/TontooOS/Libs

## Adding to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
sdk = { path = "/Library/System/sdk", features = ["TontooUI"] }
```

Then at the crate root:

```rust
sdk::preinclude!();
use TontooUI::{TitleBar};
```

## License

TCL v26.1
