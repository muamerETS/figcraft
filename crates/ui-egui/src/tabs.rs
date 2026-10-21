//! The top bar: the main menu, Home and a tab per open file, then the window's tools, the people,
//! Present and Share. In a frameless window it is also the title bar: its empty space drags the
//! window (double-click maximises), and it carries the window buttons; the window's edges resize
//! it.

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::a11y::Tip;
use crate::i18n::{tr, trf};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, semibold};
use crate::{FigApp, TitleBar};

/// The top bar's height.
pub const HEIGHT: f32 = 48.0;
/// A tab's height, and the width it never grows past or shrinks under.
const TAB_H: f32 = 34.0;
const TAB_MAX: f32 = 240.0;
const TAB_MIN: f32 = 96.0;

/// The top bar: the main menu, Home, the open files' tabs, a new file and the list of every open
/// file; at the right the command palette, the variables, the left panel, the theme, then the
/// people, Present and Share. While presenting, the presentation's controls sit between them.
pub fn tab_bar(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let editing = app.has_files() && !app.ui.home && app.ui.present.is_none();
    egui::Panel::top("tab_bar").exact_size(HEIGHT).show_separator_line(false).frame(egui::Frame::NONE.fill(t.tab_bar)).show(ui, |ui| {
        let rect = ui.max_rect();
        // Empty space moves the window (first, so the tabs and buttons on top take their clicks).
        window_drag(app, ui, rect, "titlebar_drag");
        ui.painter().hline(rect.x_range(), rect.max.y - 0.5, Stroke::new(1.0, t.border));
        let buttons_w = if app.ui.title_bar == TitleBar::Custom { window_buttons(app, ui, rect) } else { 0.0 };
        let cy = rect.center().y;
        let left = rect.min.x + if app.ui.title_bar == TitleBar::Inset { INSET_LEFT } else { 10.0 };
        let menu_end = main_menu_button(app, ui, &t, left, cy);
        let tools_start = right_tools(app, ui, &t, rect.max.x - buttons_w - 8.0, cy, editing);
        let tabs_end = file_tabs(app, ui, &t, menu_end + 8.0, tools_start - 12.0, cy, editing);
        // Presenting: the presentation's own controls between the tabs and the tools.
        if let Some(mut p) = app.ui.present.take() {
            let room = Rect::from_min_max(pos2(tabs_end + 12.0, cy - 20.0), pos2(tools_start - 12.0, cy + 20.0));
            let now = ui.input(|i| i.time);
            app.ui.present_action = crate::present::full_bar(&mut p, ui, room, now).or(app.ui.present_action);
            app.ui.present = Some(p);
        }
    });
}

/// The main menu (File, Edit, View…) at `x`; returns where it ends.
fn main_menu_button(app: &mut FigApp, ui: &mut Ui, t: &Tokens, x: f32, cy: f32) -> f32 {
    let lr = Rect::from_min_size(pos2(x, cy - 15.0), vec2(30.0, 30.0));
    let lresp = ui.interact(lr, ui.id().with("rail_menu"), Sense::click()).tip(tr("Main menu"));
    ui.painter().rect_filled(lr, CornerRadius::same(8), if lresp.hovered() || app.ui.main_menu { t.accent_hover } else { crate::theme::accent() });
    icons::paint(ui.painter(), Rect::from_center_size(lr.center(), vec2(16.0, 16.0)), Icon::Menu, Color32::WHITE);
    if lresp.clicked() {
        app.ui.main_menu = !app.ui.main_menu;
    }
    if app.ui.main_menu {
        crate::a11y::popup(&mut app.ui.nav, lresp.id, crate::menus::main_menu_id());
    }
    lr.max.x
}

/// A small square button on the bar, `active` while what it opens is open.
fn bar_button(ui: &mut Ui, t: &Tokens, r: Rect, id: &str, icon: Icon, tip: &str, active: bool) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(id), Sense::click()).tip(tip);
    crate::a11y::button(&resp, tip);
    if active || resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(8), if active { t.pressed } else { t.hover });
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
        icon,
        if active || resp.hovered() { t.icon } else { t.icon_secondary },
    );
    resp
}

/// The tools at the bar's right, ending at `right`: while editing the people, Present and Share,
/// and before them the command palette, the variables, the left panel and the theme. Returns
/// where they start.
fn right_tools(app: &mut FigApp, ui: &mut Ui, t: &Tokens, right: f32, cy: f32, editing: bool) -> f32 {
    let mut x = right;
    app.ui.command_rect = None;
    if editing {
        let share = Rect::from_min_max(pos2(right - 230.0, cy - 20.0), pos2(right + 8.0, cy + 20.0));
        crate::right::share_cluster(app, ui, share);
        x = share.min.x;
        let sep = x - 2.0;
        ui.painter().vline(sep, (cy - 10.0)..=(cy + 10.0), Stroke::new(1.0, t.border));
        x = sep - 6.0;
    }
    let next = |x: &mut f32| {
        let r = Rect::from_min_size(pos2(*x - 32.0, cy - 16.0), vec2(32.0, 32.0));
        *x -= 34.0;
        r
    };
    let r = next(&mut x);
    if bar_button(ui, t, r, "theme", if app.ui.dark { Icon::Sun } else { Icon::Moon }, tr("Toggle light/dark theme"), false).clicked() {
        let cmd = if app.ui.dark { "view.themeLight" } else { "view.themeDark" };
        app.run(cmd, json!({}));
    }
    if editing {
        let r = next(&mut x);
        if bar_button(ui, t, r, "hideui", Icon::Panels, tr("Collapse panel"), app.ui.left_collapsed).clicked() {
            app.ui.left_collapsed = !app.ui.left_collapsed;
        }
        let r = next(&mut x);
        if bar_button(ui, t, r, "variables", Icon::Variable, tr("Variables"), app.ui.variables.is_some()).clicked() {
            app.ui.variables.get_or_insert_with(Default::default);
        }
        let r = next(&mut x);
        let open = app.ui.palette.is_some();
        let tip = crate::shortcuts::tip(tr("Actions"), "app.actions");
        let resp = bar_button(ui, t, r, "command_bar", Icon::Search, tip, open);
        crate::a11y::menu_button(&resp, tr("Search or run a command"), open);
        app.ui.command_rect = Some(r);
        // While the palette is open, the keyboard is its: Enter "clicking" this button would
        // close it instead of running its row.
        let toggle = if open { resp.clicked_by(egui::PointerButton::Primary) } else { resp.clicked() };
        if open && resp.has_focus() {
            resp.surrender_focus();
        }
        if toggle {
            crate::menus::toggle_palette(&mut app.ui);
        }
    }
    x
}

/// One open file's tab, as laid out this frame.
struct Tab {
    index: usize,
    uid: u64,
    name: String,
    dirty: bool,
    live: Option<bool>,
    active: bool,
    width: f32,
}

/// Home, the open files' tabs (narrower when they don't all fit; the ones left over are in the
/// open files list), the presentation's tab, a new file and the open files list, from `x` up to
/// `max_x`. Returns where they end.
fn file_tabs(app: &mut FigApp, ui: &mut Ui, t: &Tokens, mut x: f32, max_x: f32, cy: f32, editing: bool) -> f32 {
    let font = semibold(12.5);
    let several = !ui.ctx().embed_viewports();
    // Home.
    let hr = Rect::from_min_size(pos2(x, cy - TAB_H / 2.0), vec2(TAB_H, TAB_H));
    let home_active = (app.ui.home || !app.has_files()) && app.ui.present.is_none();
    let hresp = ui.interact(hr, ui.id().with("home"), Sense::click()).tip(tr("Home"));
    crate::a11y::tab(&hresp, tr("Home"), home_active);
    tab_fill(ui, t, hr, home_active, hresp.hovered());
    icons::paint(
        ui.painter(),
        Rect::from_center_size(hr.center(), vec2(16.0, 16.0)),
        Icon::Home,
        if home_active { t.icon } else { t.icon_secondary },
    );
    if hresp.clicked() {
        if let Some(p) = app.ui.present.take() {
            app.ui.paused_present = Some(p);
        }
        app.ui.home = true;
    }
    x = hr.max.x + 4.0;
    let presenting = app.ui.present.is_some() || app.ui.paused_present.is_some();
    let mut tabs: Vec<Tab> = app
        .session
        .docs
        .iter()
        .enumerate()
        .filter(|(_, d)| app.shows(d.uid))
        .map(|(i, d)| {
            let live = live_mark(app, d.uid);
            let name = d.doc.name.clone();
            // The name, with the logo before it and the close button (and any marks) after it.
            let extra = 84.0 + if live.is_some() { 18.0 } else { 0.0 } + if d.is_dirty() { 10.0 } else { 0.0 };
            let natural = ui.painter().layout_no_wrap(name.clone(), font.clone(), t.text).size().x + extra;
            Tab {
                index: i,
                uid: d.uid,
                name,
                dirty: d.is_dirty(),
                live,
                active: i == app.session.active && editing,
                width: natural.clamp(TAB_MIN, TAB_MAX),
            }
        })
        .collect();
    // What follows the tabs: the presentation, a new file and the list.
    let tail = if presenting { 124.0 } else { 0.0 } + 4.0 + 30.0 + 4.0 + 26.0;
    let room = (max_x - x - tail).max(0.0);
    let gap = 4.0;
    let wanted: f32 = tabs.iter().map(|tab| tab.width + gap).sum();
    if wanted > room && !tabs.is_empty() {
        let each = (room / tabs.len() as f32 - gap).max(TAB_MIN);
        for tab in &mut tabs {
            tab.width = tab.width.min(each);
        }
        // Still too many: the ones that don't fit are left to the list (the active one stays).
        let mut used = 0.0;
        let mut kept = Vec::new();
        let active = tabs.iter().position(|tab| tab.active);
        let active_w = active.and_then(|a| tabs.get(a)).map_or(0.0, |tab| tab.width + gap);
        for (k, tab) in tabs.into_iter().enumerate() {
            let reserve = if Some(k) == active || active.is_none_or(|a| a < k) { 0.0 } else { active_w };
            if tab.active || used + tab.width + gap + reserve <= room {
                used += tab.width + gap;
                kept.push(tab);
            }
        }
        tabs = kept;
    }
    let mut switch = None;
    let mut close = None;
    let mut detach = None;
    // A tab dragged along the bar moves among the others.
    let mut dragging = None;
    let mut dropped = None;
    let mut visible = Vec::with_capacity(tabs.len());
    for tab in &tabs {
        let r = Rect::from_min_size(pos2(x, cy - TAB_H / 2.0), vec2(tab.width, TAB_H));
        visible.push((tab.index, r));
        let resp = ui.interact(r, ui.id().with(("tab", tab.uid)), Sense::click_and_drag());
        let label = match tab.live {
            Some(true) => trf("{name}, in a live session", &[("name", &tab.name)]),
            Some(false) => trf("{name}, online, open only to you", &[("name", &tab.name)]),
            None => tab.name.clone(),
        };
        let resp = resp.on_hover_text(&label);
        crate::a11y::tab(&resp, &label, tab.active);
        tab_fill(ui, t, r, tab.active, resp.hovered());
        // Dragged well off the bar: the file moves to a window of its own; let go on the bar: the
        // tab moves among the others.
        if (resp.dragged() || resp.drag_stopped())
            && let Some(p) = ui.input(|inp| inp.pointer.latest_pos())
        {
            if !resp.drag_stopped() {
                dragging = Some((tab.index, p));
            } else if several && (p.y - cy).abs() > HEIGHT * 2.0 {
                detach = Some((tab.index, Some(p)));
            } else if (p.y - cy).abs() <= HEIGHT / 2.0 + 12.0 {
                dropped = Some((tab.index, p.x));
            }
        }
        resp.context_menu(|ui| {
            ui.set_width(200.0);
            if crate::menus::item(ui, tr("Rename"), "", true, false, t) {
                switch = Some(tab.index);
                app.ui.rename_file = true;
                ui.close();
            }
            if several && crate::menus::item(ui, tr("Open in new window"), "", true, false, t) {
                detach = Some((tab.index, None));
                ui.close();
            }
            if crate::menus::item(ui, tr("Close tab"), "", true, false, t) {
                close = Some(tab.index);
                ui.close();
            }
        });
        let ink = if tab.active { t.text } else { t.text_secondary };
        let mut name_x = r.min.x + 12.0;
        if tab.width >= 120.0 {
            icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 19.0, cy), vec2(14.0, 14.0)), Icon::Logo, crate::theme::accent());
            name_x = r.min.x + 33.0;
        }
        let close_r = Rect::from_center_size(pos2(r.max.x - 16.0, cy), vec2(20.0, 20.0));
        let mut name_end = close_r.min.x - 6.0;
        if let Some(shared) = tab.live {
            let at = Rect::from_center_size(pos2(name_end - 7.0, cy), vec2(14.0, 14.0));
            icons::paint(ui.painter(), at, crate::online::mark(shared), if shared { t.success } else { t.icon_secondary });
            name_end = at.min.x - 4.0;
        }
        let name_r = Rect::from_min_max(pos2(name_x, cy - 10.0), pos2(name_end, cy + 10.0));
        let rename_id = Id::new("filename");
        let renaming = tab.active && (app.ui.rename_file || ui.data(|d| d.get_temp::<String>(rename_id.with("buf")).is_some()));
        if renaming {
            let start = std::mem::take(&mut app.ui.rename_file);
            if let (Some(new), _) = crate::widgets::rename_field(ui, rename_id, &tab.name, name_r, font.clone(), start) {
                app.run("file.rename", json!({"name": new}));
            }
        } else {
            let shown = crate::bidi::paint_name(ui.painter(), pos2(name_r.min.x, cy), &tab.name, font.clone(), ink, (name_r.width() - 10.0).max(0.0));
            if tab.dirty {
                ui.painter().circle_filled(pos2(shown.max.x + 6.0, cy), 3.0, t.warning);
            }
        }
        if resp.hovered() || tab.active {
            let cresp = ui.interact(close_r, ui.id().with(("tabclose", tab.uid)), Sense::click()).tip(tr("Close tab"));
            if cresp.hovered() {
                ui.painter().rect_filled(close_r, CornerRadius::same(5), t.pressed);
            }
            icons::paint(ui.painter(), Rect::from_center_size(close_r.center(), vec2(10.0, 10.0)), Icon::Close, t.icon_secondary);
            if cresp.clicked() {
                close = Some(tab.index);
            }
        }
        if resp.double_clicked() && tab.active {
            app.ui.rename_file = true;
        } else if resp.clicked() && close.is_none() && !tab.active {
            switch = Some(tab.index);
        }
        if resp.middle_clicked() {
            close = Some(tab.index);
        }
        x = r.max.x + gap;
    }
    if let Some((from, p)) = dragging
        && (p.y - cy).abs() <= HEIGHT / 2.0 + 12.0
        && let Some((_, at)) = tab_drop_position(&visible, from, p.x)
    {
        let at = at - gap / 2.0;
        ui.painter().line_segment([pos2(at, cy - TAB_H / 2.0 + 4.0), pos2(at, cy + TAB_H / 2.0 - 4.0)], Stroke::new(3.0, crate::theme::accent()));
    }
    let reorder = dropped.and_then(|(from, at)| tab_drop_position(&visible, from, at).map(|(to, _)| (from, to)));
    // Presentations live in their own tab; switching back to the design keeps the player state.
    if presenting {
        let r = Rect::from_min_size(pos2(x, cy - TAB_H / 2.0), vec2(120.0, TAB_H));
        let resp = ui.interact(r, ui.id().with("presentation_tab"), Sense::click()).tip(tr("Present"));
        let active = app.ui.present.is_some();
        crate::a11y::tab(&resp, tr("Present"), active);
        tab_fill(ui, t, r, active, resp.hovered());
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 19.0, cy), vec2(14.0, 14.0)), Icon::Play, t.icon);
        ui.painter().text(pos2(r.min.x + 33.0, cy), Align2::LEFT_CENTER, tr("Present"), font.clone(), if active { t.text } else { t.text_secondary });
        let cr = Rect::from_center_size(pos2(r.max.x - 16.0, cy), vec2(20.0, 20.0));
        let mut closed = false;
        if resp.hovered() || active {
            let close_resp = ui.interact(cr, ui.id().with("presentation_close"), Sense::click()).tip(tr("Close preview"));
            if close_resp.hovered() {
                ui.painter().rect_filled(cr, CornerRadius::same(5), t.pressed);
            }
            icons::paint(ui.painter(), Rect::from_center_size(cr.center(), vec2(10.0, 10.0)), Icon::Close, t.icon_secondary);
            if close_resp.clicked() {
                app.ui.present = None;
                app.ui.paused_present = None;
                app.ui.present_uid = None;
                closed = true;
            }
        }
        if resp.clicked() && !active && !closed {
            resume_presentation(app);
        }
        x = r.max.x + gap;
    }
    // A new file.
    let nr = Rect::from_min_size(pos2(x, cy - 15.0), vec2(30.0, 30.0));
    let nresp = ui.interact(nr, ui.id().with("newtab"), Sense::click()).tip(crate::shortcuts::tip(tr("New design file"), "file.new"));
    if nresp.hovered() {
        ui.painter().rect_filled(nr, CornerRadius::same(8), t.hover);
    }
    icons::paint(ui.painter(), Rect::from_center_size(nr.center(), vec2(14.0, 14.0)), Icon::Plus, t.icon_secondary);
    if nresp.clicked() {
        if let Some(p) = app.ui.present.take() {
            app.ui.paused_present = Some(p);
        }
        app.run("file.new", json!({}));
        app.ui.home = false;
    }
    // Every open file, Home and the file commands.
    let br = Rect::from_min_size(pos2(nr.max.x + 4.0, cy - 13.0), vec2(26.0, 26.0));
    let files = ui.interact(br, ui.id().with("open_files"), Sense::click()).tip(tr("Open files"));
    crate::a11y::menu_button(&files, tr("Open files"), crate::a11y::menu_open(&files));
    if files.hovered() {
        ui.painter().rect_filled(br, CornerRadius::same(8), t.hover);
    }
    icons::paint(ui.painter(), Rect::from_center_size(br.center(), vec2(10.0, 10.0)), Icon::Chevron, t.icon_secondary);
    files_menu(app, ui, &files, editing);
    if let Some(i) = switch {
        if let Some(p) = app.ui.present.take() {
            app.ui.paused_present = Some(p);
        }
        app.run("doc.switch", json!({"index": i}));
        app.ui.home = false;
    }
    let ctx = ui.ctx().clone();
    if let Some((i, at)) = detach {
        // Where the pointer let go, on the screen.
        let at = at.and_then(|p| ui.input(|inp| inp.viewport().inner_rect).map(|r| r.min + p.to_vec2() - vec2(60.0, HEIGHT / 2.0)));
        if let Err(e) = crate::windows::open(app, &ctx, Some(i), None, at) {
            app.toast(e);
        }
    }
    if let Some((from, to)) = reorder
        && from != to
    {
        app.run("file.moveTab", json!({"from": from, "to": to}));
    }
    if let Some(i) = close {
        close_file(app, &ctx, i);
    }
    br.max.x
}

/// Final document index and insertion marker for a tab dropped among the visible tabs.
fn tab_drop_position(visible: &[(usize, Rect)], from: usize, pointer_x: f32) -> Option<(usize, f32)> {
    let others: Vec<_> = visible.iter().copied().filter(|(index, _)| *index != from).collect();
    let slot = others.partition_point(|(_, rect)| rect.center().x < pointer_x);
    if let Some(&(index, rect)) = others.get(slot) {
        Some((index - usize::from(from < index), rect.min.x))
    } else {
        let &(index, rect) = others.last()?;
        Some((index + usize::from(from > index), rect.max.x))
    }
}

/// A tab's background: the panel's colour and edge for the shown tab, a tint under the pointer.
fn tab_fill(ui: &Ui, t: &Tokens, r: Rect, active: bool, hovered: bool) {
    let radius = CornerRadius::same(10);
    if active {
        ui.painter().rect_filled(r, radius, t.panel);
        ui.painter().rect_stroke(r, radius, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    } else if hovered {
        ui.painter().rect_filled(r, radius, t.hover);
    }
}

/// Without the system's title bar, `rect` moves the window and a double-click on it maximises.
/// Drawn before what sits on it, so that takes its own clicks.
pub(crate) fn window_drag(app: &FigApp, ui: &Ui, rect: Rect, salt: &str) {
    if app.ui.title_bar == TitleBar::Native {
        return;
    }
    let drag = ui.interact(rect, ui.id().with(salt), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

/// How far the islands keep from the window's top-left corner: on macOS the system's window
/// buttons sit there.
pub(crate) const INSET_LEFT: f32 = 76.0;
pub(crate) const INSET_TOP: f32 = 36.0;

/// The open files, Home, the presentation and the file commands: what the tab strip offered.
pub(crate) fn files_menu(app: &mut FigApp, ui: &Ui, anchor: &egui::Response, editing: bool) {
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
    let presenting_now = app.ui.present.is_some();
    let mut switch = None;
    let mut close = None;
    let mut detach = None;
    let ctx = ui.ctx().clone();
    crate::right::menu_popup(anchor).width(240.0).show(|ui| {
        if crate::right::menu_check(ui, !editing && !presenting_now, tr("Home"), "") {
            if let Some(p) = app.ui.present.take() {
                app.ui.paused_present = Some(p);
            }
            app.ui.home = true;
            ui.close();
        }
        if !docs.is_empty() || presenting {
            ui.separator();
        }
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
        if crate::right::menu_item_beside_checks(ui, tr("Open file"), crate::shortcuts::shortcut("file.open").unwrap_or("")) {
            app.open_dialog();
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
pub(crate) fn window_buttons(app: &mut FigApp, ui: &mut Ui, bar: Rect) -> f32 {
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

#[cfg(test)]
mod layout_tests {
    use super::tab_drop_position;
    use egui::{Rect, pos2};

    #[test]
    fn drop_position_places_a_tab_between_its_neighbors() {
        let visible = (0..4)
            .map(|index| (index, Rect::from_min_max(pos2(index as f32 * 120.0, 0.0), pos2((index + 1) as f32 * 120.0, 40.0))))
            .collect::<Vec<_>>();
        assert_eq!(tab_drop_position(&visible, 0, 330.0).map(|(to, _)| to), Some(2));
        assert_eq!(tab_drop_position(&visible, 3, 30.0).map(|(to, _)| to), Some(0));
    }
}
