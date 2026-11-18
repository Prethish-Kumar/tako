//! 8×8 pixel icons for projects (port of the sidebar ICONS table).

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

type Icon = (&'static str, [&'static str; 8], &'static [(char, u32)]);

pub const ICONS: &[Icon] = &[
    ("octo", ["..bbbb..", ".bhbbbb.", ".bbbbbb.", ".wkbbwk.", ".bbbbbb.", "..bbbb..", ".b.bb.b.", "b..b..b."], &[('b', 0x3f72e6), ('h', 0xa8c8ff), ('w', 0xffffff), ('k', 0x0a1430)]),
    ("rocket", ["...rr...", "..rwwr..", "..rwbr..", "..rwwr..", ".rrwwrr.", ".r.ww.r.", "...oo...", "...yy..."], &[('r', 0xe5675f), ('w', 0xf2efe9), ('b', 0x7aa7e8), ('o', 0xf08a3a), ('y', 0xffd27a)]),
    ("cat", [".k....k.", ".kk..kk.", ".kkkkkk.", ".kgkkgk.", ".kkkpkk.", ".kkkkkk.", "..kkkk..", "........"], &[('k', 0xe2995a), ('g', 0x2a2a2a), ('p', 0xf294b8)]),
    ("ghost", ["..wwww..", ".wwwwww.", ".wbwwbw.", ".wwwwww.", ".wwwwww.", ".wwwwww.", ".w.ww.w.", "........"], &[('w', 0xe9e6f2), ('b', 0x2a2540)]),
    ("mushroom", ["..rrrr..", ".rwrrwr.", "rrrrrrrr", "rwrrrrwr", "..wwww..", "..wbwb..", "..wwww..", "........"], &[('r', 0xe5675f), ('w', 0xf4efe6), ('b', 0x3a2a2a)]),
    ("heart", ["........", ".rr..rr.", "rrrrrrrr", "rwrrrrrr", ".rrrrrr.", "..rrrr..", "...rr...", "........"], &[('r', 0xec5f7a), ('w', 0xffd0da)]),
    ("star", ["...yy...", "...yy...", "yyyyyyyy", ".yyyyyy.", "..yyyy..", ".yy..yy.", ".y....y.", "........"], &[('y', 0xf2c94c)]),
    ("sword", [".......w", "......ww", ".....ww.", ".b..ww..", "..bww...", "..bb....", ".b..b...", "b......."], &[('w', 0xcfd8e3), ('b', 0xb07a40)]),
    ("potion", ["...cc...", "...ww...", "..w..w..", ".wppppw.", ".wpwppw.", ".wppppw.", "..wwww..", "........"], &[('w', 0xcfd8e3), ('c', 0xb07a40), ('p', 0x9b6bf5)]),
    ("planet", ["........", "..pppp..", ".pppppp.", "yyyyyyyy", ".pppppp.", "..pppp..", "........", "........"], &[('p', 0x4fb3c8), ('y', 0xf2c94c)]),
    ("tree", ["...gg...", "..gggg..", ".gggggg.", "..gggg..", ".gggggg.", "gggggggg", "...bb...", "...bb..."], &[('g', 0x5fae5a), ('b', 0x8a5a3a)]),
    ("flame", ["...o....", "...oo...", "..oyo...", "..oyyo..", ".oyyyyo.", ".oyrryo.", "..oyyo..", "........"], &[('o', 0xf08a3a), ('y', 0xffd27a), ('r', 0xe5675f)]),
    ("key", ["........", ".yyy....", "y...y...", "y...yyyy", "y...y.y.", ".yyy..y.", "........", "........"], &[('y', 0xf2c94c)]),
    ("crown", ["........", "y..yy..y", "yy.yy.yy", "yyyyyyyy", "yryyyyby", "yyyyyyyy", "........", "........"], &[('y', 0xf2c94c), ('r', 0xe5675f), ('b', 0x7aa7e8)]),
    ("gem", ["........", ".bbbbbb.", "bwbbbbbb", "bbbbbbbb", ".bbbbbb.", "..bbbb..", "...bb...", "........"], &[('b', 0x4fb3c8), ('w', 0xe6fbff)]),
    ("robot", ["...r....", ".gggggg.", ".gwggwg.", ".gggggg.", ".gbbbbg.", ".gggggg.", "..g..g..", "........"], &[('g', 0x9aa3b0), ('w', 0x7af0c8), ('b', 0x3a3f48), ('r', 0xe5675f)]),
    ("leaf", ["......gg", "....gggg", "..ggggg.", ".gggggg.", ".ggggg..", "gg.gg...", "g.......", "........"], &[('g', 0x7cc56a)]),
];

pub fn icon_ids() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|i| i.0)
}

/// Paint an 8×8 pixel icon into a square of `size` px.
pub fn pix(id: &str, size: f32) -> impl IntoElement {
    let ic = ICONS.iter().find(|i| i.0 == id).unwrap_or(&ICONS[1]);
    let (rows, pal) = (ic.1, ic.2);
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let cell = b.size.width / 8.;
            for (y, row) in rows.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if let Some((_, col)) = pal.iter().find(|(c, _)| *c == ch) {
                        let o = point(b.origin.x + cell * x as f32, b.origin.y + cell * y as f32);
                        window.paint_quad(fill(Bounds::new(o, gpui_kit::size(cell, cell)), rgb(*col)));
                    }
                }
            }
        },
    )
    .size(px(size))
    .flex_none()
}
