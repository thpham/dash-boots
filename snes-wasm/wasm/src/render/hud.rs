use super::{RenderList, CYAN, WHITE, YELLOW};
use crate::game::{Game, Phase};

pub fn draw(g: &Game, r: &mut RenderList) {
    r.clear();
    let (vw, vh) = (r.view_w, r.view_h);
    match g.phase {
        Phase::Title => draw_title(g, r),
        Phase::Won => {
            draw_hud(g, r);
            draw_clear(g, r);
        }
        _ => draw_hud(g, r),
    }
    if g.paused {
        r.rect(0.0, 0.0, vw, vh, [0.0, 0.0, 0.05, 0.55]);
        r.text_centered(vh * 0.5 - 12.0, "PAUSED", WHITE, 2.0);
        r.text_centered(vh * 0.5 + 10.0, "PRESS ENTER TO RESUME", CYAN, 1.0);
    }
}

fn format_time(t: f32) -> String {
    let s = t.max(0.0) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn draw_hud(g: &Game, r: &mut RenderList) {
    let coins = format!("$ {:02}/{:02}", g.coins_collected, g.coins_total);
    r.text_shadow(6.0, 6.0, &coins, WHITE, 1.0);
    if g.player.dash_boots {
        r.text_shadow(6.0 + RenderList::text_width(&coins, 1.0) + 6.0, 6.0, "^", WHITE, 1.0);
    }
    let deaths = format!("# {}", g.deaths);
    let dx = ((r.view_w - RenderList::text_width(&deaths, 1.0)) * 0.5).round();
    r.text_shadow(dx, 6.0, &deaths, WHITE, 1.0);
    let time = format!("TIME {}", format_time(g.time));
    let tx = r.view_w - RenderList::text_width(&time, 1.0) - 6.0;
    r.text_shadow(tx, 6.0, &time, WHITE, 1.0);
}

fn draw_title(g: &Game, r: &mut RenderList) {
    let (vw, vh) = (r.view_w, r.view_h);
    r.rect(0.0, 0.0, vw, vh, [0.02, 0.0, 0.08, 0.35]);
    r.text_centered(26.0, "DASH BOOTS", YELLOW, 3.0);
    r.text_centered(54.0, "A 16-BIT PLATFORM ADVENTURE", CYAN, 1.0);
    if (g.frame / 30).is_multiple_of(2) {
        r.text_centered(84.0, "PRESS ENTER OR SPACE", WHITE, 1.0);
    }
    let panel_y = 106.0;
    r.rect(24.0, panel_y - 4.0, vw - 48.0, 66.0, [0.0, 0.0, 0.0, 0.45]);
    let lines = [
        "ARROWS/WASD   MOVE",
        "SPACE/K/Z     JUMP (HOLD = HIGHER)",
        "SHIFT/J/X     RUN  (+ DASH W/ BOOTS)",
        "DOWN/S        CROUCH / SLIDE",
        "ENTER  PAUSE   M  MUTE   C  CRT",
    ];
    for (i, l) in lines.iter().enumerate() {
        r.text_shadow(34.0, panel_y + 2.0 + i as f32 * 12.0, l, WHITE, 1.0);
    }
}

fn draw_clear(g: &Game, r: &mut RenderList) {
    let (vw, vh) = (r.view_w, r.view_h);
    let t = (g.phase_time / 0.4).min(1.0);
    let h = (96.0 * t).round();
    r.rect(40.0, (vh - h) * 0.5, vw - 80.0, h, [0.0, 0.0, 0.08, 0.7]);
    if t < 1.0 {
        return;
    }
    let top = (vh - 96.0) * 0.5;
    r.text_centered(top + 8.0, "COURSE CLEAR!", YELLOW, 2.0);
    r.text_centered(
        top + 34.0,
        &format!("COINS   {:02}/{:02}", g.coins_collected, g.coins_total),
        WHITE,
        1.0,
    );
    r.text_centered(top + 46.0, &format!("TIME    {}", format_time(g.time)), WHITE, 1.0);
    r.text_centered(top + 58.0, &format!("DEATHS  {:5}", g.deaths), WHITE, 1.0);
    if g.phase_time > 1.2 && (g.frame / 30).is_multiple_of(2) {
        r.text_centered(top + 78.0, "PRESS ENTER TO PLAY AGAIN", CYAN, 1.0);
    }
}
