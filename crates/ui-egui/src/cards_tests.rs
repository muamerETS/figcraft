//! The inspector's section cards.

use egui::{RawInput, Rect, Shape, pos2, vec2};

use crate::theme::Tokens;
use crate::widgets;

#[test]
fn separators_inside_cards_split_them_and_draw_rules_outside() {
    let ctx = egui::Context::default();
    let t = Tokens::dark();
    crate::theme::apply(&ctx, t);
    let input = RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0))), ..Default::default() };
    let mut out = ctx.run_ui(input, |ui| {
        widgets::cards(ui, |ui| {
            ui.label("Fill");
            widgets::separator(ui);
            ui.label("Stroke");
            widgets::separator(ui);
            widgets::separator(ui);
        });
        widgets::separator(ui);
    });
    out.textures_delta.clear();
    let cards = out.shapes.iter().filter(|s| matches!(&s.shape, Shape::Rect(r) if r.fill == t.card)).count();
    let rules = out.shapes.iter().filter(|s| matches!(&s.shape, Shape::LineSegment { .. })).count();
    assert_eq!(cards, 2, "one card per section; the empty one between the last separators isn't drawn");
    assert_eq!(rules, 1, "outside the cards a separator is a rule");
}
