//! Target `.app` inspection for AboutThisApp.
//!
//! A TontooOS `.app` is a TAPP single-file container (built by TBuild) with
//! a `<Name>.app/` top directory holding `Info.tontoo` (fico manifest),
//! `App/` (binary plus `icon.tico`) and `Resources/` (including
//! `icon.tico` and `lang/`). This module also accepts an already extracted
//! `<Name>.app` directory, so both single-file containers and installed
//! bundles work.
//!
//! Containers are opened by index: only the manifest and the icon entries
//! are read, assets are never touched. `Info.tontoo` shape (fico):
//!
//! ```text
//! app {
//!   bundle_id: com.tontoo.finder
//!   version: "26.1.0"
//!   executable: App/finder
//!   icon: App/icon.tico
//!   name {
//!     en_us: "Finder"
//!     de_de: "Finder"
//!   }
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::ArchiveKit::{AppManifest, AppReader};

/// TAPP icon probes (container-relative) when the manifest has no icon.
const ICON_PROBES: [&str; 2] = ["App/icon.tico", "Resources/icon.tico"];

/// Resolved display data of the target app.
#[derive(Debug, Clone)]
pub struct TargetInfo {
  /// Localized app name (falls back to the file stem without `.app`).
  pub display_name: String,
  /// Version string, empty when unknown.
  pub version: String,
  /// Extracted icon file path, when the bundle contained one.
  /// Always a temp PNG/TICO file; callers finish it through CoreIcon
  /// (`beautify_icon`) and track the glass state themselves.
  pub icon_path: Option<PathBuf>,
}

fn safe_name(app_name: &str) -> String {
  app_name
    .chars()
    .map(|c| if c.is_alphanumeric() { c } else { '_' })
    .collect()
}

fn stem_fallback(path: &Path) -> String {
  path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("App")
    .to_string()
}

fn pick_localized_name(manifest: &AppManifest, locale: &str, fallback: &str) -> String {
  if let Some(hit) = manifest.name(locale) {
    if !hit.is_empty() {
      return hit.to_string();
    }
  }
  // Any available locale wins over the file stem.
  for key in ["en_us", "de_de"] {
    if let Some(hit) = manifest.name(key) {
      if !hit.is_empty() {
        return hit.to_string();
      }
    }
  }
  if let Some(first) = manifest.display_name() {
    if !first.is_empty() {
      return first.to_string();
    }
  }
  fallback.to_string()
}

/// Read `Info.tontoo` (fico) from an extracted `<Name>.app` directory.
fn read_dir_bundle(dir: &Path, locale: &str) -> Option<TargetInfo> {
  let text = fs::read_to_string(dir.join("Info.tontoo")).ok()?;
  let manifest = AppManifest::from_fico(&text).ok()?;
  let fallback = stem_fallback(dir);
  let icon = manifest
    .icon
    .as_deref()
    .map(|entry| dir.join(entry))
    .filter(|p| p.is_file())
    .or_else(|| {
      ICON_PROBES
        .iter()
        .map(|rel| dir.join(rel))
        .find(|p| p.is_file())
    });
  Some(TargetInfo {
    display_name: pick_localized_name(&manifest, locale, &fallback),
    version: manifest.version.clone(),
    icon_path: icon,
  })
}

/// Read the manifest plus the icon from a TAPP `.app` container.
///
/// Only the footer, the central directory, the manifest and the icon
/// entries are touched; the binary and assets are never read. The icon
/// entry is written to a temp file for the CoreIcon pipeline.
fn read_container(path: &Path, locale: &str) -> Result<TargetInfo, String> {
  let mut reader = AppReader::open(path)
    .map_err(|e| format!("'{}' is not a valid .app container: {}", path.display(), e))?;
  let manifest = reader
    .read_manifest()
    .map_err(|e| format!("cannot read Info.tontoo in '{}': {}", path.display(), e))?;
  let top = reader
    .manifest_name()
    .and_then(|entry| entry.strip_suffix("Info.tontoo"))
    .unwrap_or("")
    .to_string();

  let fallback = stem_fallback(path);
  let display_name = pick_localized_name(&manifest, locale, &fallback);

  let mut icon_entry = manifest.icon.clone();
  if icon_entry.is_none() {
    icon_entry = ICON_PROBES
      .iter()
      .map(|rel| format!("{top}{rel}"))
      .find(|full| reader.find(full).is_some_and(|meta| !meta.is_dir()))
      .map(|full| full.strip_prefix(&top).unwrap_or(&full).to_string());
  }
  let icon_path = icon_entry.and_then(|rel| {
    let full = format!("{top}{rel}");
    let bytes = reader.read_file(&full).ok()?;
    let file_name = Path::new(&rel)
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_else(|| "icon.tico".to_string());
    write_temp_icon(&display_name, &file_name, &bytes)
  });

  Ok(TargetInfo {
    display_name,
    version: manifest.version.clone(),
    icon_path,
  })
}

fn write_temp_icon(app_name: &str, file_name: &str, bytes: &[u8]) -> Option<PathBuf> {
  let path = std::env::temp_dir().join(format!(
    "about-this-app-icon-{}-{}",
    safe_name(app_name),
    file_name
  ));
  if fs::write(&path, bytes).is_ok() {
    Some(path)
  } else {
    None
  }
}

/// Load the display info for the target `.app` path.
///
/// Accepts both the single-file TAPP container (`Finder.app` file) and an
/// extracted `<Name>.app` directory. Missing `Info.tontoo` in a directory
/// is not fatal: the file stem is used as the name and the version stays
/// empty.
pub fn load_target(path: &Path, locale: &str) -> Result<TargetInfo, String> {
  if !path.exists() {
    return Err(format!("'{}' does not exist", path.display()));
  }
  if path.is_dir() {
    if let Some(info) = read_dir_bundle(path, locale) {
      return Ok(info);
    }
    return Ok(TargetInfo {
      display_name: stem_fallback(path),
      version: String::new(),
      icon_path: None,
    });
  }
  read_container(path, locale)
}

/// Render a `.tico` icon entry into a plain PNG via CoreIcon (1024px,
/// Apple app-icon finish baked in). Returns the PNG path, or `None` when
/// loading or rendering fails.
pub fn tico_to_png(tico: &Path, app_name: &str) -> Option<PathBuf> {
  use crate::CoreIcon::tico::Tico;
  let icon = Tico::load(tico).ok()?;
  let image = icon.render(1024, None).ok()?;
  let out = std::env::temp_dir().join(format!(
    "about-this-app-icon-{}-tico.png",
    safe_name(app_name)
  ));
  image.save(&out).ok()?;
  Some(out)
}

/// Run a raw bundle icon through CoreIcon so a plain file becomes a proper
/// Apple-style app icon: 1024x1024 squircle with the Liquid Glass depth
/// finish (ambient + artwork shadow, inner bevel, specular rim, top gloss,
/// vibrancy pop, grounding shade). `.tico` entries render through the
/// CoreIcon pipeline; other files load as plain artwork. `dark` selects
/// the dark-mode background (artwork colors preserved), otherwise the
/// original background is kept.
///
/// Returns the finished PNG path, or `None` when processing fails (callers
/// fall back to the raw icon).
pub fn beautify_icon(raw: &Path, app_name: &str, dark: bool) -> Option<PathBuf> {
  let mut icon = crate::CoreIcon::generator::AppIcon::from_file(raw);
  icon = if dark { icon.dark() } else { icon.light() };
  let out = std::env::temp_dir().join(format!(
    "about-this-app-icon-{}-glass.png",
    safe_name(app_name)
  ));
  icon.save(&out).ok()?;
  Some(out)
}

/// Generate a CoreIcon fallback tile for bundles without an icon: a
/// white/black gradient squircle with the Apple Liquid Glass finish.
/// Returns the PNG path, or `None` when generation fails (callers fall back
/// to the `FileImage` theme placeholder).
pub fn fallback_icon(app_name: &str) -> Option<PathBuf> {
  use crate::CoreIcon::generator::{Background, IconCanvas};
  use crate::CoreIcon::{Color, Gradient};
  let canvas = IconCanvas::new()
    .background(Background::Gradient(Gradient::linear_two(
      Color::WHITE,
      Color::BLACK,
    )))
    .glass();
  let out = std::env::temp_dir().join(format!(
    "about-this-app-icon-{}-fallback.png",
    safe_name(app_name)
  ));
  canvas.save(&out).ok()?;
  Some(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ArchiveKit::{AppBuilder, AppManifest};

  fn manifest(locale_name: &str) -> AppManifest {
    let mut manifest = AppManifest::new("com.tontoo.demo", "2.0", "App/demo");
    manifest.icon = Some("App/icon.tico".to_string());
    manifest
      .names
      .push(("en_us".to_string(), locale_name.to_string()));
    manifest
  }

  fn test_tico() -> Vec<u8> {
    use crate::CoreIcon::generator::{Background, IconCanvas, Layer, LayerContent};
    use crate::CoreIcon::{Color, tico::Tico};
    let canvas = IconCanvas::new()
      .background(Background::color(Color::BLACK))
      .layer(Layer::new(LayerContent::circle(512.0)))
      .glass();
    let path = std::env::temp_dir().join("about-this-app-test-icon.tico");
    Tico::export(&canvas, "t", &path).expect("tico export");
    fs::read(&path).expect("read tico")
  }

  fn write_container(dir: &Path, asset_bytes: Option<Vec<u8>>) -> PathBuf {
    let path = dir.join("Demo.app");
    let mut builder = AppBuilder::new("Demo").expect("builder");
    builder.set_manifest(manifest("Demo"));
    builder
      .add_executable("App/demo", b"binary".to_vec())
      .unwrap();
    builder
      .add_icon_tico("App/icon.tico", test_tico())
      .unwrap();
    if let Some(asset) = asset_bytes {
      builder.add_file("Resources/big-asset.bin", asset).unwrap();
    }
    builder.write_to_file(&path).expect("write container");
    path
  }

  #[test]
  fn stem_fallback_strips_app_extension() {
    let info = pick_localized_name(&manifest("x"), "en_us", "Finder");
    assert_eq!(info, "x");
  }

  #[test]
  fn picks_locale_name_first() {
    let mut with_de = manifest("Finder");
    with_de
      .names
      .push(("de_de".to_string(), "FinderDE".to_string()));
    assert_eq!(pick_localized_name(&with_de, "de_de", "x"), "FinderDE");
    assert_eq!(pick_localized_name(&with_de, "en_us", "x"), "Finder");
  }

  #[test]
  fn missing_target_errors() {
    let result = load_target(Path::new("/definitely/not/here.app"), "en_us");
    assert!(result.is_err());
  }

  #[test]
  fn container_reads_manifest_and_icon_only() {
    let work = std::env::temp_dir().join("about-this-app-test");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    let container = write_container(&work, Some(vec![0u8; 8192]));

    let info = load_target(&container, "en_us").expect("load container");
    assert_eq!(info.display_name, "Demo");
    assert_eq!(info.version, "2.0");
    let icon = info.icon_path.expect("icon extracted");
    assert!(icon.is_file());
    assert_eq!(icon.extension().and_then(|s| s.to_str()), Some("tico"));
    // No temp binary: assets and the executable were never touched.
    let temp: Vec<_> = fs::read_dir(std::env::temp_dir())
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| {
        e.file_name()
          .to_string_lossy()
          .starts_with("about-this-app-icon-Demo-")
      })
      .collect();
    assert_eq!(temp.len(), 1);
    let _ = fs::remove_dir_all(&work);
  }

  #[test]
  fn dir_bundle_reads_fico() {
    let root = std::env::temp_dir().join("about-this-app-test-dir");
    let _ = fs::remove_dir_all(&root);
    let bundle = root.join("Demo.app");
    fs::create_dir_all(bundle.join("App")).unwrap();
    fs::write(
      bundle.join("Info.tontoo"),
      "app {\n  bundle_id: com.tontoo.demo\n  version: \"2.0\"\n  executable: App/demo\n  name {\n    en_us: \"Demo\"\n  }\n}\n",
    )
    .unwrap();
    let info = load_target(&bundle, "de_de").expect("load dir");
    assert_eq!(info.display_name, "Demo");
    assert_eq!(info.version, "2.0");
    assert!(info.icon_path.is_none());
    let _ = fs::remove_dir_all(&root);
  }

  #[test]
  fn tico_renders_to_png() {
    use crate::CoreIcon::generator::{Background, IconCanvas, Layer, LayerContent};
    use crate::CoreIcon::{Color, tico::Tico};
    let canvas = IconCanvas::new()
      .background(Background::color(Color::BLACK))
      .layer(Layer::new(LayerContent::circle(512.0)))
      .glass();
    let path = std::env::temp_dir().join("about-this-app-test-render.tico");
    Tico::export(&canvas, "t", &path).expect("tico export");
    let png = tico_to_png(&path, "RenderTest").expect("tico renders");
    let bytes = fs::read(&png).expect("read png");
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
  }

  #[test]
  fn coreicon_roundtrip_produces_glass_png() {
    // No fixture needed: generate the fallback tile, then run it through
    // the beautify pipeline (pure image ops, no display required).
    let fallback = fallback_icon("RoundtripTest").expect("fallback icon");
    assert!(fallback.is_file());
    let glass = beautify_icon(&fallback, "RoundtripTest", true).expect("glass icon");
    assert!(glass.is_file());
    let bytes = std::fs::read(&glass).expect("read glass png");
    // PNG signature + IHDR width/height 1024x1024.
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert_eq!((w, h), (1024, 1024));
  }
}
