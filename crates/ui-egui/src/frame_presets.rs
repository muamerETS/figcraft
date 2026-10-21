//! The Frame tool's panel: frame presets by category. Clicking one adds a frame of that size next
//! to the page's frames and returns to the Move tool.

use egui::{Align2, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, INSET, PAD};

pub fn panel(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Frame"), &[], false);
    let w = ui.available_width();
    let mut cats: Vec<&str> = Vec::new();
    for (c, ..) in figcraft_engine::cmd::create::FRAME_PRESETS {
        if !cats.contains(c) {
            cats.push(c);
        }
    }
    let mut chosen = None;
    for cat in cats {
        widgets::separator(ui);
        let open_id = Id::new(("preset_cat", cat));
        // The first category starts open.
        let open: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(cat == "Phone");
        let presets: Vec<_> = figcraft_engine::cmd::create::FRAME_PRESETS.iter().filter(|(c, ..)| *c == cat).collect();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
        let row = r.shrink2(vec2(INSET + 4.0, 4.0));
        if resp.hovered() {
            ui.painter().rect_filled(row, CornerRadius::same(6), t.hover);
        }
        let title = ui.painter().layout_no_wrap(tr(cat).to_string(), semibold(11.5), t.text);
        let title_w = title.size().x;
        ui.painter().galley(pos2(r.min.x + PAD, r.center().y - title.size().y / 2.0), title, t.text);
        ui.painter().text(
            pos2(r.min.x + PAD + title_w + 6.0, r.center().y),
            Align2::LEFT_CENTER,
            presets.len().to_string(),
            regular(11.0),
            t.text_secondary,
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(r.max.x - PAD - 4.0, r.center().y), vec2(10.0, 10.0)),
            if open { Icon::Chevron } else { Icon::ChevronRight },
            t.icon_secondary,
        );
        if resp.clicked() {
            ui.data_mut(|d| d.insert_temp(open_id, !open));
        }
        if !open {
            continue;
        }
        for (_, name, pw, ph) in presets {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r.shrink2(vec2(INSET + 4.0, 1.0)), CornerRadius::same(6), t.hover);
            }
            device(ui, pos2(r.min.x + PAD + 7.0, r.center().y), *pw, *ph, if resp.hovered() { t.accent } else { t.icon_secondary });
            ui.painter().text(pos2(r.min.x + PAD + 22.0, r.center().y), Align2::LEFT_CENTER, *name, regular(11.0), t.text);
            ui.painter().text(
                pos2(r.max.x - PAD, r.center().y),
                Align2::RIGHT_CENTER,
                format!("{} × {}", crate::canvas::fmt(*pw), crate::canvas::fmt(*ph)),
                regular(10.5),
                t.text_secondary,
            );
            if resp.clicked() {
                chosen = Some(*name);
            }
        }
    }
    if let Some(name) = chosen {
        app.run("frame.create", json!({"preset": name}));
        app.run("tool.set", json!({"tool": "move"}));
        // Show the new frame.
        app.run("view.zoomSelection", json!({}));
    }
}

/// A preset's outline at `center`, its longer side 14 px, in its own proportions.
fn device(ui: &Ui, center: egui::Pos2, w: f64, h: f64, color: egui::Color32) {
    let long = w.max(h).max(1.0);
    let size = vec2((14.0 * w / long).max(4.0) as f32, (14.0 * h / long).max(4.0) as f32);
    let rect = Rect::from_center_size(center, size);
    ui.painter().rect_stroke(rect, CornerRadius::same(2), egui::Stroke::new(1.3, color), egui::StrokeKind::Inside);
}
