//! Dialogs: About, Preferences, keyboard shortcuts, find & replace, rename layers, variables,
//! export.

use std::path::Path;

use egui::{Align2, Color32, Context, CornerRadius, Rect, RichText, Sense, Stroke, vec2};
use figcraft_color::ColorProfile;
use figcraft_doc::{ExportFormat, ExportSetting, ExportSize, NodeId};
use figcraft_engine::cmd::file::{export_jobs, exportable_layers};
use serde_json::{Value, json};

use crate::FigApp;
use crate::export_panel::{export_layers, save_one, suggested_name};
use crate::i18n::{self, tr, trf, trn};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where a report goes.
const ISSUES: &str = "https://github.com/storytold/figcraft/issues";

/// What the desktop app says when its window couldn't open with `error` (no frame was drawn: the
/// language is the one [`i18n::set`] picked from the preferences). `backend_set`: `WGPU_BACKEND`
/// named the graphics API, so the hint is to unset it rather than to try another one.
pub fn window_failed(error: &str, backend_set: bool) -> String {
    let hint = if backend_set {
        tr("WGPU_BACKEND is set and names the graphics API to use; unsetting it lets FigCraft choose.").to_string()
    } else if cfg!(windows) {
        // Direct3D 12 and OpenGL are both offered, but wgpu keeps the Direct3D 12 adapter it picked
        // even when its device then fails, so naming OpenGL alone can still help.
        format!(
            "{}

set WGPU_BACKEND=gl
figcraft.exe",
            tr("Updating the graphics driver often helps. To use OpenGL only, start FigCraft from a Command Prompt with:")
        )
    } else {
        tr("Updating the graphics driver often helps. WGPU_BACKEND (vulkan, gl, metal) names another graphics API to try.").to_string()
    };
    format!(
        "{}

{hint}

{}",
        trf("FigCraft couldn't open its window: {error}", &[("error", &error)]),
        trf("If it keeps happening, please report it at {link} with this message.", &[("link", &ISSUES)])
    )
}

/// A dialog window, centred.
pub(crate) fn window<'a>(title: &'a str, ctx: &Context) -> Dialog<'a> {
    floating(title, ctx).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
}

/// A window styled like the dialogs that the user can move.
pub(crate) fn floating<'a>(title: &'a str, ctx: &Context) -> Dialog<'a> {
    let t = Tokens::get(ctx);
    let win = egui::Window::new(title).collapsible(false).resizable(false).title_bar(false);
    Dialog { win, title, open: None, bar: true, frame: egui::Frame::window(&ctx.global_style()).fill(t.panel) }
}

/// The dialogs' space around their content, left and right.
pub(crate) const DIALOG_PAD: f32 = 20.0;
/// The height of a dialog's title bar.
const BAR_H: f32 = 44.0;

/// A dialog window: its own title bar (the title on the left, the close button on the right, a
/// line under them), then the content.
pub(crate) struct Dialog<'a> {
    win: egui::Window<'a>,
    title: &'a str,
    open: Option<&'a mut bool>,
    bar: bool,
    frame: egui::Frame,
}

impl<'a> Dialog<'a> {
    pub fn id(mut self, id: egui::Id) -> Self {
        self.win = self.win.id(id);
        self
    }

    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn order(mut self, order: egui::Order) -> Self {
        self.win = self.win.order(order);
        self
    }

    pub fn anchor(mut self, align: Align2, offset: egui::Vec2) -> Self {
        self.win = self.win.anchor(align, offset);
        self
    }

    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.win = self.win.pivot(pivot);
        self
    }

    pub fn default_pos(mut self, pos: egui::Pos2) -> Self {
        self.win = self.win.default_pos(pos);
        self
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.win = self.win.movable(movable);
        self
    }

    pub fn interactable(mut self, interactable: bool) -> Self {
        self.win = self.win.interactable(interactable);
        self
    }

    pub fn fade_in(mut self, fade_in: bool) -> Self {
        self.win = self.win.fade_in(fade_in);
        self
    }

    /// Without the title bar (and its close button).
    pub fn title_bar(mut self, bar: bool) -> Self {
        self.bar = bar;
        self
    }

    pub fn show<R>(self, ctx: &Context, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<egui::InnerResponse<Option<R>>> {
        let Dialog { win, title, open, bar, frame } = self;
        if open.as_deref() == Some(&false) {
            return None;
        }
        let pad = DIALOG_PAD as i8;
        if !bar {
            return win.frame(frame.inner_margin(egui::Margin { left: pad, right: pad, top: 12, bottom: 12 })).show(ctx, add);
        }
        let mut closed = false;
        let shown = win.frame(frame.inner_margin(0)).show(ctx, |ui| {
            let t = Tokens::get(ui.ctx());
            let spacing = ui.spacing().item_spacing;
            ui.spacing_mut().item_spacing.y = 0.0;
            let top = ui.cursor().min;
            ui.allocate_exact_size(vec2(0.0, BAR_H), Sense::hover());
            let inner = egui::Frame::NONE
                .inner_margin(egui::Margin { left: pad, right: pad, top: 16, bottom: 16 })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = spacing;
                    add(ui)
                })
                .inner;
            let bar_r = Rect::from_min_size(top, vec2(ui.min_rect().width(), BAR_H));
            let close = Rect::from_center_size(egui::pos2(bar_r.max.x - DIALOG_PAD + 2.0 - 12.0, bar_r.center().y), vec2(26.0, 26.0));
            let shown_title = crate::widgets::elide(ui, title, (close.min.x - bar_r.min.x - DIALOG_PAD - 8.0).max(40.0));
            ui.painter().text(egui::pos2(bar_r.min.x + DIALOG_PAD, bar_r.center().y), Align2::LEFT_CENTER, shown_title, semibold(13.0), t.text);
            let resp = ui.interact(close, ui.id().with("dialog_close"), Sense::click());
            crate::a11y::button(&resp, tr("Close"));
            if resp.hovered() {
                ui.painter().rect_filled(close, CornerRadius::same(6), t.hover);
            }
            icons::paint(
                ui.painter(),
                Rect::from_center_size(close.center(), vec2(12.0, 12.0)),
                Icon::Close,
                if resp.hovered() { t.icon } else { t.icon_secondary },
            );
            closed = resp.clicked();
            ui.painter().line_segment([bar_r.left_bottom(), bar_r.right_bottom()], Stroke::new(1.0, t.border));
            inner
        });
        if closed && let Some(open) = open {
            *open = false;
        }
        shown
    }
}

/// What marks a dialog's lead: an icon, or a character for what there's no icon for.
pub(crate) enum Mark {
    Icon(Icon),
    Glyph(&'static str),
}

/// A dialog's lead, `w` wide: `mark` in a square tinted `tint`, beside `title` and, under it, `sub`.
pub(crate) fn dialog_lead(ui: &mut egui::Ui, w: f32, mark: Mark, tint: Color32, title: &str, sub: &str) {
    let t = Tokens::get(ui.ctx());
    let text_w = (w - 52.0).max(80.0);
    let title_g = ui.painter().layout(title.to_string(), semibold(13.0), t.text, text_w);
    let sub_g = (!sub.is_empty()).then(|| ui.painter().layout(sub.to_string(), regular(11.5), t.text_secondary, text_w));
    let text_h = title_g.size().y + sub_g.as_ref().map_or(0.0, |g| 4.0 + g.size().y);
    let (r, _) = ui.allocate_exact_size(vec2(w, text_h.max(40.0)), Sense::hover());
    let tile = Rect::from_min_size(r.min, vec2(40.0, 40.0));
    ui.painter().rect_filled(tile, CornerRadius::same(9), tint.gamma_multiply(0.16));
    match mark {
        Mark::Icon(i) => icons::paint(ui.painter(), Rect::from_center_size(tile.center(), vec2(18.0, 18.0)), i, tint),
        Mark::Glyph(g) => {
            ui.painter().text(tile.center(), Align2::CENTER_CENTER, g, semibold(18.0), tint);
        }
    }
    let x = tile.max.x + 12.0;
    let top = if text_h < 40.0 { r.min.y + (40.0 - text_h) / 2.0 } else { r.min.y };
    let title_h = title_g.size().y;
    ui.painter().galley(egui::pos2(x, top), title_g, t.text);
    if let Some(g) = sub_g {
        ui.painter().galley(egui::pos2(x, top + title_h + 4.0), g, t.text_secondary);
    }
}

/// A dialog's buttons, `w` wide, laid out from the right (the first one `add` draws is the
/// rightmost).
pub(crate) fn dialog_buttons(ui: &mut egui::Ui, w: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(16.0);
    // The line above the buttons spans the window, across its margins.
    let t = Tokens::get(ui.ctx());
    let (line, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().line_segment(
        [egui::pos2(line.min.x - DIALOG_PAD, line.center().y), egui::pos2(line.min.x + w + DIALOG_PAD, line.center().y)],
        Stroke::new(1.0, t.border),
    );
    ui.add_space(14.0);
    ui.allocate_ui_with_layout(vec2(w, crate::widgets::FIELD_H), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        add(ui);
    });
}

/// A card holding a dialog's list.
pub(crate) fn list_card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// The About window's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AboutTab {
    #[default]
    About,
    Contributors,
    Models,
}

/// Values collected by a menu or Actions command before dispatching it to the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuInput {
    Link(String),
    Offset(f64),
    MoveToPage,
}

impl AboutTab {
    pub const ALL: [AboutTab; 3] = [AboutTab::About, AboutTab::Contributors, AboutTab::Models];

    pub fn label(self) -> &'static str {
        match self {
            AboutTab::About => tr("About"),
            AboutTab::Contributors => tr("Contributors"),
            AboutTab::Models => tr("Models"),
        }
    }
}

/// The About window's width, the same on every tab (the Contributors table needs the room).
pub const ABOUT_WIDTH: f32 = 660.0;

/// The About window's tab row, drawn like the Design / Prototype tabs of the right panel.
/// A row of tabs (the About window's, the shortcuts panel's): the current one highlighted.
pub(crate) fn tab_strip<T: Copy + PartialEq>(ui: &mut egui::Ui, t: &Tokens, tabs: &[T], label: fn(T) -> &'static str, current: &mut T) {
    let galleys: Vec<_> = tabs
        .iter()
        .map(|&tab| ui.painter().layout_no_wrap(label(tab).into(), semibold(11.5), if *current == tab { t.text } else { t.text_secondary }))
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 24.0).collect();
    let (track, _) = ui.allocate_exact_size(vec2(widths.iter().sum::<f32>() + 6.0, 32.0), Sense::hover());
    ui.painter().rect_filled(track, CornerRadius::same(8), t.field);
    let mut x = track.min.x + 3.0;
    for ((&tab, galley), w) in tabs.iter().zip(galleys).zip(widths) {
        let label = label(tab);
        let active = *current == tab;
        let r = Rect::from_min_size(egui::pos2(x, track.min.y + 3.0), vec2(w, track.height() - 6.0));
        x += w;
        let resp = ui.interact(r, ui.id().with(("tab_strip", label)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label));
        if active {
            ui.painter().rect_filled(r.translate(vec2(0.0, 1.0)), CornerRadius::same(6), Color32::from_black_alpha(24));
            ui.painter().rect_filled(r, CornerRadius::same(6), t.panel);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
        }
        ui.painter().galley(egui::pos2(r.center().x - galley.size().x / 2.0, r.center().y - galley.size().y / 2.0), galley, t.text);
        if resp.clicked() {
            *current = tab;
        }
    }
}

/// About ▸ About: the logo, version, blurb and links.
fn about_body(ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        crate::theme::paint_app_icon(ui, r);
        ui.vertical(|ui| {
            let (wordmark, _) = ui.allocate_exact_size(vec2(130.0, 23.0), Sense::hover());
            crate::theme::paint_wordmark(ui.painter(), egui::pos2(wordmark.min.x, wordmark.center().y), 18.0, t);
            ui.label(RichText::new(trf("Version {version}", &[("version", &VERSION)])).color(t.text_secondary));
        });
    });
    ui.add_space(12.0);
    ui.label(tr(
        "Interface design and prototyping. A free, open-source, clean-room design tool written from scratch in Rust, part of the ArtCraft Crafting Apps.",
    ));
    ui.add_space(8.0);
    ui.hyperlink_to("getartcraft.com/apps/figcraft", "https://getartcraft.com/apps/figcraft");
    ui.hyperlink_to(tr("Join us on Discord"), "https://discord.gg/artcraft");
    ui.add_space(8.0);
    ui.label(
        RichText::new(tr("Dual-licensed MIT OR Apache-2.0. Made by the ArtCraft team and community.")).color(t.text_secondary).font(regular(11.0)),
    );
}

/// Escape closes the topmost simple dialog (one without menus of its own), before the canvas's
/// shortcuts see it (it doesn't deselect), like a system dialog: from one of its fields too. A menu
/// open over the dialog (Preferences ▸ Interface size) takes the first Escape and closes alone.
pub fn keys(app: &mut FigApp, ctx: &Context) {
    if egui::Popup::is_any_open(ctx) {
        return;
    }
    let u = &mut app.ui;
    let open = u.color_profile.is_some()
        || u.menu_input.is_some()
        || u.find.is_some()
        || u.batch_rename.is_some()
        || u.import_report.is_some()
        || u.missing_fonts.is_some()
        || u.confirm_close
        || u.export_preview
        || u.shortcuts_panel
        || u.preferences
        || u.about;
    if !open || !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        return;
    }
    if u.menu_input.take().is_some()
        || u.color_profile.take().is_some()
        || u.find.take().is_some()
        || u.batch_rename.take().is_some()
        || u.import_report.take().is_some()
        || u.missing_fonts.take().is_some()
    {
        return;
    }
    for flag in [&mut u.confirm_close, &mut u.export_preview, &mut u.shortcuts_panel, &mut u.preferences, &mut u.about] {
        if std::mem::take(flag) {
            return;
        }
    }
}

pub fn show(app: &mut FigApp, ctx: &Context) {
    // Dropped files waiting to open: their card, then (once it has shown) the files.
    crate::pending_open::pending_opens(app, ctx);
    crate::import_report::show(app, ctx);
    crate::missing_fonts::show(app, ctx);
    let t = Tokens::get(ctx);
    if app.ui.about {
        let mut open = true;
        window(tr("About FigCraft"), ctx).open(&mut open).show(ctx, |ui| {
            ui.set_width(ABOUT_WIDTH);
            tab_strip(ui, &t, &AboutTab::ALL, AboutTab::label, &mut app.ui.about_tab);
            ui.add_space(12.0);
            match app.ui.about_tab {
                AboutTab::About => about_body(ui, &t),
                AboutTab::Contributors => crate::credits::contributors_ui(ui),
                AboutTab::Models => crate::credits::models_ui(ui),
            }
        });
        if !open {
            app.ui.about = false;
        }
    }
    crate::preferences::show(app, ctx);
    crate::shortcuts_panel::show(app, ctx);
    if let Some((mut find, mut replace)) = app.ui.find.clone() {
        let mut open = true;
        let mut action = None;
        floating(tr("Find and replace"), ctx)
            .id(egui::Id::new("Find and replace"))
            .open(&mut open)
            .anchor(Align2::LEFT_TOP, vec2(260.0, 120.0))
            .show(ctx, |ui| {
                const W: f32 = 300.0;
                ui.set_width(W);
                ui.spacing_mut().item_spacing.y = 8.0;
                let r = crate::widgets::text_input(ui, egui::Id::new("find_text"), &mut find, W, tr("Find"));
                crate::widgets::select_all_on_focus(ui, &r, &find);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let r = crate::widgets::text_input(ui, egui::Id::new("replace_text"), &mut replace, W, tr("Replace with"));
                crate::widgets::select_all_on_focus(ui, &r, &replace);
                let ok = !find.is_empty();
                dialog_buttons(ui, W, |ui| {
                    if crate::widgets::button(ui, tr("Replace all"), true, ok).clicked() {
                        action = Some(true);
                    }
                    if crate::widgets::button(ui, tr("Find"), false, ok).clicked() || enter {
                        action = Some(false);
                    }
                });
            });
        if let Some(rep) = action
            && !find.is_empty()
        {
            let r = if rep {
                app.run("edit.findReplace", json!({"find": find, "replace": replace, "layerNames": false}))
            } else {
                app.run("edit.findReplace", json!({"find": find, "layerNames": true}))
            };
            if let Some(v) = r {
                let n = v["matches"].as_array().map(|a| a.len()).unwrap_or(0);
                app.toast(trn("{n} matches", n, &[]));
            }
        }
        app.ui.find = if open { Some((find, replace)) } else { None };
    }
    batch_rename_dialog(app, ctx);
    menu_input_dialog(app, ctx);
    crate::variables_panel::show(app, ctx);
    crate::dev_compare::show(app, ctx);
    crate::branch_ui::show(app, ctx);
    if app.ui.export_preview {
        export_dialog(app, ctx);
    }
    color_profile_dialog(app, ctx);
    crate::plugins_ui::show(app, ctx);
    crate::live::join_dialog(app, ctx);
    crate::live::publish_dialog(app, ctx);
    crate::online::dialogs(app, ctx);
    crate::windows::confirm_close(app, ctx);
}

/// Menus and Actions cannot pass a value themselves. Gather it here, then run the regular
/// command so the same validation and undo behaviour apply as for programmatic callers.
fn menu_input_dialog(app: &mut FigApp, ctx: &Context) {
    let Some(mut input) = app.ui.menu_input.take() else { return };
    let t = Tokens::get(ctx);
    let title = match &input {
        MenuInput::Link(_) => i18n::command("text.link", "Create link"),
        MenuInput::Offset(_) => i18n::command("vector.offset", "Offset path"),
        MenuInput::MoveToPage => i18n::command("object.moveToPage", "Move to page"),
    };
    let mut open = true;
    let mut action: Option<(&str, serde_json::Value)> = None;
    window(&title, ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(320.0);
        match &mut input {
            MenuInput::Link(url) => {
                ui.horizontal(|ui| {
                    let field = crate::widgets::text_input(ui, egui::Id::new("menu_input_link"), url, 250.0, "https://");
                    let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let ok = !url.trim().is_empty();
                    if (crate::widgets::button(ui, tr("Done"), true, ok).clicked() || enter) && ok {
                        action = Some(("text.link", json!({"url": url.trim()})));
                    }
                });
            }
            MenuInput::Offset(offset) => {
                ui.horizontal(|ui| {
                    let id = egui::Id::new("menu_input_offset");
                    if let Some(crate::widgets::FieldEdit::Set(v) | crate::widgets::FieldEdit::Scrub(v, _)) =
                        crate::widgets::number_field(ui, id, crate::widgets::Lead::None, Some(*offset), "", 120.0, 1.0)
                    {
                        *offset = v.clamp(-100_000.0, 100_000.0);
                    }
                    if crate::widgets::button(ui, tr("Offset"), true, true).clicked() {
                        action = Some(("vector.offset", json!({"offset": *offset})));
                    }
                });
            }
            MenuInput::MoveToPage => {
                let mut choices = 0;
                if let Ok(ds) = app.session.ds() {
                    for &page in &ds.doc.pages {
                        if page == ds.page || ds.doc.is_divider(page) {
                            continue;
                        }
                        if let Some(node) = ds.doc.node(page)
                            && crate::widgets::button(ui, &node.name, false, true).clicked()
                        {
                            action = Some(("object.moveToPage", json!({"page": page})));
                        }
                        choices += 1;
                    }
                }
                if choices == 0 {
                    crate::widgets::label(ui, tr("No other pages"), t.text_secondary);
                }
            }
        }
    });
    if let Some((id, params)) = action {
        app.run(id, params);
    } else if open {
        app.ui.menu_input = Some(input);
    }
}

/// The Rename layers dialog's fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatchRenameUi {
    /// The layers, in stacking order (bottom first), as numbered.
    pub ids: Vec<NodeId>,
    /// Match: empty renames the whole name.
    pub find: String,
    pub regex: bool,
    /// Rename to.
    pub to: String,
    /// Where up numbers start and down numbers stop.
    pub start: u32,
}

/// Layers whose new names the dialog previews.
const PREVIEW_ROWS: usize = 5;

/// Open the Rename layers dialog for `ids` (several layers: Cmd/Ctrl+R, F2, the context menu).
pub(crate) fn open_batch_rename(app: &mut FigApp, ids: &[NodeId]) {
    let Ok(doc) = app.session.doc() else { return };
    let ids = figcraft_engine::cmd::edit::z_sorted(doc, ids);
    app.ui.batch_rename = Some(BatchRenameUi { ids, start: 1, ..Default::default() });
}

/// The rename the fields describe: a match replaced, or (no match) the whole name.
pub(crate) fn batch_rename_params(b: &BatchRenameUi) -> serde_json::Value {
    let mut p = json!({"ids": b.ids, "regex": b.regex, "start": b.start});
    if b.find.is_empty() {
        p["name"] = json!(b.to);
    } else {
        p["find"] = json!(b.find);
        p["replace"] = json!(b.to);
    }
    p
}

/// Rename layers: Match, Rename to with buttons for the current name and numbers, where numbers
/// start, a preview of the first new names, then one `object.batchRename`.
fn batch_rename_dialog(app: &mut FigApp, ctx: &Context) {
    let Some(mut b) = app.ui.batch_rename.clone() else { return };
    let t = Tokens::get(ctx);
    let find = (!b.find.is_empty()).then_some(b.find.as_str());
    let rename = figcraft_engine::rename::BatchRename::new(find, b.regex, &b.to, find.is_none().then_some(b.to.as_str()), Some(f64::from(b.start)));
    let n = b.ids.len();
    let ready = rename.is_ok() && !(b.find.is_empty() && b.to.is_empty());
    let preview: Vec<(String, String)> = match (&rename, app.session.doc()) {
        (Ok(r), Ok(doc)) if ready => b
            .ids
            .iter()
            .enumerate()
            .take(PREVIEW_ROWS)
            .filter_map(|(k, i)| doc.node(*i).map(|node| (node.name.clone(), r.apply(&node.name, k, n))))
            .collect(),
        _ => Vec::new(),
    };
    let mut open = true;
    let mut cancel = false;
    let mut apply = false;
    window(tr("Rename layers"), ctx).id(egui::Id::new("Rename layers")).open(&mut open).show(ctx, |ui| {
        const W: f32 = 340.0;
        ui.set_width(W);
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.label(RichText::new(trn("{n} layers", n, &[])).font(regular(11.5)).color(t.text_secondary));
        let r = crate::widgets::text_input(ui, egui::Id::new("rename_find"), &mut b.find, W, tr("Match (empty: the whole name)"));
        crate::widgets::select_all_on_focus(ui, &r, &b.find);
        if crate::widgets::checkbox(ui, b.regex, tr("Regular expression")) {
            b.regex = !b.regex;
        }
        let to = crate::widgets::text_input(ui, egui::Id::new("rename_to"), &mut b.to, W, tr("Rename to"));
        crate::widgets::select_all_on_focus(ui, &to, &b.to);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (label, code) in [(tr("Current name"), "$&"), (tr("Number ↑"), "$n"), (tr("Number ↓"), "$N")] {
                if crate::widgets::button(ui, label, false, true).clicked() {
                    b.to.push_str(code);
                    to.request_focus();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Start ascending from / stop descending at")).font(regular(11.5)).color(t.text));
            ui.allocate_ui_with_layout(vec2(ui.available_width(), crate::widgets::FIELD_H), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let shown = b.start.to_string();
                if let Some(v) =
                    crate::widgets::text_field(ui, egui::Id::new("rename_start"), &shown, 72.0, "").and_then(|s| s.trim().parse::<u32>().ok())
                {
                    b.start = v.min(1_000_000);
                }
            });
        });
        match &rename {
            Err(e) => {
                ui.label(RichText::new(e).font(regular(11.5)).color(t.danger));
            }
            Ok(_) if !preview.is_empty() => {
                egui::Frame::new()
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.card_border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for (old, new) in &preview {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(old).font(regular(11.5)).color(t.text_secondary));
                                ui.label(RichText::new("→").font(regular(11.5)).color(t.text_tertiary));
                                ui.label(RichText::new(new).font(semibold(11.5)).color(t.text));
                            });
                        }
                        if ready && n > PREVIEW_ROWS {
                            ui.label(RichText::new(trn("and {n} more", n - PREVIEW_ROWS, &[])).font(regular(11.0)).color(t.text_secondary));
                        }
                    });
            }
            Ok(_) => {}
        }
        dialog_buttons(ui, W, |ui| {
            let enter = to.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            apply = (crate::widgets::button(ui, tr("Rename"), true, ready).clicked() || enter) && ready;
            cancel = crate::widgets::button(ui, tr("Cancel"), false, true).clicked();
        });
    });
    if apply {
        app.run("object.batchRename", batch_rename_params(&b));
    }
    app.ui.batch_rename = (open && !cancel && !apply).then_some(b);
}

/// File ▸ Color profile: pick sRGB or Display P3, then keep the colours' appearance (convert the
/// values) or keep the values (reinterpret them in the new space).
fn color_profile_dialog(app: &mut FigApp, ctx: &Context) {
    let Some(mut chosen) = app.ui.color_profile else { return };
    let Ok(current) = app.session.doc().map(|d| d.color_profile) else {
        app.ui.color_profile = None;
        return;
    };
    let t = Tokens::get(ctx);
    let mut open = true;
    let mut apply = None;
    window(tr("Color profile"), ctx).open(&mut open).show(ctx, |ui| {
        const W: f32 = 340.0;
        ui.set_width(W);
        ui.label(RichText::new(tr("Color values in this file are in this color space.")).color(t.text_secondary).font(regular(11.0)));
        ui.add_space(8.0);
        for p in ColorProfile::ALL {
            let label = if p == current { trf("{profile} (current)", &[("profile", &p.label())]) } else { p.label().to_string() };
            if crate::widgets::radio(ui, chosen == p, &label) {
                chosen = p;
            }
        }
        ui.add_space(8.0);
        if chosen == current {
            ui.label(RichText::new(tr("Choose another profile to change it.")).color(t.text_secondary).font(regular(11.0)));
            return;
        }
        ui.label(
            RichText::new(trf(
                "Keep appearance converts every color value to {profile} so colors look the same. Keep values leaves the numbers as they are, so colors shift.",
                &[("profile", &chosen.label())],
            ))
            .font(regular(11.0)),
        );
        if !chosen.is_srgb() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr("The canvas previews colors in sRGB; exported PNG, JPG and WebP files are tagged Display P3."))
                    .color(t.text_secondary)
                    .font(regular(11.0)),
            );
        }
        dialog_buttons(ui, W, |ui| {
            if crate::widgets::button(ui, tr("Keep appearance"), true, true).clicked() {
                apply = Some(true);
            }
            if crate::widgets::button(ui, tr("Keep values"), false, true).clicked() {
                apply = Some(false);
            }
        });
    });
    app.ui.color_profile = open.then_some(chosen);
    if let Some(convert) = apply {
        app.run("file.colorProfile", json!({"profile": chosen.id(), "convert": convert}));
        app.ui.color_profile = None;
    }
}

/// A `.fig` Save as result is an export; only native saves report that the document was saved.
pub(crate) fn file_saved(app: &mut FigApp, result: &Value) {
    if result["format"] == "fig" {
        fig_exported(app, result);
    } else {
        app.toast(trf("Saved {name}", &[("name", &result_file_name(result))]));
    }
}

fn result_file_name(result: &Value) -> String {
    result["path"].as_str().and_then(|p| Path::new(p).file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The `.fig` result stays an export: native save state is unchanged, and omissions are reported.
pub(crate) fn fig_exported(app: &mut FigApp, result: &Value) {
    let file = result_file_name(result);
    let left_out = result["warnings"].as_array().map_or(0, Vec::len);
    app.toast(match left_out {
        0 => trf("Exported {name}", &[("name", &file)]),
        n => trn("Exported {name}; {n} kinds of content were left out", n, &[("name", &file)]),
    });
}

/// Save the whole file as `.fig`; the toast says how much was left out.
fn save_fig(app: &mut FigApp, name: &str) {
    let filename = format!("{name}.fig");
    app.pick_save(tr("Export"), &filename, &["fig"], |app, p| {
        if let Some(r) = app.run("file.export", json!({"format": "fig", "path": p.to_string_lossy()})) {
            fig_exported(app, &r);
        }
    });
}

/// What the Export dialog was asked to do.
enum ExportChoice {
    /// Every layer with export settings, with those settings.
    Layers,
    /// The layers together as one file.
    Once(ExportSetting),
    /// The whole file as `.fig` (experimental).
    Fig,
}

/// The Export dialog: the layers' own export settings when they have some, and one-off presets for
/// the selection (or the page) as one file.
fn export_dialog(app: &mut FigApp, ctx: &Context) {
    let t = Tokens::get(ctx);
    let mut open = true;
    let sel = app.session.selection();
    let Ok(ds) = app.session.ds() else {
        app.ui.export_preview = false;
        return;
    };
    let (doc, page) = (ds.doc.clone(), ds.page);
    let ids = if sel.is_empty() { doc.node(page).map(|p| p.children.clone()).unwrap_or_default() } else { sel.clone() };
    // With nothing selected, every layer on the page that has export settings.
    let with_settings = if sel.is_empty() { exportable_layers(&doc, page) } else { sel };
    let jobs = export_jobs(&doc, &with_settings);
    let print = app.ui.pdf_print;
    let preset = |size: f64, format| ExportSetting { size: ExportSize::Scale { value: size }, format, print, ..Default::default() };
    let presets = [
        preset(1.0, ExportFormat::Png),
        preset(2.0, ExportFormat::Png),
        preset(3.0, ExportFormat::Png),
        preset(2.0, ExportFormat::Jpg),
        preset(2.0, ExportFormat::Webp),
        preset(1.0, ExportFormat::Svg),
        preset(1.0, ExportFormat::Pdf),
    ];
    let mut choice = None;
    const W: f32 = 340.0;
    window(tr("Export"), ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(W);
        let caption = |ui: &mut egui::Ui, s: &str| {
            ui.label(RichText::new(s).font(regular(11.0)).color(t.text_secondary));
        };
        let title = |ui: &mut egui::Ui, s: &str| {
            let (r, _) = ui.allocate_exact_size(vec2(W, 24.0), Sense::hover());
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, s.to_uppercase(), semibold(10.0), t.text_secondary);
        };
        let divider = |ui: &mut egui::Ui| {
            ui.add_space(12.0);
            let (r, _) = ui.allocate_exact_size(vec2(W, 1.0), Sense::hover());
            ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, t.border));
            ui.add_space(10.0);
        };
        // A choice on a card: `label` and, under it, `sub`.
        let tile = |ui: &mut egui::Ui, w: f32, label: &str, sub: &str| {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 48.0), Sense::click());
            crate::a11y::button(&resp, label);
            let border = if resp.hovered() { t.accent } else { t.card_border };
            ui.painter().rect(
                r,
                CornerRadius::same(8),
                if resp.hovered() { t.hover } else { t.card },
                Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(egui::pos2(r.min.x + 12.0, r.center().y - 7.0), Align2::LEFT_CENTER, label, semibold(12.0), t.text);
            ui.painter().text(egui::pos2(r.min.x + 12.0, r.center().y + 8.0), Align2::LEFT_CENTER, sub, regular(10.5), t.text_secondary);
            resp.clicked()
        };
        if !jobs.is_empty() {
            title(ui, tr("With the layers' export settings"));
            const SHOWN: usize = 6;
            for j in jobs.iter().take(SHOWN) {
                ui.label(RichText::new(&j.file).font(regular(12.0)).color(t.text));
            }
            if jobs.len() > SHOWN {
                caption(ui, &trn("and {n} more", jobs.len() - SHOWN, &[]));
            }
            ui.add_space(6.0);
            let layers = jobs.iter().map(|j| j.id).collect::<std::collections::BTreeSet<_>>().len();
            let label = trn("Export {n} layers", layers, &[]);
            if crate::widgets::button(ui, &label, true, true).clicked() {
                choice = Some(ExportChoice::Layers);
            }
            divider(ui);
        }
        title(ui, &trn("{n} layers as one file", ids.len(), &[]));
        ui.add_space(4.0);
        let cell = (W - 8.0) / 2.0;
        for pair in presets.chunks(2) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for p in pair {
                    let sub = if p.format.is_raster() { format!("@{}", p.size.label()) } else { tr("Vector").to_string() };
                    if tile(ui, cell, p.format.label(), &sub) {
                        choice = Some(ExportChoice::Once(p.clone()));
                    }
                }
            });
            ui.add_space(8.0);
        }
        // The PDF preset's print options.
        ui.add_space(4.0);
        title(ui, tr("PDF print options"));
        if let Some(next) = crate::export_panel::print_options(ui, egui::Id::new("export_dialog_print"), &print) {
            app.ui.pdf_print = next;
        }
        divider(ui);
        title(ui, tr("The whole file"));
        ui.add_space(4.0);
        if tile(ui, W, tr(".fig (experimental)"), tr("Other apps may not open every .fig file exported here.")) {
            choice = Some(ExportChoice::Fig);
        }
    });
    let close = choice.is_some() || !open;
    match choice {
        Some(ExportChoice::Layers) => export_layers(app, &with_settings, false),
        Some(ExportChoice::Once(s)) if !ids.is_empty() => save_one(app, &ids, &s, &suggested_name(&doc, &ids, &s, "Export")),
        Some(ExportChoice::Once(_)) => app.toast(tr("Nothing to export")),
        Some(ExportChoice::Fig) => save_fig(app, &doc.name),
        None => {}
    }
    if close {
        app.ui.export_preview = false;
    }
}
