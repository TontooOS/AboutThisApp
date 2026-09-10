//! AboutThisApp: macOS-style About window for TontooOS `.app` bundles.
//!
//! Usage: `about-this-app "<path-to-app>.app"` (e.g.
//! `about-this-app "/Users/paul/Applications/Finder.app"`).
//!
//! Reads `Info.tontoo` (localized `name`, `version`) and the icon
//! (`App/icon.png`, then `Resources/icon.png`) from the target bundle and
//! shows a fixed 340x460 card: title bar without maximize button, centered
//! 120px icon (every icon is finished through CoreIcon into an Apple-style
//! squircle with the Liquid Glass depth effect), bold app name and a
//! `Version %version%` line. All text uses SF Pro Display with `en_us` and
//! `de_de` strings from `lang/`.

mod about;
mod lang;

sdk::preinclude!();

use UIKit::prelude::*;
use UIKit::widget::{WidgetId, next_widget_id};
use gtk::prelude::*;
use std::path::PathBuf;

const SF_PRO: &str = "SF Pro Display";

struct AboutDelegate {
  window_title: String,
  app_name: String,
  version: String,
  icon_path: Option<PathBuf>,
  icon_glass: bool,
}

impl AppDelegate for AboutDelegate {
  fn view(&self) -> Box<dyn Widget> {
    Box::new(AboutRoot {
      id: next_widget_id(),
      window_title: self.window_title.clone(),
      app_name: self.app_name.clone(),
      version: self.version.clone(),
      icon_path: self.icon_path.clone(),
      icon_glass: self.icon_glass,
    })
  }
}

fn markup_label(text: &str, size: u32, weight: &str, color: &str) -> gtk::Label {
  let label = gtk::Label::new(None);
  label.set_use_markup(true);
  label.set_halign(gtk::Align::Center);
  label.set_justify(gtk::Justification::Center);
  label.set_wrap(true);
  label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
  label.set_markup(&format!(
    "<span font_desc=\"{} {} {}\" foreground=\"{}\">{}</span>",
    SF_PRO,
    weight,
    size,
    color,
    glib::markup_escape_text(text),
  ));
  label
}

/// Centered 120px app icon. CoreIcon-finished squircles (`glass`) are shown
/// as-is (rounded with transparency baked in); raw square PNGs get a CSS
/// corner radius so they never render as a hard square.
fn icon_widget(icon_path: Option<&PathBuf>, glass: bool, dark: bool) -> gtk::Widget {
  if let Some(path) = icon_path {
    if let Some(path_str) = path.to_str() {
      let picture = gtk::Picture::for_filename(path_str);
      // Fixed card size: never expand, always centered (GtkPicture would
      // otherwise stretch to fill the content width).
      picture.set_size_request(120, 120);
      picture.set_hexpand(false);
      picture.set_vexpand(false);
      picture.set_halign(gtk::Align::Center);
      picture.set_valign(gtk::Align::Center);
      picture.set_content_fit(gtk::ContentFit::Contain);
      if !glass {
        crate::UIKit::widget::apply_css(
          &picture,
          ".about-icon { border-radius: 28px; box-shadow: 0 2px 12px rgba(0,0,0,0.18); }",
        );
        picture.add_css_class("about-icon");
      }
      return picture.upcast();
    }
  }
  // Fallback: plain white/black gradient tile (120px, rounded 28px).
  let tile = gtk::Box::new(gtk::Orientation::Vertical, 0);
  tile.set_size_request(120, 120);
  tile.set_halign(gtk::Align::Center);
  tile.set_valign(gtk::Align::Center);
  let edge = if dark { "rgba(255,255,255,0.20)" } else { "rgba(0,0,0,0.16)" };
  crate::UIKit::widget::apply_css(
    &tile,
    &format!(
      ".about-icon-fallback {{ background: linear-gradient(180deg, #FFFFFF, #000000); \
       border-radius: 28px; border: 1px solid {}; \
       box-shadow: 0 2px 12px rgba(0,0,0,0.18); }}",
      edge
    ),
  );
  tile.add_css_class("about-icon-fallback");
  tile.upcast()
}

/// Root widget: custom title bar (no maximize) plus centered about card.
struct AboutRoot {
  id: WidgetId,
  window_title: String,
  app_name: String,
  version: String,
  icon_path: Option<PathBuf>,
  icon_glass: bool,
}

impl Widget for AboutRoot {
  fn id(&self) -> WidgetId {
    self.id
  }

  fn to_gtk(&self) -> gtk::Widget {
    let dark = crate::UIKit::app::current_color_scheme()
      .unwrap_or_else(ColorScheme::detect_system)
      == ColorScheme::Dark;
    let (bg, fg, secondary) = if dark {
      ("#1d1d1d", "#F5F5F7", "#A1A1A6")
    } else {
      ("#ececec", "#1E1E1E", "#6E6E73")
    };

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_hexpand(true);
    outer.set_vexpand(true);
    crate::UIKit::widget::apply_css(&outer, &format!(".about {{ background-color: {}; }}", bg));
    outer.add_css_class("about");

    // Title bar with close + minimize only (no green maximize button).
    let bar = TontooUI::TitleBar::new()
      .title(self.window_title.clone())
      .without_maximize();
    let bar_widget = bar.to_gtk();
    outer.append(&bar_widget);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.set_margin_top(24);
    content.set_margin_bottom(28);
    content.set_margin_start(32);
    content.set_margin_end(32);

    content.append(&icon_widget(self.icon_path.as_ref(), self.icon_glass, dark));
    content.append(&markup_label(&self.app_name, 15, "bold", fg));
    if !self.version.trim().is_empty() {
      let line = lang::t("about.version").replace("{version}", self.version.trim());
      content.append(&markup_label(&line, 12, "normal", secondary));
    }
    outer.append(&content);

    outer.upcast()
  }
}

fn usage_error() -> ! {
  eprintln!("{}", lang::t("about.usage"));
  std::process::exit(2);
}

fn main() {
  lang::init();
  let locale = lang::locale();

  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.len() != 1 || !args[0].to_lowercase().ends_with(".app") {
    usage_error();
  }
  let target = PathBuf::from(&args[0]);
  let info = match about::load_target(&target, &locale) {
    Ok(info) => info,
    Err(err) => {
      eprintln!("{}", err);
      std::process::exit(1);
    }
  };

  let window_title = lang::t("about.title").replace("{name}", &info.display_name);

  // Every icon goes through CoreIcon: a plain bundle PNG becomes a proper
  // squircle with the Liquid Glass finish. Dark mode gets the `#1d1d1d`
  // background treatment, light mode keeps the original background.
  let dark = ColorScheme::detect_system() == ColorScheme::Dark;
  let (icon_path, icon_glass) = match &info.icon_path {
    Some(raw) => match about::beautify_icon(raw, &info.display_name, dark) {
      Some(glass) => (Some(glass), true),
      None => (Some(raw.clone()), false),
    },
    None => (about::fallback_icon(&info.display_name), true),
  };

  let delegate = AboutDelegate {
    window_title: window_title.clone(),
    app_name: info.display_name,
    version: info.version,
    icon_path,
    icon_glass,
  };

  let mut app = App::with_delegate(window_title, 340, 460, delegate);
  app.auto_color_scheme();
  app.no_window_bar();
  app.fixed_size();
  app.no_scroll();
  app.force_size(340, 460);
  app.run();
}
