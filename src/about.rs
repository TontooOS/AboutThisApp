//! Target `.app` inspection for AboutThisApp.
//!
//! A TontooOS `.app` bundle is a ZIP archive (built by TBuild) containing a
//! top-level `<Name>.app/` directory with `Info.tontoo`, `App/` (binary plus
//! `icon.png`) and `Resources/` (including `icon.png` and `lang/`). This
//! module also accepts an already extracted `<Name>.app` directory, so both
//! installed bundles and build outputs work.
//!
//! `Info.tontoo` shape (written by TBuild):
//!
//! ```json
//! {
//!   "bundle_id": "com.tontoo.finder",
//!   "name": { "en_us": "Finder", "de_de": "Finder" },
//!   "version": "26.1.0"
//! }
//! ```

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Resolved display data of the target app.
#[derive(Debug, Clone)]
pub struct TargetInfo {
  /// Localized app name (falls back to the file stem without `.app`).
  pub display_name: String,
  /// Version string, empty when unknown.
  pub version: String,
  /// Extracted icon file path, when the bundle contained one.
  pub icon_path: Option<PathBuf>,
}

fn stem_fallback(path: &Path) -> String {
  path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("App")
    .to_string()
}

fn pick_localized_name(info: &serde_json::Value, locale: &str, fallback: &str) -> String {
  if let Some(names) = info.get("name") {
    if let Some(obj) = names.as_object() {
      if let Some(hit) = obj.get(locale).and_then(|v| v.as_str()) {
        if !hit.is_empty() {
          return hit.to_string();
        }
      }
      // Any available locale wins over the file stem.
      for key in ["en_us", "de_de"] {
        if let Some(hit) = obj.get(key).and_then(|v| v.as_str()) {
          if !hit.is_empty() {
            return hit.to_string();
          }
        }
      }
    }
    if let Some(plain) = names.as_str() {
      if !plain.is_empty() {
        return plain.to_string();
      }
    }
  }
  fallback.to_string()
}

fn version_of(info: &serde_json::Value) -> String {
  info
    .get("version")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string()
}

/// Read `Info.tontoo` from an extracted `<Name>.app` directory.
fn read_dir_bundle(dir: &Path, locale: &str) -> Option<TargetInfo> {
  let info_path = dir.join("Info.tontoo");
  let content = fs::read_to_string(&info_path).ok()?;
  let info: serde_json::Value = serde_json::from_str(&content).ok()?;
  let fallback = stem_fallback(dir);
  let icon = ["App/icon.png", "Resources/icon.png"]
    .iter()
    .map(|rel| dir.join(rel))
    .find(|p| p.is_file());
  Some(TargetInfo {
    display_name: pick_localized_name(&info, locale, &fallback),
    version: version_of(&info),
    icon_path: icon,
  })
}

/// Read `Info.tontoo` plus the icon from a zipped `.app` bundle.
///
/// Returns the parsed info and, when the bundle ships an icon, the icon bytes.
/// Icon lookup order mirrors TBuild: `App/icon.png` first, then
/// `Resources/icon.png`.
fn read_zip_bundle(path: &Path, locale: &str) -> Result<TargetInfo, String> {
  let file = fs::File::open(path)
    .map_err(|e| format!("cannot open '{}': {}", path.display(), e))?;
  let mut zip = zip::ZipArchive::new(file)
    .map_err(|e| format!("'{}' is not a valid .app bundle: {}", path.display(), e))?;

  let mut info_json: Option<serde_json::Value> = None;
  let mut icon_bytes: Option<Vec<u8>> = None;
  let mut icon_fallback: Option<Vec<u8>> = None;

  for i in 0..zip.len() {
    let mut entry = zip
      .by_index(i)
      .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;
    let name = entry.name().to_string();
    if name.ends_with("Info.tontoo") && info_json.is_none() {
      let mut buf = String::new();
      entry
        .read_to_string(&mut buf)
        .map_err(|e| format!("cannot read Info.tontoo: {}", e))?;
      info_json = serde_json::from_str::<serde_json::Value>(&buf).ok();
    } else if name.ends_with("App/icon.png") && icon_bytes.is_none() {
      let mut buf = Vec::new();
      entry.read_to_end(&mut buf).map_err(|e| format!("cannot read icon: {}", e))?;
      icon_bytes = Some(buf);
    } else if name.ends_with("Resources/icon.png") && icon_fallback.is_none() {
      let mut buf = Vec::new();
      entry.read_to_end(&mut buf).map_err(|e| format!("cannot read icon: {}", e))?;
      icon_fallback = Some(buf);
    }
  }

  let fallback = stem_fallback(path);
  let (display_name, version) = match info_json {
    Some(info) => (pick_localized_name(&info, locale, &fallback), version_of(&info)),
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
  let safe: String = app_name
    .chars()
    .map(|c| if c.is_alphanumeric() { c } else { '_' })
    .collect();
  let path = std::env::temp_dir().join(format!("about-this-app-icon-{}.png", safe));
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn stem_fallback_strips_app_extension() {
    let info = pick_localized_name(&serde_json::json!({}), "en_us", "Finder");
    assert_eq!(info, "Finder");
  }

  #[test]
  fn picks_locale_name_first() {
    let value = serde_json::json!({ "name": { "en_us": "Finder", "de_de": "FinderDE" } });
    assert_eq!(pick_localized_name(&value, "de_de", "x"), "FinderDE");
    assert_eq!(pick_localized_name(&value, "en_us", "x"), "Finder");
  }

  #[test]
  fn missing_target_errors() {
    let result = load_target(Path::new("/definitely/not/here.app"), "en_us");
    assert!(result.is_err());
  }
}
