//! The quick actions floating over the selection on the canvas: auto layout, alignment, create
//! component and the rest of the context menu. Each runs the same command as its menu item.

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use figcraft_engine::ToolId;
use figcraft_engine::tools::selection_box;
use serde_json::json;

use crate::a11y::{self, Tip};
use crate::i18n::{tr, tr_noop};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, semibold};
use crate::{FigApp, RightTab};

const H: f32 = 40.0;
const CELL: f32 = 30.0;
/// Between the bar and the selection (room for a frame's name label above it).
const GAP: f32 = 30.0;
/// The tool rail along the canvas's left edge, which the bar stays clear of.
const RAIL_CLEAR: f32 = 74.0;

enum Item {
    Wide(Icon, &'static str, &'static str),
    Cell(Icon, &'static str, &'static str),
    /// An alignment button: shown only when the selection has something to align to.
    Align(Icon, &'static str, &'static str),
    More,
    Divider,
}

const ITEMS: &[Item] = &[
    Item::Wide(Icon::AutoLayout, tr_noop("Auto layout"), "layout.add"),
    Item::Divider,
    Item::Align(Icon::AlignLeft, tr_noop("Align left"), "align.left"),
    Item::Align(Icon::AlignHCenter, tr_noop("Align horizontal centers"), "align.hcenter"),
    Item::Align(Icon::AlignRight, tr_noop("Align right"), "align.right"),
    Item::Divider,
    Item::Cell(Icon::Component, tr_noop("Create component"), "component.create"),
    Item::More,
];

/// The items shown: without the alignment buttons (and a divider left doubled) when there's
/// nothing to align to.
fn items(can_align: bool) -> Vec<&'static Item> {
    let mut out: Vec<&'static Item> = Vec::new();
    for it in ITEMS {
        match it {
            Item::Align(..) if !can_align => {}
            Item::Divider if matches!(out.last(), Some(Item::Divider)) => {}
            _ => out.push(it),
        }
    }
    out
}

/// Whether the bar shows now: the Move tool in Design mode on an editable file, a selection, no
/// text or vector editing, nothing pressed (it gets out of the way of drags) and no menu open.
pub fn shown(app: &FigApp, pressed: bool) -> bool {
    let Ok(ds) = app.session.ds() else { return false };
    app.ui.right_tab == RightTab::Design
        && app.session.tool.tool == ToolId::Move
        && app.session.active_read_only().is_none()
        && !ds.selection.is_empty()
        && ds.text_edit.is_none()
        && ds.vector_edit.is_none()
        && !ds.view.ui_hidden
        && !pressed
        && app.ui.context_menu.is_none()
        && app.ui.palette.is_none()
}

fn width(ui: &Ui, items: &[&Item]) -> f32 {
    items.iter().fold(12.0, |w, it| {
        w + match it {
            Item::Wide(_, label, _) => wide_width(ui, label),
            Item::Cell(..) | Item::Align(..) | Item::More => CELL + 2.0,
            Item::Divider => 9.0,
        }
    })
}

fn wide_width(ui: &Ui, label: &str) -> f32 {
    let g = ui.painter().layout_no_wrap(tr(label).to_string(), semibold(11.5), Color32::PLACEHOLDER);
    10.0 + 16.0 + 6.0 + g.size().x + 10.0 + 2.0
}

/// Where the bar goes for a selection whose screen bounds are `sel` on `canvas`: centred above
/// it, else below it (under its size label), else along the canvas's top; always on the canvas
/// and clear of the tool rail. `None` when the selection is off screen.
pub fn place(canvas: Rect, sel: Rect, w: f32) -> Option<Rect> {
    if !sel.is_finite() || !canvas.intersects(sel) {
        return None;
    }
    let (left, right) = (canvas.min.x + RAIL_CLEAR, canvas.max.x - 8.0);
    if right - left < w {
        return None;
    }
    let x = (sel.center().x - w / 2.0).clamp(left, right - w);
    let above = sel.min.y - GAP - H;
    let below = sel.max.y + GAP;
    let y = if above >= canvas.min.y + 8.0 {
        above
    } else if below + H <= canvas.max.y - 8.0 {
        below
    } else {
        canvas.min.y + 8.0
    };
    Some(Rect::from_min_size(pos2(x, y), vec2(w, H)))
}

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let pressed = ui.input(|i| i.pointer.any_down());
    if !shown(app, pressed) {
        return;
    }
    let Ok(ds) = app.session.ds() else { return };
    let Some(b) = selection_box(&ds.doc, &ds.selection) else { return };
    let items = items(figcraft_engine::cmd::arrange::can_align(&ds.doc, &ds.selection));
    let sel = Rect::from_points(&b.map(|p| app.to_screen(p.x, p.y)));
    let Some(rect) = place(app.canvas.rect, sel, width(ui, &items)) else { return };
    let t = Tokens::get(ui.ctx());
    // The bar takes presses over it: the canvas under it would start a marquee.
    ui.interact(rect, ui.id().with("context_bar"), Sense::CLICK | Sense::DRAG);
    a11y::container_bounds(ui, rect);
    let painter = ui.painter();
    for (i, a) in [(6.0, 10u8), (3.0, 16), (1.0, 24)] {
        painter.rect_filled(rect.expand(i).translate(vec2(0.0, i * 0.5)), CornerRadius::same((14.0 + i) as u8), Color32::from_black_alpha(a));
    }
    painter.rect_filled(rect, CornerRadius::same(14), t.panel);
    painter.rect_stroke(rect, CornerRadius::same(14), Stroke::new(1.0, t.border), StrokeKind::Inside);
    let cy = rect.center().y;
    let mut x = rect.min.x + 6.0;
    let mut run = None;
    for it in items {
        match it {
            Item::Divider => {
                painter.line_segment([pos2(x + 4.0, cy - 10.0), pos2(x + 4.0, cy + 10.0)], Stroke::new(1.0, t.border));
                x += 9.0;
            }
            Item::Wide(icon, label, cmd) => {
                let w = wide_width(ui, label) - 2.0;
                let r = Rect::from_min_size(pos2(x, cy - CELL / 2.0), vec2(w, CELL));
                let resp = ui.interact(r, ui.id().with(("ctxbar", *cmd)), Sense::click()).tip(crate::shortcuts::tip(tr(label), cmd));
                ui.painter().rect_filled(r, CornerRadius::same(9), if resp.hovered() { t.selected_row } else { t.accent.gamma_multiply(0.14) });
                icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 18.0, cy), vec2(16.0, 16.0)), *icon, t.accent_text);
                ui.painter().text(pos2(r.min.x + 32.0, cy), Align2::LEFT_CENTER, tr(label), semibold(11.5), t.accent_text);
                if resp.clicked() {
                    run = Some(*cmd);
                }
                x += w + 2.0;
            }
            Item::Cell(icon, label, cmd) | Item::Align(icon, label, cmd) => {
                let r = Rect::from_min_size(pos2(x, cy - CELL / 2.0), vec2(CELL, CELL));
                let resp = ui.interact(r, ui.id().with(("ctxbar", *cmd)), Sense::click()).tip(crate::shortcuts::tip(tr(label), cmd));
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(8), t.hover);
                }
                icons::paint(ui.painter(), r.shrink(7.0), *icon, if *icon == Icon::Component { t.component } else { t.icon });
                if resp.clicked() {
                    run = Some(*cmd);
                }
                x += CELL + 2.0;
            }
            Item::More => {
                let r = Rect::from_min_size(pos2(x, cy - CELL / 2.0), vec2(CELL, CELL));
                let resp = ui.interact(r, ui.id().with("ctxbar_more"), Sense::click()).tip(tr("More actions"));
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(8), t.hover);
                }
                icons::paint(ui.painter(), r.shrink(7.0), Icon::Dots, t.icon);
                if resp.clicked() {
                    app.ui.context_menu = Some((r.left_bottom() + vec2(0.0, 6.0), app.session.selection()));
                }
                x += CELL + 2.0;
            }
        }
    }
    if let Some(cmd) = run {
        app.run(cmd, json!({}));
    }
}
