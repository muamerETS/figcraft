//! The quick actions bar over the selection (where it goes, when it shows) and the inspector's
//! section cards.

use egui::{Pos2, RawInput, Rect, Shape, pos2, vec2};
use figcraft_engine::Session;
use serde_json::json;

use crate::context_bar::{place, shown};
use crate::layers_drag_tests::Ui;
use crate::theme::Tokens;
use crate::{RightTab, widgets};

const CANVAS: Rect = Rect { min: Pos2 { x: 300.0, y: 40.0 }, max: Pos2 { x: 1200.0, y: 900.0 } };

#[test]
fn the_bar_sits_above_the_selection_centred_on_it() {
    let sel = Rect::from_min_size(pos2(600.0, 400.0), vec2(200.0, 100.0));
    let bar = place(CANVAS, sel, 240.0).unwrap();
    assert_eq!(bar.center().x, sel.center().x);
    assert!(bar.max.y < sel.min.y - 20.0, "room for a frame's name label: {bar:?}");
}

#[test]
fn without_room_above_it_goes_below_then_along_the_top() {
    let near_top = Rect::from_min_size(pos2(600.0, 60.0), vec2(200.0, 100.0));
    let bar = place(CANVAS, near_top, 240.0).unwrap();
    assert!(bar.min.y > near_top.max.y, "below: {bar:?}");
    let tall = Rect::from_min_max(pos2(600.0, 0.0), pos2(800.0, 1000.0));
    let bar = place(CANVAS, tall, 240.0).unwrap();
    assert_eq!(bar.min.y, CANVAS.min.y + 8.0, "along the top of the canvas");
}

#[test]
fn it_stays_on_the_canvas_and_clear_of_the_tool_rail() {
    let at_left = Rect::from_min_size(pos2(290.0, 400.0), vec2(20.0, 20.0));
    let bar = place(CANVAS, at_left, 240.0).unwrap();
    assert!(bar.min.x >= CANVAS.min.x + 70.0, "{bar:?}");
    let at_right = Rect::from_min_size(pos2(1190.0, 400.0), vec2(20.0, 20.0));
    let bar = place(CANVAS, at_right, 240.0).unwrap();
    assert!(bar.max.x <= CANVAS.max.x, "{bar:?}");
}

#[test]
fn nothing_for_a_selection_off_screen_or_a_canvas_too_narrow() {
    let off = Rect::from_min_size(pos2(5000.0, 400.0), vec2(20.0, 20.0));
    assert_eq!(place(CANVAS, off, 240.0), None);
    let nan = Rect::from_min_size(pos2(f32::NAN, 400.0), vec2(20.0, 20.0));
    assert_eq!(place(CANVAS, nan, 240.0), None);
    let sel = Rect::from_min_size(pos2(600.0, 400.0), vec2(20.0, 20.0));
    assert_eq!(place(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 600.0)), sel, 240.0), None);
}

#[test]
fn it_shows_for_a_selection_with_the_move_tool_in_design_mode() {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 300, "height": 200})).unwrap();
    let mut u = Ui::new(s);
    u.frame(vec![]);
    assert!(shown(&u.app, false));
    assert!(!shown(&u.app, true), "out of the way while a button is down (a drag)");
    u.app.ui.right_tab = RightTab::Prototype;
    assert!(!shown(&u.app, false), "Prototype mode");
    u.app.ui.right_tab = RightTab::Design;
    u.app.run("tool.set", json!({"tool": "rectangle"}));
    assert!(!shown(&u.app, false), "another tool");
    u.app.run("tool.set", json!({"tool": "move"}));
    u.app.run("select.set", json!({"ids": []}));
    assert!(!shown(&u.app, false), "nothing selected");
}

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
