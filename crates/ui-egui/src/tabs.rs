//! The top bar: islands for the file (main menu, name, open files), the command bar and sharing.
//! In a frameless window it is also the title bar: its empty space drags the window
//! (double-click maximises), and it carries the window buttons; the window's edges resize it.

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::a11y::Tip;
use crate::i18n::{tr, trf};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};
use crate::{FigApp, TitleBar};

/// The top bar's height: its islands and the margin above them.
pub const HEIGHT: f32 = 56.0;
const ISLAND_H: f32 = 44.0;

/// The top bar: the file island (main menu, file name, open files), the command bar and the
/// share island (theme, people, Present, Share), floating over the page like the panels below
/// them. In a frameless window it is also the title bar: its empty space drags the window
/// (double-click maximises), and it carries the window buttons.
pub fn tab_bar(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let editing = app.has_files() && !app.ui.home && app.ui.present.is_none();
    let bg = if editing { crate::canvas::page_background(app, ui.ctx()) } else { t.panel };
    egui::Panel::top("tab_bar").exact_size(HEIGHT).frame(egui::Frame::NONE.fill(bg)).show(ui, |ui| {
        let rect = ui.max_rect();
        // Empty space moves the window (first, so the islands on top take their clicks).
        if app.ui.title_bar != TitleBar::Native {
            let drag = ui.interact(rect, ui.id().with("titlebar_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
        }
        let buttons_w = if app.ui.title_bar == TitleBar::Custom { window_buttons(app, ui, rect) } else { 0.0 };
        let gap = f32::from(crate::ISLAND_GAP);
        let inset = if app.ui.title_bar == TitleBar::Inset { 76.0 } else { gap };
        let y = rect.min.y + gap;
        // The islands line up with the panels under them.
        let (left_w, right_w) = app.panel_widths(rect.width());
        let lw = (if left_w > 0.0 { left_w } else { crate::PANEL_DEFAULT } - (inset - gap)).max(180.0);
        let left = Rect::from_min_size(pos2(rect.min.x + inset, y), vec2(lw, ISLAND_H));
        file_island(app, ui, left, &t, editing);
        let rw = if editing { right_w } else { ISLAND_H };
        let right = Rect::from_min_size(pos2(rect.max.x - buttons_w - gap - rw, y), vec2(rw, ISLAND_H));
        share_island(app, ui, right, &t, editing);
        app.ui.command_rect = None;
        if editing {
            command_bar(app, ui, Rect::from_min_max(pos2(left.max.x + gap, y), pos2(right.min.x - gap, y + ISLAND_H)), rect.center().x);
        }
    });
}

/// An island's panel: filled, outlined and rounded like the floating panels.
fn island(ui: &Ui, r: Rect, t: &Tokens) {
    let radius = CornerRadius::same(crate::ISLAND_R);
    ui.interact(r, ui.id().with(("island", r.min.x as i32)), Sense::click());
    ui.painter().rect_filled(r, radius, t.panel);
    ui.painter().rect_stroke(r, radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
}

/// The file island: the main menu (the logo), the file's name (a click renames it) and the menu
/// of open files, and the buttons for the variables and the left panel.
fn file_island(app: &mut FigApp, ui: &mut Ui, r: Rect, t: &Tokens, editing: bool) {
    island(ui, r, t);
    let cy = r.center().y;
    let lr = Rect::from_min_size(pos2(r.min.x + 8.0, cy - 14.0), vec2(28.0, 28.0));
    let lresp = ui.interact(lr, ui.id().with("rail_menu"), Sense::click()).tip(tr("Main menu"));
    ui.painter().rect_filled(lr, CornerRadius::same(9), if lresp.hovered() || app.ui.main_menu { t.accent_hover } else { crate::theme::ACCENT });
    icons::paint(ui.painter(), Rect::from_center_size(lr.center(), vec2(16.0, 16.0)), Icon::Logo, Color32::WHITE);
    if lresp.clicked() {
        app.ui.main_menu = !app.ui.main_menu;
    }
    if app.ui.main_menu {
        crate::a11y::popup(&mut app.ui.nav, lresp.id, crate::menus::main_menu_id());
    }
    // Right end: the left panel, the variables (while editing) and the open files.
    let mut x = r.max.x - 8.0;
    let button = |ui: &mut Ui, x: &mut f32, id: &str, icon: Icon, tip: &str| {
        let br = Rect::from_min_size(pos2(*x - 26.0, cy - 13.0), vec2(26.0, 26.0));
        *x -= 28.0;
        crate::widgets::icon_button_at(ui, ui.id().with(id), br, icon, tip)
    };
    let files = {
        let br = Rect::from_min_size(pos2(x - 22.0, cy - 13.0), vec2(22.0, 26.0));
        x -= 24.0;
        let resp = ui.interact(br, ui.id().with("open_files"), Sense::click()).tip(tr("Open files"));
        crate::a11y::menu_button(&resp, tr("Open files"), crate::a11y::menu_open(&resp));
        if resp.hovered() {
            ui.painter().rect_filled(br, CornerRadius::same(6), t.hover);
        }
        icons::paint(ui.painter(), Rect::from_center_size(br.center(), vec2(10.0, 10.0)), Icon::Chevron, t.icon_secondary);
        resp
    };
    if editing {
        if button(ui, &mut x, "variables", Icon::Variable, tr("Variables")) {
            app.ui.variables.get_or_insert_with(Default::default);
        }
        if button(ui, &mut x, "hideui", Icon::Panels, tr("Collapse panel")) {
            app.ui.left_collapsed = !app.ui.left_collapsed;
        }
    }
    let text_x = lr.max.x + 10.0;
    let name_r = Rect::from_min_max(pos2(text_x - 2.0, cy - 17.0), pos2(x - 2.0, cy + 1.0));
    if editing {
        let name = app.session.doc().map(|d| d.name.clone()).unwrap_or_default();
        let start = std::mem::take(&mut app.ui.rename_file);
        if let (Some(new), _) = crate::widgets::rename_field(ui, Id::new("filename"), &name, name_r, semibold(13.0), start) {
            app.run("file.rename", json!({"name": new}));
        }
        ui.painter().text(pos2(text_x, cy + 9.0), Align2::LEFT_CENTER, tr("Drafts"), regular(10.5), t.text_secondary);
    } else {
        let (title, sub) = match app.ui.present.is_some() {
            true => (app.session.doc().map(|d| d.name.clone()).unwrap_or_default(), tr("Present")),
            false => ("FigCraft".to_string(), tr("Home")),
        };
        let shown = crate::widgets::elide_font(ui, &title, (x - text_x).max(0.0), semibold(13.0));
        ui.painter().text(pos2(text_x, cy - 8.0), Align2::LEFT_CENTER, shown, semibold(13.0), t.text);
        ui.painter().text(pos2(text_x, cy + 9.0), Align2::LEFT_CENTER, sub, regular(10.5), t.text_secondary);
    }
    files_menu(app, ui, &files, editing);
}

/// The open files, Home, the presentation and the file commands: what the tab strip offered.
fn files_menu(app: &mut FigApp, ui: &Ui, anchor: &egui::Response, editing: bool) {
    let several = !ui.ctx().embed_viewports();
    let docs: Vec<(usize, String, bool, bool)> = app
        .session
        .docs
        .iter()
        .enumerate()
        .filter(|(_, d)| app.shows(d.uid))
        .map(|(i, d)| {
            // In a live session the row says whether others get in.
            let name = match live_mark(app, d.uid) {
                Some(true) => trf("{name}, in a live session", &[("name", &d.doc.name)]),
                Some(false) => trf("{name}, online, open only to you", &[("name", &d.doc.name)]),
                None => d.doc.name.clone(),
            };
            (i, name, d.is_dirty(), i == app.session.active && editing)
        })
        .collect();
    let presenting = app.ui.present.is_some() || app.ui.paused_present.is_some();
    let mut switch = None;
    let mut close = None;
    let mut detach = None;
    let ctx = ui.ctx().clone();
    crate::right::menu_popup(anchor).width(240.0).show(|ui| {
        if crate::right::menu_item_beside_checks(ui, tr("Home"), "") {
            if let Some(p) = app.ui.present.take() {
                app.ui.paused_present = Some(p);
            }
            app.ui.home = true;
            ui.close();
        }
        ui.separator();
        for (i, name, dirty, active) in &docs {
            let label = if *dirty { format!("{name} •") } else { name.clone() };
            if crate::right::menu_check(ui, *active, &label, "") {
                switch = Some(*i);
                ui.close();
            }
        }
        if presenting && crate::right::menu_check(ui, app.ui.present.is_some(), tr("Present"), "") {
            resume_presentation(app);
            ui.close();
        }
        ui.separator();
        if crate::right::menu_item_beside_checks(ui, tr("New design file"), crate::shortcuts::shortcut("file.new").unwrap_or("")) {
            if let Some(p) = app.ui.present.take() {
                app.ui.paused_present = Some(p);
            }
            app.run("file.new", json!({}));
            app.ui.home = false;
            ui.close();
        }
        if editing {
            if crate::right::menu_item_beside_checks(ui, tr("Rename"), "") {
                app.ui.rename_file = true;
                ui.close();
            }
            if several && crate::right::menu_item_beside_checks(ui, tr("Open in new window"), "") {
                detach = Some(app.session.active);
                ui.close();
            }
            if crate::right::menu_item_beside_checks(ui, tr("Close tab"), "") {
                close = Some(app.session.active);
                ui.close();
            }
        }
    });
    if let Some(i) = switch {
        if let Some(p) = app.ui.present.take() {
            app.ui.paused_present = Some(p);
        }
        app.run("doc.switch", json!({"index": i}));
        app.ui.home = false;
    }
    if let Some(i) = detach
        && let Err(e) = crate::windows::open(app, &ctx, Some(i), None, None)
    {
        app.toast(e);
    }
    if let Some(i) = close {
        close_file(app, &ctx, i);
    }
}

/// Close open file `i`: a file with unsaved changes only with Alt held (a toast says so).
fn close_file(app: &mut FigApp, ctx: &egui::Context, i: usize) {
    // A branch tab's edits live on in its main file: closing it loses nothing.
    let dirty = app.session.docs.get(i).is_some_and(|d| d.is_dirty() && d.branch_of.is_none());
    if dirty {
        app.ui.find = None;
        app.toast(trf("Unsaved changes: save first, or close again with {key} to discard", &[("key", &crate::toolbar::pretty_shortcut("Alt"))]));
        if ctx.input(|inp| inp.modifiers.alt) {
            app.run("file.close", json!({"index": i, "force": true}));
        }
    } else {
        app.run("file.close", json!({"index": i}));
    }
    if app.ui.present_uid.is_some_and(|uid| !app.session.docs.iter().any(|d| d.uid == uid)) {
        app.ui.present = None;
        app.ui.paused_present = None;
        app.ui.present_uid = None;
    }
}

/// The share island: the theme, and while editing the people, Present and Share.
fn share_island(app: &mut FigApp, ui: &mut Ui, r: Rect, t: &Tokens, editing: bool) {
    island(ui, r, t);
    let theme_r = Rect::from_min_size(pos2(r.min.x + 8.0, r.center().y - 14.0), vec2(28.0, 28.0));
    let tresp = ui.interact(theme_r, ui.id().with("theme"), Sense::click()).tip(tr("Toggle light/dark theme"));
    if tresp.hovered() {
        ui.painter().rect_filled(theme_r, CornerRadius::same(8), t.hover);
    }
    icons::paint(ui.painter(), theme_r.shrink(7.0), if app.ui.dark { Icon::Sun } else { Icon::Moon }, t.icon);
    if tresp.clicked() {
        let cmd = if app.ui.dark { "view.themeLight" } else { "view.themeDark" };
        app.run(cmd, json!({}));
    }
    if editing {
        crate::right::share_cluster(app, ui, Rect::from_min_max(pos2(theme_r.max.x + 2.0, r.min.y), r.max));
    }
}

/// The command bar, an island centred on `centre` within `room` (narrower in a small window; not
/// drawn without room). A click opens the Actions palette, as its shortcut does.
fn command_bar(app: &mut FigApp, ui: &mut Ui, room: Rect, centre: f32) {
    const MAX_W: f32 = 440.0;
    const MIN_W: f32 = 200.0;
    let t = Tokens::get(ui.ctx());
    let w = MAX_W.min(room.width());
    if w < MIN_W {
        return;
    }
    let x = (centre - w / 2.0).clamp(room.min.x, room.max.x - w);
    let r = Rect::from_min_size(pos2(x, room.min.y), vec2(w, room.height()));
    let keys = crate::shortcuts::shortcut("app.actions").map(crate::toolbar::pretty_shortcut).unwrap_or_default();
    let resp = ui.interact(r, ui.id().with("command_bar"), Sense::click()).tip(crate::shortcuts::tip(tr("Actions"), "app.actions"));
    crate::a11y::menu_button(&resp, tr("Search or run a command"), app.ui.palette.is_some());
    app.ui.command_rect = Some(r);
    let open = app.ui.palette.is_some();
    let radius = CornerRadius::same(crate::ISLAND_R);
    ui.painter().rect_filled(r, radius, if resp.hovered() && !open { t.hover } else { t.panel });
    ui.painter().rect_stroke(r, radius, Stroke::new(1.0, if open { t.accent } else { t.border }), egui::StrokeKind::Inside);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 22.0, r.center().y), vec2(15.0, 15.0)), Icon::Search, t.icon_secondary);
    let mut text_right = r.max.x - 12.0;
    if !keys.is_empty() {
        let g = ui.painter().layout_no_wrap(keys, semibold(11.0), t.text_secondary);
        let kr = Rect::from_min_size(pos2(r.max.x - 10.0 - g.size().x - 14.0, r.center().y - 12.0), vec2(g.size().x + 14.0, 24.0));
        ui.painter().rect_filled(kr, CornerRadius::same(7), t.field);
        ui.painter().galley(kr.center() - g.size() / 2.0, g, t.text_secondary);
        text_right = kr.min.x - 8.0;
    }
    let hint = crate::widgets::elide_font(ui, tr("Search or run a command"), (text_right - r.min.x - 40.0).max(0.0), regular(12.5));
    ui.painter().text(pos2(r.min.x + 40.0, r.center().y), Align2::LEFT_CENTER, hint, regular(12.5), t.text_secondary);
    // While the palette is open, the keyboard is its: arrows moving focus up here and Enter
    // "clicking" the bar would close it instead of running its row.
    let toggle = if open { resp.clicked_by(egui::PointerButton::Primary) } else { resp.clicked() };
    if open && resp.has_focus() {
        resp.surrender_focus();
    }
    if toggle {
        crate::menus::toggle_palette(&mut app.ui);
    }
}

fn resume_presentation(app: &mut FigApp) {
    let Some(uid) = app.ui.present_uid else { return };
    let Some(index) = app.session.docs.iter().position(|d| d.uid == uid) else {
        app.ui.paused_present = None;
        app.ui.present_uid = None;
        return;
    };
    app.run("doc.switch", json!({"index": index}));
    app.ui.home = false;
    app.ui.present = app.ui.paused_present.take();
}

/// What the tab of document `uid` says about its live session: others get in (`true`: a session
/// this app hosts or joined, a file shared online), only you do (`false`: a file of yours on a
/// server, not shared now), or it has none.
pub fn live_mark(app: &FigApp, uid: u64) -> Option<bool> {
    let live = app.session.live_of(uid);
    live.shared().filter(|_| live.doc_uid == Some(uid))
}

/// Minimise, maximise/restore and close at the right end of the title bar; returns their width.
fn window_buttons(app: &mut FigApp, ui: &mut Ui, bar: Rect) -> f32 {
    const W: f32 = 46.0;
    let t = Tokens::get(ui.ctx());
    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let buttons = [
        (Icon::WinMinimize, tr("Minimize"), egui::ViewportCommand::Minimized(true)),
        (
            if max { Icon::WinRestore } else { Icon::WinMaximize },
            if max { tr("Restore") } else { tr("Maximize") },
            egui::ViewportCommand::Maximized(!max),
        ),
        (Icon::Close, tr("Close"), egui::ViewportCommand::Close),
    ];
    for (k, (icon, tip, cmd)) in buttons.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(bar.max.x - W * (3 - k) as f32, bar.min.y), vec2(W, bar.height()));
        let resp = ui.interact(r, ui.id().with(("window_button", k)), Sense::click()).tip(tip);
        let close = k == 2;
        let mut col = t.icon;
        if resp.hovered() {
            // Closing turns the button red, the others take the hover tint.
            let fill = if close { Color32::from_rgb(0xE8, 0x11, 0x23) } else { t.hover };
            ui.painter().rect_filled(r, CornerRadius::ZERO, fill);
            if close {
                col = Color32::WHITE;
            }
        }
        icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(14.0, 14.0)), icon, col);
        if resp.clicked() {
            if close {
                crate::windows::request_close(app, ui.ctx());
            } else {
                ui.ctx().send_viewport_cmd(cmd);
            }
        }
    }
    W * 3.0
}

/// Invisible strips along a frameless window's edges and corners that resize it.
pub fn resize_edges(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection as D};
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    const E: f32 = 5.0;
    let s = ctx.content_rect();
    let (x0, y0, x1, y1) = (s.min.x, s.min.y, s.max.x, s.max.y);
    let zones = [
        (Rect::from_min_max(pos2(x0 + E, y0), pos2(x1 - E, y0 + E)), D::North, CursorIcon::ResizeNorth),
        (Rect::from_min_max(pos2(x0 + E, y1 - E), pos2(x1 - E, y1)), D::South, CursorIcon::ResizeSouth),
        (Rect::from_min_max(pos2(x0, y0 + E), pos2(x0 + E, y1 - E)), D::West, CursorIcon::ResizeWest),
        (Rect::from_min_max(pos2(x1 - E, y0 + E), pos2(x1, y1 - E)), D::East, CursorIcon::ResizeEast),
        (Rect::from_min_max(pos2(x0, y0), pos2(x0 + E, y0 + E)), D::NorthWest, CursorIcon::ResizeNorthWest),
        (Rect::from_min_max(pos2(x1 - E, y0), pos2(x1, y0 + E)), D::NorthEast, CursorIcon::ResizeNorthEast),
        (Rect::from_min_max(pos2(x0, y1 - E), pos2(x0 + E, y1)), D::SouthWest, CursorIcon::ResizeSouthWest),
        (Rect::from_min_max(pos2(x1 - E, y1 - E), pos2(x1, y1)), D::SouthEast, CursorIcon::ResizeSouthEast),
    ];
    egui::Area::new(egui::Id::new("window_resize_edges")).order(egui::Order::Foreground).fixed_pos(s.min).interactable(true).show(ctx, |ui| {
        for (k, (r, dir, cursor)) in zones.into_iter().enumerate() {
            let resp = ui.interact(r, egui::Id::new(("window_edge", k)), Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(cursor);
            }
            if resp.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
            }
        }
    });
}
