//! Target `.app` inspection for AboutThisApp.
//!
//! A TontooOS `.app` bundle is a ZIP archive (built by TBuild) containing a
//! top-level `<Name>.app/` directory with `Info.tontoo`, `App/` (binary plus
//! `icon.png`) and `Resources/` (including `icon.png` and `lang/`). This
//! module also accepts an already extracted `<Name>.app` directory, so both
//! installed bundles and build outputs work.
//!
//! `Info.tontoo` shape (written by TBuild), parsed with Foundation
//! (`JsonDocument`, no serde usage in this crate):
//!
//! ```json
//! {
//!   "bundle_id": "com.tontoo.finder",
//!   "name": { "en_us": "Finder", "de_de": "Finder" },
//!   "version": "26.1.0"
//! }
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::ArchiveKit::zip_unpack;
use crate::Foundation::serialization::JsonDocument;

/// Resolved display data of the target app.
#[derive(Debug, Clone)]
pub struct TargetInfo {
  /// Localized app name (falls back to the file stem without `.app`).
  pub display_name: String,
  /// Version string, empty when unknown.
  pub version: String,
  /// Extracted icon file path, when the bundle contained one.
  /// Always the raw bundle file; callers finish it through CoreIcon
  /// (`beautify_icon`).
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

fn pick_localized_name(doc: &JsonDocument, locale: &str, fallback: &str) -> String {
  // A plain string `name` wins over everything.
  if let Ok(Some(plain)) = doc.str_field("name") {
    if !plain.is_empty() {
      return plain;
    }
  }
  if let Ok(Some(names)) = doc.nested("name") {
    // Requested locale first, then any available locale.
    for key in [locale, "en_us", "de_de"] {
      if let Ok(Some(hit)) = names.str_field(key) {
        if !hit.is_empty() {
          return hit;
        }
      }
    }
  }
  fallback.to_string()
}

fn version_of(doc: &JsonDocument) -> String {
  doc
    .str_field("version")
    .ok()
    .flatten()
    .unwrap_or_default()
}

/// Read `Info.tontoo` from an extracted `<Name>.app` directory.
fn read_dir_bundle(dir: &Path, locale: &str) -> Option<TargetInfo> {
  let content = fs::read_to_string(dir.join("Info.tontoo")).ok()?;
  let doc = JsonDocument::parse(&content).ok()?;
  let fallback = stem_fallback(dir);
  let icon = ["App/icon.png", "Resources/icon.png"]
    .iter()
    .map(|rel| dir.join(rel))
    .find(|p| p.is_file());
  Some(TargetInfo {
    display_name: pick_localized_name(&doc, locale, &fallback),
    version: version_of(&doc),
    icon_path: icon,
  })
}

/// Read `Info.tontoo` plus the icon from a zipped `.app` bundle with
/// ArchiveKit (Stored + Deflate).
///
/// Returns the parsed info and, when the bundle ships an icon, the icon bytes
/// written to a temp file. Icon lookup order mirrors TBuild:
/// `App/icon.png` first, then `Resources/icon.png`.
fn read_zip_bundle(path: &Path, locale: &str) -> Result<TargetInfo, String> {
  let bytes = fs::read(path)
    .map_err(|e| format!("cannot open '{}': {}", path.display(), e))?;
  let entries = zip_unpack(&bytes)
    .map_err(|e| format!("'{}' is not a valid .app bundle: {}", path.display(), e))?;

  let mut info_doc: Option<JsonDocument> = None;
  let mut icon_bytes: Option<Vec<u8>> = None;
  let mut icon_fallback: Option<Vec<u8>> = None;

  for entry in &entries {
    if entry.is_dir() {
      continue;
    }
    if entry.name.ends_with("Info.tontoo") && info_doc.is_none() {
      if let Ok(text) = String::from_utf8(entry.data.clone()) {
        info_doc = JsonDocument::parse(&text).ok();
      }
    } else if entry.name.ends_with("App/icon.png") && icon_bytes.is_none() {
      icon_bytes = Some(entry.data.clone());
    } else if entry.name.ends_with("Resources/icon.png") && icon_fallback.is_none() {
      icon_fallback = Some(entry.data.clone());
    }
  }

  let fallback = stem_fallback(path);
  let (display_name, version) = match info_doc {
    Some(doc) => (
      pick_localized_name(&doc, locale, &fallback),
      version_of(&doc),
    ),
    None => (fallback, String::new()),
  };

  let icon_path = icon_bytes
    .or(icon_fallback)
    .and_then(|bytes| write_temp_icon(&display_name, &bytes));

  Ok(TargetInfo {
    display_name,
    version,
    icon_path,
  })
}

fn write_temp_icon(app_name: &str, bytes: &[u8]) -> Option<PathBuf> {
  let path = std::env::temp_dir().join(format!("about-this-app-icon-{}.png", safe_name(app_name)));
  if fs::write(&path, bytes).is_ok() {
    Some(path)
  } else {
    None
  }
}

/// Load the display info for the target `.app` path.
///
/// Accepts both the zipped bundle (`Finder.app` file) and an extracted
/// `<Name>.app` directory. Missing `Info.tontoo` is not fatal: the file stem
/// is used as the name and the version stays empty.
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
  read_zip_bundle(path, locale)
}

/// Run a raw bundle icon through CoreIcon so a plain PNG becomes a proper
/// Apple-style app icon: 1024x1024 squircle with the Liquid Glass depth
/// finish (ambient + artwork shadow, inner bevel, specular rim, top gloss,
/// vibrancy pop, grounding shade). `dark` selects the dark-mode background
/// (artwork colors preserved), otherwise the original background is kept.
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

  fn doc(json: &str) -> JsonDocument {
    JsonDocument::parse(json).expect("valid test json")
  }

  #[test]
  fn stem_fallback_when_no_name() {
    let info = pick_localized_name(&doc("{}"), "en_us", "Finder");
    assert_eq!(info, "Finder");
  }

  #[test]
  fn picks_locale_name_first() {
    let value = doc(r#"{ "name": { "en_us": "Finder", "de_de": "FinderDE" } }"#);
    assert_eq!(pick_localized_name(&value, "de_de", "x"), "FinderDE");
    assert_eq!(pick_localized_name(&value, "en_us", "x"), "Finder");
  }

  #[test]
  fn plain_string_name_wins() {
    let value = doc(r#"{ "name": "Plain", "version": "1.2" }"#);
    assert_eq!(pick_localized_name(&value, "de_de", "x"), "Plain");
    assert_eq!(version_of(&value), "1.2");
  }

  #[test]
  fn missing_target_errors() {
    let result = load_target(Path::new("/definitely/not/here.app"), "en_us");
    assert!(result.is_err());
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
