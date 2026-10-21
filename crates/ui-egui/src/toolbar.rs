//! The floating tool rail at the canvas's left edge (tool groups with flyouts, resources,
//! actions, modes), and the bottom pills of the edit modes (vector edit, shape builder, view only).

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use figcraft_engine::ToolId;
use figcraft_engine::vectoredit::VTool;
use serde_json::json;

use crate::FigApp;
use crate::a11y::{self, Nav, Region, Tip};
use crate::i18n::{tr, tr_noop, trf};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, accent, regular};

struct Group {
    key: &'static str,
    /// The flyout button's name for screen readers.
    name: &'static str,
    tools: &'static [(ToolId, Icon)],
}

const GROUPS: &[Group] = &[
    Group {
        key: "move",
        name: tr_noop("Move tools"),
        tools: &[(ToolId::Move, Icon::Move), (ToolId::Hand, Icon::Hand), (ToolId::Scale, Icon::Scale)],
    },
    Group {
        key: "frame",
        name: tr_noop("Frame tools"),
        tools: &[(ToolId::Frame, Icon::Frame), (ToolId::Section, Icon::Section), (ToolId::Slice, Icon::Slice)],
    },
    Group {
        key: "shape",
        name: tr_noop("Shape tools"),
        tools: &[
            (ToolId::Rectangle, Icon::Rectangle),
            (ToolId::Line, Icon::Line),
            (ToolId::Arrow, Icon::Arrow),
            (ToolId::Ellipse, Icon::Ellipse),
            (ToolId::Polygon, Icon::Polygon),
            (ToolId::Star, Icon::Star),
        ],
    },
    Group {
        key: "pen",
        name: tr_noop("Drawing tools"),
        tools: &[(ToolId::Pen, Icon::Pen), (ToolId::Pencil, Icon::Pencil), (ToolId::ShapeBuilder, Icon::ShapeBuilder)],
    },
    Group { key: "text", name: tr_noop("Text"), tools: &[(ToolId::Text, Icon::Text)] },
    Group { key: "comment", name: tr_noop("Comment"), tools: &[(ToolId::Comment, Icon::Comment)] },
];

fn remembered(ui: &Ui, key: &str, default: ToolId) -> ToolId {
    ui.data(|d| d.get_temp::<u8>(egui::Id::new(("tb_last", key)))).and_then(|i| ToolId::ALL.get(i as usize).copied()).unwrap_or(default)
}

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let canvas = app.canvas.rect;
    app.ui.actions_rect = None;
    // A file you can't edit now (view only, or on its way to a server): moving around only.
    if let Some(why) = app.session.active_read_only() {
        view_only_toolbar(app, ui, why);
        return;
    }
    // Vector edit mode swaps the tools for the edit tools.
    if let Some(tool) = app.session.ds().ok().and_then(|d| d.vector_edit.as_ref()).map(|v| v.tool) {
        edit_toolbar(app, ui, tool);
        return;
    }
    // So does the shape builder, for its merge/remove modes.
    if app.session.tool.tool == ToolId::ShapeBuilder {
        builder_toolbar(app, ui);
        return;
    }
    main_toolbar(app, ui, canvas, &t);
}

/// The tool rail: one cell per tool group (a mark in its corner when it has a flyout), Comment
/// below a divider, then Actions.
const RAIL_W: f32 = 52.0;
const CELL_W: f32 = 40.0;
const CELL_H: f32 = 36.0;
const ROW: f32 = 40.0;

fn rail_height() -> f32 {
    6.0 + (GROUPS.len() + 1) as f32 * ROW + 9.0 + 6.0
}

/// Where the tool rail sits: 12 px in from the canvas's left edge, centred on it (from its top in
/// a canvas too short for it).
pub(crate) fn rail_rect(canvas: Rect) -> Rect {
    let h = rail_height();
    let top = (canvas.center().y - h / 2.0).max(canvas.min.y + 12.0);
    Rect::from_min_size(pos2(canvas.min.x + 12.0, top), vec2(RAIL_W, h))
}

fn main_toolbar(app: &mut FigApp, ui: &mut Ui, canvas: Rect, t: &Tokens) {
    let rect = pill_at(ui, &mut app.ui.nav, rail_rect(canvas), t);
    let current = app.session.tool.tool;
    let x = rect.min.x + (RAIL_W - CELL_W) / 2.0;
    let mut y = rect.min.y + 6.0;
    for g in GROUPS {
        if g.key == "comment" {
            hdivider(ui, t, rect, y + 4.5);
            y += 9.0;
        }
        let in_group = g.tools.iter().any(|(tid, _)| *tid == current);
        let shown = if in_group { current } else { remembered(ui, g.key, g.tools.first().map(|x| x.0).unwrap_or(ToolId::Move)) };
        let icon = g.tools.iter().find(|(tid, _)| *tid == shown).map(|(_, i)| *i).unwrap_or(Icon::Move);
        let r = Rect::from_min_size(pos2(x, y + (ROW - CELL_H) / 2.0), vec2(CELL_W, CELL_H));
        let resp = tool_cell(ui, t, r, ("tool", g.key), icon, in_group, &format!("{}  {}", tr(shown.label()), pretty_shortcut(shown.shortcut())));
        if in_group {
            a11y::prefer(&mut app.ui.nav, Region::Toolbar, resp.id);
        }
        if resp.clicked() {
            // Enter (a keyboard click) on the shape or frame tool already chosen places one; the
            // canvas takes the keyboard.
            if in_group && shown.draws_shape() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                app.run("tool.place", json!({}));
                app.ui.nav.go(Region::Canvas);
            } else {
                app.run("tool.set", json!({"tool": shown.id()}));
            }
        }
        // A group's other tools: the mark in the cell's corner, or a right-click on the cell.
        if g.tools.len() > 1 {
            let cr = Rect::from_min_size(pos2(r.max.x - 13.0, r.max.y - 13.0), vec2(13.0, 13.0));
            let cresp = ui.interact(cr, ui.id().with(("flyout", g.key)), Sense::click());
            let open = app.ui.tool_flyout == Some(g.key);
            a11y::menu_button(&cresp, tr(g.name), open);
            if open {
                a11y::popup(&mut app.ui.nav, cresp.id, flyout_id(g.key));
            }
            let col = if in_group {
                Color32::WHITE
            } else if cresp.hovered() || open {
                t.icon
            } else {
                t.icon_secondary
            };
            let (a, b) = (r.max - vec2(5.0, 10.0), r.max - vec2(5.0, 5.0));
            ui.painter().add(egui::Shape::convex_polygon(vec![a, b, r.max - vec2(10.0, 5.0)], col, Stroke::NONE));
            if cresp.clicked() || resp.secondary_clicked() {
                app.ui.tool_flyout = if open { None } else { Some(g.key) };
            }
        }
        if app.ui.tool_flyout == Some(g.key) {
            flyout(app, ui, g, pos2(rect.max.x + 8.0, r.min.y - 8.0));
        }
        y += ROW;
    }
    // Actions: the palette (on while it is open; clicking again closes it).
    let r = Rect::from_min_size(pos2(x, y + (ROW - CELL_H) / 2.0), vec2(CELL_W, CELL_H));
    let resp = ui.interact(r, ui.id().with("actions"), Sense::click()).tip(crate::shortcuts::tip(tr("Actions"), "app.actions"));
    app.ui.actions_rect = Some(r);
    let actions_open = app.ui.palette.is_some();
    if actions_open {
        ui.painter().rect_filled(r, CornerRadius::same(8), accent());
    } else if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(8), t.hover);
    }
    icons::paint(
        ui.painter(),
        Rect::from_center_size(r.center(), vec2(18.0, 18.0)),
        Icon::Actions,
        if actions_open { Color32::WHITE } else { t.component },
    );
    if resp.clicked() {
        crate::menus::toggle_palette(&mut app.ui);
    }
    zoom_island(app, ui, canvas, t);
    status_chip(app, ui, canvas, t);
}

/// Zoom out, the zoom (its menu) and zoom in, and help, floating at the canvas's bottom right.
fn zoom_island(app: &mut FigApp, ui: &mut Ui, canvas: Rect, t: &Tokens) {
    const C: f32 = 30.0;
    let zoom = app.session.ds().map(|d| d.view.zoom).unwrap_or(1.0);
    let pct = format!("{}%", (zoom * 100.0).round());
    let w = 6.0 + C + 58.0 + C + 9.0 + C + 6.0;
    let rect = Rect::from_min_size(pos2(canvas.max.x - 12.0 - w, canvas.max.y - 12.0 - 40.0), vec2(w, 40.0));
    float_at(ui, rect, t);
    a11y::toolbar_area(&mut app.ui.nav, rect);
    let cy = rect.center().y;
    let mut x = rect.min.x + 6.0;
    let cell = |ui: &mut Ui, x: &mut f32, w: f32, id: &str| {
        let r = Rect::from_min_size(pos2(*x, cy - 15.0), vec2(w, 30.0));
        *x += w;
        let resp = ui.interact(r, ui.id().with(("zoom_island", id)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
        }
        resp
    };
    let out = cell(ui, &mut x, C, "out").tip(crate::shortcuts::tip(tr("Zoom out"), "view.zoomOut"));
    icons::paint(ui.painter(), out.rect.shrink(8.0), Icon::Minus, t.icon);
    let menu = cell(ui, &mut x, 58.0, "menu");
    let value = if a11y::active(&menu.ctx) { pct.clone() } else { String::new() };
    a11y::combo(&menu, tr("Zoom"), &value, a11y::menu_open(&menu));
    ui.painter().text(menu.rect.center(), Align2::CENTER_CENTER, &pct, crate::theme::semibold(12.0), t.text);
    let zin = cell(ui, &mut x, C, "in").tip(crate::shortcuts::tip(tr("Zoom in"), "view.zoomIn"));
    icons::paint(ui.painter(), zin.rect.shrink(8.0), Icon::Plus, t.icon);
    ui.painter().line_segment([pos2(x + 4.0, cy - 10.0), pos2(x + 4.0, cy + 10.0)], Stroke::new(1.0, t.border));
    x += 9.0;
    let help = cell(ui, &mut x, C, "help").tip(tr("Help and resources"));
    ui.painter().text(help.rect.center(), Align2::CENTER_CENTER, "?", crate::theme::semibold(13.0), t.text);
    if out.clicked() {
        app.run("view.zoomOut", json!({}));
    }
    if zin.clicked() {
        app.run("view.zoomIn", json!({}));
    }
    crate::right::zoom_menu(app, &menu);
    crate::right::menu_popup(&help).show(|ui| {
        for (l, c) in [
            (tr("Getting started"), "help.gettingStarted"),
            (tr("Keyboard shortcuts"), "help.shortcuts"),
            (tr("Help page"), "help.page"),
            (tr("About FigCraft"), "help.about"),
        ] {
            if crate::right::menu_item(ui, l, crate::shortcuts::shortcut(c).unwrap_or("")) {
                app.run(c, json!({}));
                ui.close();
            }
        }
    });
}

/// Whether the file is saved, floating at the canvas's bottom left.
fn status_chip(app: &mut FigApp, ui: &mut Ui, canvas: Rect, t: &Tokens) {
    let dirty = app.session.ds().is_ok_and(|d| d.is_dirty());
    let (label, dot) = if dirty { (tr("Unsaved changes"), t.warning) } else { (tr("Saved"), t.success) };
    let g = ui.painter().layout_no_wrap(label.to_string(), crate::theme::semibold(11.0), t.text_secondary);
    let rect = Rect::from_min_size(pos2(canvas.min.x + 12.0, canvas.max.y - 12.0 - 30.0), vec2(12.0 + 8.0 + 8.0 + g.size().x + 14.0, 30.0));
    if rect.max.x > canvas.max.x - 200.0 {
        return;
    }
    let resp = ui.interact(rect, ui.id().with("status_chip"), Sense::hover());
    a11y::status(&resp, label, "");
    let radius = CornerRadius::same(11);
    ui.painter().rect_filled(rect, radius, t.panel);
    ui.painter().rect_stroke(rect, radius, Stroke::new(1.0, t.border), StrokeKind::Inside);
    ui.painter().circle_filled(pos2(rect.min.x + 16.0, rect.center().y), 4.0, dot);
    ui.painter().galley(pos2(rect.min.x + 28.0, rect.center().y - g.size().y / 2.0), g, t.text_secondary);
}

/// The pill shown for a file you can't edit now: Move and Hand, and a badge saying why ("View
/// only", or "Publishing…" until the file is on the server).
fn view_only_toolbar(app: &mut FigApp, ui: &mut Ui, why: &'static str) {
    let t = Tokens::get(ui.ctx());
    // The editing tools are off: the tool falls back to moving around.
    if !matches!(app.session.tool.tool, ToolId::Move | ToolId::Hand) {
        app.run("tool.set", json!({"tool": "move"}));
    }
    let label = if why == figcraft_engine::live::PUBLISHING { tr("Publishing…") } else { tr("View only") };
    let cell = 32.0;
    let text = ui.painter().layout_no_wrap(label.to_string(), crate::theme::semibold(11.5), t.text);
    let badge_w = 12.0 + 14.0 + 6.0 + text.size().x + 12.0;
    let width = 2.0 * (cell + 4.0) + 8.0 + 12.0 + badge_w + 8.0;
    let rect = pill(ui, &mut app.ui.nav, app.canvas.rect, width, &t);
    let cy = rect.center().y;
    let mut x = rect.min.x + 8.0;
    for (tool, icon) in [(ToolId::Move, Icon::Move), (ToolId::Hand, Icon::Hand)] {
        let r = Rect::from_center_size(pos2(x + cell / 2.0, cy), vec2(cell, cell));
        let active = app.session.tool.tool == tool;
        let resp = tool_cell(ui, &t, r, ("tool", tool.id()), icon, active, &format!("{}  {}", tr(tool.label()), pretty_shortcut(tool.shortcut())));
        if active {
            a11y::prefer(&mut app.ui.nav, Region::Toolbar, resp.id);
        }
        if resp.clicked() {
            app.run("tool.set", json!({"tool": tool.id()}));
        }
        x += cell + 4.0;
    }
    divider(ui, &t, x, cy);
    let badge = Rect::from_min_size(pos2(x + 12.0, cy - 13.0), vec2(badge_w, 26.0));
    let resp = ui.interact(badge, ui.id().with("view_only"), Sense::hover()).on_hover_text(why);
    a11y::status(&resp, label, why);
    ui.painter().rect_filled(badge, CornerRadius::same(10), t.field);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(badge.min.x + 12.0 + 7.0, cy), vec2(14.0, 14.0)), Icon::Eye, t.icon_secondary);
    ui.painter().galley(pos2(badge.min.x + 12.0 + 14.0 + 6.0, cy - text.size().y / 2.0), text, t.text);
}

/// The pill shown in vector edit mode: the edit tools and Done.
fn edit_toolbar(app: &mut FigApp, ui: &mut Ui, current: VTool) {
    let t = Tokens::get(ui.ctx());
    let canvas = app.canvas.rect;
    let cell = 32.0;
    let width = VTool::ALL.len() as f32 * (cell + 4.0) + 8.0 + 12.0 + DONE_W + 8.0;
    let rect = pill(ui, &mut app.ui.nav, canvas, width, &t);
    let cy = rect.center().y;
    let mut x = rect.min.x + 8.0;
    // Holding Cmd with the Move tool bends: show it.
    let held = current == VTool::Move && ui.input(|i| i.modifiers.command);
    for tool in VTool::ALL {
        let r = Rect::from_center_size(pos2(x + cell / 2.0, cy), vec2(cell, cell));
        let tip = if tool == VTool::Bend {
            trf("{tool}  hold {key}", &[("tool", &tr(tool.label())), ("key", &pretty_shortcut(tool.shortcut()))])
        } else {
            format!("{}  {}", tr(tool.label()), pretty_shortcut(tool.shortcut()))
        };
        let active = tool == current && !held || held && tool == VTool::Bend;
        let icon = match tool {
            VTool::Move => Icon::Move,
            VTool::Pen => Icon::Pen,
            VTool::Bend => Icon::Bend,
            VTool::Paint => Icon::PaintBucket,
            VTool::Lasso => Icon::Lasso,
            VTool::Cut => Icon::Cut,
            VTool::Eraser => Icon::Eraser,
            VTool::Width => Icon::VariableWidth,
        };
        let resp = tool_cell(ui, &t, r, ("vtool", tool.id()), icon, active, &tip);
        if active {
            a11y::prefer(&mut app.ui.nav, Region::Toolbar, resp.id);
        }
        if resp.clicked() {
            app.run("vector.edit.tool", json!({"tool": tool.id()}));
        }
        x += cell + 4.0;
    }
    if done_button(ui, &t, x, cy, &format!("{}  Esc", tr("Done editing"))) {
        app.run("vector.edit.end", json!({}));
    }
}

/// The pill shown while the shape builder is active: Merge and Remove modes, a hint and Done.
fn builder_toolbar(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let cell = 32.0;
    let remove = app.session.tool.builder_remove;
    let has_regions = figcraft_engine::shapebuilder::current(&app.session).is_some_and(|a| !a.regions.is_empty());
    let alt = pretty_shortcut("Alt");
    let hint = if !has_regions {
        tr("Select overlapping shapes").to_string()
    } else if remove {
        trf("Click or drag to remove · {alt} merges", &[("alt", &alt)])
    } else {
        trf("Click or drag to merge · {alt} removes", &[("alt", &alt)])
    };
    let galley = ui.painter().layout_no_wrap(hint, regular(12.0), t.text_secondary);
    let width = 2.0 * (cell + 4.0) + 8.0 + 12.0 + galley.size().x + 12.0 + 12.0 + DONE_W + 8.0;
    let rect = pill(ui, &mut app.ui.nav, app.canvas.rect, width, &t);
    let cy = rect.center().y;
    let mut x = rect.min.x + 8.0;
    for (mode, icon, tip) in [("merge", Icon::ShapeBuilder, tr("Merge regions")), ("remove", Icon::ShapeBuilderRemove, tr("Remove regions"))] {
        let r = Rect::from_center_size(pos2(x + cell / 2.0, cy), vec2(cell, cell));
        let active = (mode == "remove") == remove;
        let resp = tool_cell(ui, &t, r, ("sbmode", mode), icon, active, tip);
        if active {
            a11y::prefer(&mut app.ui.nav, Region::Toolbar, resp.id);
        }
        if resp.clicked() {
            app.run("shapeBuilder.mode", json!({"mode": mode}));
        }
        x += cell + 4.0;
    }
    divider(ui, &t, x, cy);
    x += 12.0;
    let w = galley.size().x;
    ui.painter().galley(pos2(x, cy - galley.size().y / 2.0), galley, t.text_secondary);
    x += w + 12.0;
    if done_button(ui, &t, x, cy, &format!("{}  {}", tr("Done"), pretty_shortcut("Enter"))) {
        app.run("tool.set", json!({"tool": "move"}));
    }
}

/// Where a toolbar pill `width` wide sits: centred 12 px above the canvas's bottom edge.
pub(crate) fn pill_rect(canvas: Rect, width: f32) -> Rect {
    Rect::from_center_size(pos2(canvas.center().x, canvas.max.y - 12.0 - 24.0), vec2(width, 48.0))
}

/// The floating pill (shadow, panel, border) `width` wide at the bottom centre of the canvas: the
/// toolbar region keyboard navigation moves through.
fn pill(ui: &Ui, nav: &mut Nav, canvas: Rect, width: f32, t: &Tokens) -> Rect {
    pill_at(ui, nav, pill_rect(canvas, width), t)
}

/// A floating panel at `rect` that takes the presses over it (shadow, panel, border).
fn float_at(ui: &Ui, rect: Rect, t: &Tokens) {
    ui.interact(rect, ui.id().with(("float", rect.min.x as i32, rect.min.y as i32)), Sense::CLICK | Sense::DRAG);
    let painter = ui.painter();
    crate::widgets::soft_shadow(painter, rect, PILL_R as u8);
    painter.rect_filled(rect, CornerRadius::same(PILL_R as u8), t.panel);
    painter.rect_stroke(rect, CornerRadius::same(PILL_R as u8), Stroke::new(1.0, t.border), StrokeKind::Inside);
}

/// The floating pill at `rect`.
fn pill_at(ui: &Ui, nav: &mut Nav, rect: Rect, t: &Tokens) -> Rect {
    // The pill takes the press and the drag over it: its buttons sense clicks only, and egui gives
    // a drag the canvas under them would sense to the canvas, so the pen placed an anchor there.
    // Not focusable: the keyboard walks the buttons, not the pill.
    ui.interact(rect, ui.id().with("pill"), Sense::CLICK | Sense::DRAG);
    a11y::area(nav, Region::Toolbar, rect);
    a11y::container_bounds(ui, rect);
    let painter = ui.painter();
    crate::widgets::soft_shadow(painter, rect, PILL_R as u8);
    painter.rect_filled(rect, CornerRadius::same(PILL_R as u8), t.panel);
    painter.rect_stroke(rect, CornerRadius::same(PILL_R as u8), Stroke::new(1.0, t.border), StrokeKind::Inside);
    rect
}

const PILL_R: f32 = 16.0;

/// A horizontal separator across the rail `rect` at `y`.
fn hdivider(ui: &Ui, t: &Tokens, rect: Rect, y: f32) {
    ui.painter().line_segment([pos2(rect.min.x + 12.0, y), pos2(rect.max.x - 12.0, y)], Stroke::new(1.0, t.border));
}

/// A square tool button: accent when active, tinted on hover; `tip` is its tooltip and name.
fn tool_cell(ui: &mut Ui, t: &Tokens, r: Rect, id: impl egui::AsIdSalt, icon: Icon, active: bool, tip: &str) -> egui::Response {
    let resp = ui.interact(r, ui.id().with(id), Sense::click()).on_hover_ui(|ui| {
        ui.label(tip);
    });
    a11y::toggle(&resp, tip, active);
    let bg = if active {
        accent()
    } else if resp.hovered() {
        t.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(r, CornerRadius::same(8), bg);
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(18.0, 18.0)), icon, if active { Color32::WHITE } else { t.icon });
    resp
}

/// A vertical separator just after `x`.
fn divider(ui: &Ui, t: &Tokens, x: f32, cy: f32) {
    ui.painter().line_segment([pos2(x + 2.0, cy - 14.0), pos2(x + 2.0, cy + 14.0)], Stroke::new(1.0, t.border));
}

const DONE_W: f32 = 56.0;

/// A divider at `x` and the accent Done button after it; true when clicked.
fn done_button(ui: &mut Ui, t: &Tokens, x: f32, cy: f32, tip: &str) -> bool {
    divider(ui, t, x, cy);
    let done = Rect::from_min_size(pos2(x + 12.0, cy - 14.0), vec2(DONE_W, 28.0));
    let resp = ui.interact(done, ui.id().with("tool_done"), Sense::click()).tip(tip);
    ui.painter().rect_filled(done, CornerRadius::same(6), if resp.hovered() { t.accent_hover } else { accent() });
    ui.painter().text(done.center(), Align2::CENTER_CENTER, tr("Done"), crate::theme::semibold(11.5), Color32::WHITE);
    resp.clicked()
}

/// A group's flyout, opening right of the rail from `top_left` (up from the window's bottom edge
/// when it wouldn't fit below).
fn flyout(app: &mut FigApp, ui: &mut Ui, g: &Group, top_left: egui::Pos2) {
    let t = Tokens::get(ui.ctx());
    let item_h = 32.0;
    let w = 220.0;
    let h = item_h * g.tools.len() as f32 + 16.0;
    let bottom = ui.ctx().content_rect().max.y - 8.0;
    let at = pos2(top_left.x, top_left.y.min(bottom - h));
    let mut chosen = None;
    egui::Area::new(flyout_id(g.key)).fixed_pos(at).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(10), t.menu);
        let mut y = rect.min.y + 8.0;
        for (tid, icon) in g.tools {
            let r = Rect::from_min_size(pos2(rect.min.x + 8.0, y), vec2(w - 16.0, item_h));
            let resp = ui.interact(r, ui.id().with(("fly", tid.id())), Sense::click());
            a11y::menu_item(&resp, tr(tid.label()), tid.shortcut(), Some(*tid == app.session.tool.tool));
            // The keyboard's item is highlighted like the hovered one.
            a11y::own_focus(&resp);
            if resp.hovered() || resp.has_focus() {
                ui.painter().rect_filled(r, CornerRadius::same(6), t.menu_hover);
            }
            if *tid == app.session.tool.tool {
                icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 10.0, r.center().y), vec2(12.0, 12.0)), Icon::Check, t.menu_text);
            }
            icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 32.0, r.center().y), vec2(16.0, 16.0)), *icon, t.menu_text);
            ui.painter().text(pos2(r.min.x + 48.0, r.center().y), Align2::LEFT_CENTER, tr(tid.label()), regular(12.0), t.menu_text);
            ui.painter().text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, pretty_shortcut(tid.shortcut()), regular(12.0), t.menu_dim);
            if resp.clicked() {
                chosen = Some(*tid);
            }
            y += item_h;
        }
    });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.tool_flyout = None;
    }
    if let Some(tid) = chosen {
        if let Some(i) = ToolId::ALL.iter().position(|x| *x == tid) {
            ui.data_mut(|d| d.insert_temp(egui::Id::new(("tb_last", g.key)), i as u8));
        }
        app.run("tool.set", json!({"tool": tid.id()}));
        app.ui.tool_flyout = None;
    }
}

/// A tool group's flyout (an egui area).
fn flyout_id(key: &str) -> egui::Id {
    egui::Id::new(("flyout_area", key))
}

/// "Shift+Cmd+K" → "⇧⌘K" on macOS, "Ctrl+Shift+K" elsewhere.
pub fn pretty_shortcut(s: &str) -> String {
    figcraft_engine::keymap::display(s, crate::shortcuts::platform())
}
