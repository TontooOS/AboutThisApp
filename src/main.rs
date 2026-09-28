//! AboutThisApp: macOS-style About window for TontooOS `.app` bundles.
//!
//! Usage: `about-this-app "<path-to-app>.app"` (e.g.
//! `about-this-app "/Users/paul/Applications/Finder.app"`).
//!
//! Reads `Info.tontoo` (localized `name`, `version`) and the icon
//! (`App/icon.png`, then `Resources/icon.png`) from the target bundle and
//! shows a fixed 340x460 card: title bar with a disabled maximize light,
//! centered 120px icon (every icon is finished through CoreIcon into an
//! Apple-style squircle with the Liquid Glass depth effect), bold app name
//! and a `Version {version}` line. All text uses SF Pro (system font) with
//! `en_us` and `de_de` strings from `lang/` via Accessibility.
//!
//! Rendering is TontooUI on Vello/WGPU: `Titlebar::without_maximize`,
//! `FileImage` for the CoreIcon-finished icon file, `FormattedText` with a
//! bold span for the name and `BasicText` for the version. The theme follows
//! the settings daemon live through `ThemeWatcher` (Dark `#1B2022` / Light
//! `#FFFFFF`).

mod about;
mod lang;

sdk::preinclude!();

use std::path::PathBuf;

use TontooUI::elements::{
  Align, BasicText, FileImage, FormattedText, ImageFit, Span, TextAlignment,
  TextForeground, TextStyle, Titlebar, TrafficAction, View, VStack,
};
use TontooUI::renderer::window::{App, Viewport, WindowCommand, run};
use TontooUI::renderer::{FontSystem, ImageLoader};
use TontooUI::theme::{ThemeMode, ThemeWatcher};
use vello::Scene;
use vello::peniko::Color;

const WINDOW_WIDTH: u32 = 340;
const WINDOW_HEIGHT: u32 = 460;
const ICON_PX: f32 = 120.0;
const ICON_RADIUS: f32 = 28.0;
const TEXT_WIDTH: f32 = 276.0;

struct AboutApp {
  bar: Titlebar,
  stack: VStack,
  watcher: ThemeWatcher,
  focused: bool,
  bg: Color,
  command: Option<WindowCommand>,
}

impl AboutApp {
  fn new(
    window_title: String,
    app_name: String,
    version_line: Option<String>,
    icon: PathBuf,
  ) -> Self {
    let mut stack = VStack::new()
      .spacing(8.0)
      .align(Align::Center)
      .child(
        FileImage::new(icon, ICON_PX, ICON_PX)
          .radius(ICON_RADIUS)
          .fit(ImageFit::Cover),
      )
      .child(
        FormattedText::spans(vec![Span::new(app_name).bold()])
          .style(TextStyle::Subheadline)
          .alignment(TextAlignment::Center)
          .width(TEXT_WIDTH),
      );
    if let Some(line) = version_line {
      stack = stack.child(
        BasicText::new(line)
          .style(TextStyle::Caption)
          .foreground(TextForeground::Secondary)
          .alignment(TextAlignment::Center)
          .width(TEXT_WIDTH),
      );
    }
    Self {
      bar: Titlebar::new(window_title).without_maximize(),
      stack,
      watcher: ThemeWatcher::new(),
      focused: true,
      bg: TontooUI::renderer::window::BACKGROUND,
      command: None,
    }
  }
}

impl App for AboutApp {
  fn draw(
    &mut self,
    scene: &mut Scene,
    fonts: &mut FontSystem,
    images: &mut ImageLoader<'_>,
    viewport: Viewport,
    time_secs: f64,
  ) {
    self.watcher.poll(time_secs);
    self.watcher.set_focused(self.focused, time_secs);
    let palette = self.watcher.palette(time_secs);
    self.bg = palette.bg;
    let mode = self.watcher.theme().mode;
    let dark = mode == ThemeMode::Dark;
    let focused = self.focused;
    if let Some(icon) = self.stack.child_mut::<FileImage>(0) {
      icon.set_theme(dark);
      icon.set_focused(focused);
    }
    if let Some(name) = self.stack.child_mut::<FormattedText>(1) {
      name.set_theme(mode);
      name.set_focused(focused);
    }
    for index in 2..self.stack.len() {
      if let Some(line) = self.stack.child_mut::<BasicText>(index) {
        line.set_theme(mode);
        line.set_focused(focused);
      }
    }

    self.bar.set_palette(
      palette.titlebar_bg,
      palette.titlebar_text,
      palette.divider,
    );
    self.bar.set_rect(viewport.x, viewport.y, viewport.width);
    self.bar.draw(scene, fonts);

    // Title bar height is 31 px; the card content stays centered in the
    // remaining body, like the old centered GTK box.
    let top = viewport.y + 31.0;
    let (stack_w, stack_h) = self.stack.measure(fonts);
    let x = viewport.x + ((viewport.width - stack_w) / 2.0).max(0.0);
    let body_h = (viewport.height - 31.0).max(0.0);
    let y = top + ((body_h - stack_h) / 2.0).max(0.0);
    self.stack.place(fonts, x, y, stack_w, stack_h);
    self.stack.draw(scene, fonts, images);
  }

  fn background(&self) -> Color {
    self.bg
  }

  fn drag_region(&self) -> Option<(f32, f32, f32, f32)> {
    Some(self.bar.drag_rect())
  }

  fn poll_window_command(&mut self) -> Option<WindowCommand> {
    self.command.take()
  }

  fn mouse_down(&mut self, x: f64, y: f64) {
    match self.bar.press(x as f32, y as f32) {
      Some(TrafficAction::Close) => self.command = Some(WindowCommand::Close),
      Some(TrafficAction::Minimize) => self.command = Some(WindowCommand::Minimize),
      // Unreachable: the maximize light is disabled via `without_maximize`.
      Some(TrafficAction::Maximize) => self.command = Some(WindowCommand::ToggleMaximize),
      None => {}
    }
  }

  fn mouse_move(&mut self, x: f64, y: f64) {
    self.bar.set_hover(x as f32, y as f32);
  }

  fn set_focused(&mut self, focused: bool) {
    self.focused = focused;
    self.bar.set_focused(focused);
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
      eprintln!("{err}");
      std::process::exit(1);
    }
  };

  let window_title = lang::t("about.title").replace("{name}", &info.display_name);
  let version_line = if info.version.trim().is_empty() {
    None
  } else {
    Some(
      lang::t("about.version").replace("{version}", info.version.trim()),
    )
  };

  // Every icon goes through CoreIcon: `.tico` entries render into a
  // finished PNG, plain files become a proper squircle with the Liquid
  // Glass finish. The theme is probed once from the settings daemon; dark
  // mode gets the dark background treatment, light mode keeps the
  // original background.
  let mut probe = ThemeWatcher::new();
  probe.poll(0.0);
  let dark = probe.theme().mode == ThemeMode::Dark;
  let icon = match &info.icon_path {
    Some(raw)
      if raw
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tico")) =>
    {
      about::tico_to_png(raw, &info.display_name)
        .unwrap_or_else(|| raw.clone())
    }
    Some(raw) => about::beautify_icon(raw, &info.display_name, dark)
      .unwrap_or_else(|| raw.clone()),
    None => about::fallback_icon(&info.display_name)
      .unwrap_or_else(|| PathBuf::from("__about_this_app_missing_icon__")),
  };

  let app = AboutApp::new(window_title.clone(), info.display_name, version_line, icon);
  if let Err(err) = run(&window_title, WINDOW_WIDTH, WINDOW_HEIGHT, app) {
    eprintln!("error: {err}");
    std::process::exit(1);
  }
}
