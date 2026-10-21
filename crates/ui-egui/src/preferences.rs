//! The Preferences window: settings grouped on cards, a label on the left of each row and its
//! control on the right.

use egui::{Align, Align2, Color32, CornerRadius, Layout, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::FigApp;
use crate::i18n::{self, tr, tr_noop, trf};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, FIELD_H};

const WIDTH: f32 = 460.0;
const ROW_H: f32 = 44.0;
const INSET: f32 = 14.0;
/// The width of the controls on the right of a row (fields, the theme switch, the language).
const CONTROL_W: f32 = 200.0;

pub fn show(app: &mut FigApp, ctx: &egui::Context) {
    if !app.ui.preferences {
        return;
    }
    let mut open = true;
    crate::dialogs::window(tr("Preferences"), ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing.y = 0.0;
        // A short window scrolls the settings under the title bar.
        let max = (ctx.content_rect().height() - 120.0).max(200.0);
        egui::ScrollArea::vertical().max_height(max).min_scrolled_height(max).auto_shrink([false, true]).show(ui, |ui| body(app, ui));
    });
    if !open {
        app.ui.preferences = false;
    }
}

fn body(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let prefs = app.session.prefs.clone();
    let mut toggles: Vec<&str> = Vec::new();

    section(ui, &t, tr("General"), |ui, rows| {
        rows.switch(ui, tr("Keep tool selected after use"), prefs.keep_tool, "pref.keepTool", &mut toggles);
        rows.switch(ui, tr("Rename duplicated layers"), prefs.rename_duplicates, "pref.renameDuplicates", &mut toggles);
        // The browser has no files to save to.
        if !cfg!(target_arch = "wasm32") {
            rows.switch(ui, tr("Save files automatically"), prefs.auto_save, "pref.autoSave", &mut toggles);
        }
        rows.row(ui, tr("Nudge amount"), |ui| {
            let big = nudge_field(ui, "pref_nudge_big", prefs.nudge_big, 1.0..=1000.0);
            ui.add_space(6.0);
            let small = nudge_field(ui, "pref_nudge_small", prefs.nudge_small, 0.1..=100.0);
            if small.is_some() || big.is_some() {
                app.run("pref.nudge", json!({"small": small.unwrap_or(prefs.nudge_small), "big": big.unwrap_or(prefs.nudge_big)}));
            }
        });
    });

    section(ui, &t, tr("Canvas"), |ui, rows| {
        rows.switch(ui, tr("Highlight layers on hover"), prefs.highlight_on_hover, "pref.highlightHover", &mut toggles);
        rows.switch(ui, tr("Show dimensions on objects"), prefs.show_dimensions, "pref.showDimensions", &mut toggles);
        rows.switch(ui, tr("Use number keys for opacity"), prefs.number_keys_opacity, "pref.numberKeysOpacity", &mut toggles);
    });

    section(ui, &t, tr("Appearance"), |ui, rows| {
        rows.row(ui, tr("Theme"), |ui| {
            const THEMES: [(&str, &str); 3] =
                [(tr_noop("Light"), "view.themeLight"), (tr_noop("Dark"), "view.themeDark"), (tr_noop("System"), "view.themeSystem")];
            let labels: Vec<&str> = THEMES.iter().map(|(l, _)| tr(l)).collect();
            let active = THEMES.iter().position(|(l, _)| prefs.theme.eq_ignore_ascii_case(l)).unwrap_or(2);
            let picked = ui.allocate_ui(vec2(CONTROL_W, FIELD_H), |ui| widgets::segmented_text(ui, &labels, active)).inner;
            if let Some((_, cmd)) = picked.and_then(|i| THEMES.get(i)) {
                app.run(cmd, json!({}));
            }
        });
        rows.row(ui, tr("Colors"), |ui| {
            const PALETTES: [(&str, &str, &str); 2] =
                [(tr_noop("Modern"), "modern", "view.paletteModern"), (tr_noop("Classic"), "classic", "view.paletteClassic")];
            let labels: Vec<&str> = PALETTES.iter().map(|(l, ..)| tr(l)).collect();
            let active = PALETTES.iter().position(|(_, id, _)| prefs.palette == *id).unwrap_or(0);
            let picked = ui.allocate_ui(vec2(CONTROL_W, FIELD_H), |ui| widgets::segmented_text(ui, &labels, active)).inner;
            if let Some((.., cmd)) = picked.and_then(|i| PALETTES.get(i)) {
                app.run(cmd, json!({}));
            }
        });
        rows.row(ui, tr("Language"), |ui| language(app, ui));
        if let Some(lang) = i18n::current().filter(|l| l.draft) {
            let note = trf("{language}: draft translation, waiting for a native speaker's review.", &[("language", &lang.name)]);
            rows.note(ui, &note);
        }
        rows.switch(ui, tr("Reduce motion"), prefs.reduce_motion, "pref.reduceMotion", &mut toggles);
        if app.services.reduce_motion() {
            rows.note(ui, tr("Your system asks for reduced motion: animations stay off."));
        }
    });

    section(ui, &t, tr("Sharing"), |ui, rows| {
        rows.row(ui, tr("Your name in comments"), |ui| {
            if let Some(name) = widgets::text_field(ui, egui::Id::new("pref_user_name"), &prefs.user_name, CONTROL_W, tr("You")) {
                app.run("pref.userName", json!({"name": name}));
            }
        });
        // Where Share ▸ Online keeps files and join codes are asked; no key here (preferences are
        // saved as text).
        rows.row(ui, tr("Server for online sharing"), |ui| {
            let usual = prefs.publish_server == figcraft_engine::live::DEFAULT_SERVER;
            if widgets::button(ui, tr("Reset"), false, !usual).on_hover_text(tr("Back to the FigCraft server")).clicked() {
                app.run("pref.publishServer", json!({}));
            }
            ui.add_space(6.0);
            let w = ui.available_width().min(CONTROL_W + 40.0);
            let name = tr("Server for online sharing");
            if let Some(url) = widgets::text_field_named(ui, egui::Id::new("pref_publish_server"), &prefs.publish_server, w, name) {
                app.run("pref.publishServer", json!({"url": url}));
            }
        });
        if prefs.publish_server.is_empty() {
            rows.note(ui, tr("Online sharing is off: files are shared on the local network only."));
        }
        // Where the app registers itself as the opener of invite links (Windows).
        if let Some(on) = app.session.invite_links_on() {
            let mut flip = Vec::new();
            rows.switch(ui, tr("Open invite links in FigCraft"), on, "collab.registerLinks", &mut flip);
            if !flip.is_empty() {
                app.run("collab.registerLinks", json!({"on": !on}));
            }
            rows.note(ui, tr("figcraft:// links from a browser, chat or email open this app and join the session."));
        }
    });

    for c in toggles {
        app.run(c, json!({}));
    }
}

/// The rows of a section's card: each after the first is drawn under a divider.
struct Rows {
    first: bool,
}

impl Rows {
    fn divider(&mut self, ui: &mut Ui) {
        if !self.first {
            let t = Tokens::get(ui.ctx());
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0, t.card_border));
        }
        self.first = false;
    }

    /// A row: `label` on the left, what `add` draws laid out from the right.
    fn row(&mut self, ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
        self.divider(ui);
        let t = Tokens::get(ui.ctx());
        let rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), ROW_H));
        ui.painter().text(pos2(rect.min.x, rect.center().y), Align2::LEFT_CENTER, label, regular(12.0), t.text);
        let controls = Rect::from_min_max(pos2(rect.min.x + rect.width() * 0.4, rect.min.y), rect.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(controls).layout(Layout::right_to_left(Align::Center)), add);
        // After the controls: the next row starts under the whole row, not under its controls.
        ui.allocate_rect(rect, Sense::hover());
    }

    /// A row that turns preference `cmd` on and off with a switch (the whole row is the target);
    /// a click adds `cmd` to `toggles`.
    fn switch(&mut self, ui: &mut Ui, label: &str, on: bool, cmd: &'static str, toggles: &mut Vec<&'static str>) {
        self.divider(ui);
        let t = Tokens::get(ui.ctx());
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
        crate::a11y::checkbox(&resp, label, on);
        ui.painter().text(pos2(rect.min.x, rect.center().y), Align2::LEFT_CENTER, label, regular(12.0), t.text);
        let track = Rect::from_min_size(pos2(rect.max.x - 34.0, rect.center().y - 10.0), vec2(34.0, 20.0));
        let k = ui.ctx().animate_bool(resp.id.with("switch"), on);
        let off_fill = if resp.hovered() { t.icon_secondary.gamma_multiply(0.7) } else { t.border_strong };
        ui.painter().rect_filled(track, CornerRadius::same(10), lerp_color(off_fill, t.accent, k));
        let x = egui::lerp(track.min.x + 10.0..=track.max.x - 10.0, k);
        ui.painter().circle_filled(pos2(x, track.center().y + 0.5), 7.5, Color32::from_black_alpha(40));
        ui.painter().circle_filled(pos2(x, track.center().y), 7.0, Color32::WHITE);
        if resp.clicked() {
            toggles.push(cmd);
        }
    }

    /// Small print under the row above it.
    fn note(&mut self, ui: &mut Ui, text: &str) {
        let t = Tokens::get(ui.ctx());
        ui.add_space(-6.0);
        ui.add(egui::Label::new(egui::RichText::new(text).font(regular(11.0)).color(t.text_secondary)).wrap());
        ui.add_space(10.0);
    }
}

/// A titled card holding the rows `add` lays out.
fn section(ui: &mut Ui, t: &Tokens, title: &str, add: impl FnOnce(&mut Ui, &mut Rows)) {
    ui.add_space(if ui.min_rect().height() > 1.0 { 18.0 } else { 4.0 });
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    ui.painter().text(pos2(r.min.x + 2.0, r.center().y), Align2::LEFT_CENTER, title.to_uppercase(), semibold(10.5), t.text_secondary);
    ui.add_space(6.0);
    egui::Frame::new()
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(INSET as i8, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui, &mut Rows { first: true });
        });
}

/// A nudge distance field; returns the value typed, clamped to `range`.
fn nudge_field(ui: &mut Ui, id: &str, value: f64, range: std::ops::RangeInclusive<f64>) -> Option<f64> {
    let shown = format!("{}", (value * 100.0).round() / 100.0);
    let typed = widgets::text_field(ui, egui::Id::new(id), &shown, 64.0, "")?;
    let v: f64 = typed.trim().parse().ok().filter(|v: &f64| v.is_finite())?;
    Some(v.clamp(*range.start(), *range.end()))
}

/// The language dropdown: each language by its own name; System follows the OS (the browser on
/// the web).
fn language(app: &mut FigApp, ui: &mut Ui) {
    let langs =
        || [("system", tr("System"))].into_iter().chain(i18n::languages().iter().filter(|l| crate::cjk::can_show(l.code)).map(|l| (l.code, l.name)));
    let on = |code: &str| app.session.prefs.language.eq_ignore_ascii_case(code);
    let selected = langs().find(|(code, _)| on(code)).map_or(tr("System"), |(_, name)| name);
    let resp = widgets::dropdown(ui, selected, CONTROL_W, None);
    let mut pick = None;
    crate::right::menu_popup(&resp).show(|ui| {
        for (code, name) in langs() {
            if crate::right::menu_check(ui, on(code), name, "") {
                pick = Some(code);
                ui.close();
            }
        }
    });
    if let Some(code) = pick {
        app.run("pref.language", json!({"language": code}));
        ui.ctx().request_repaint();
    }
}

fn lerp_color(a: Color32, b: Color32, k: f32) -> Color32 {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}
