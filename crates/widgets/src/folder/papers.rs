//! The newest files, standing in the folder as papers: lifted when the
//! pointer is over it or files are coming in, and spread out over it like a
//! hand of cards when it is open.

use std::rc::Rc;

use crownui::prelude::{
    AnyView, Color, CursorIcon, Listeners, Role, Runtime, Semantic, Shadow, Signal, Spring, Styled,
    Vector, View, image, keyed, rgb, spacer, text, vstack,
};

use super::{Look, Looks, PAPER, State};
use crate::scale::Scale;
use crownhome_services::folder::{self, Entry, Kind};
use crownhome_util::raster::Pixels;

const CORNER: f32 = 14.0;
const FANNED_SCALE: f32 = 0.94;
const DOCUMENT_ICON: f32 = 64.0;
const DOCUMENT: u32 = 0xFBFBFC;
const WAITING: u32 = 0xE4E4E8;
const DOCUMENT_INK: u32 = 0x1C1C1E;

/// Where a paper stands, as an offset from its place in design points, a
/// turn in degrees and a scale.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pose {
    offset: (f32, f32),
    turn: f32,
    scale: f32,
}

const fn pose(x: f32, y: f32, turn: f32, scale: f32) -> Pose {
    Pose {
        offset: (x, y),
        turn,
        scale,
    }
}

/// Resting, lifted and spread out, for the paper `index` of `count`, the newest first:
/// it ends up in front and to the right, the older ones fanning off to the left.
fn poses(index: usize, count: usize) -> [Pose; 3] {
    const RESTING: [Pose; 3] = [
        pose(0.0, 0.0, 0.0, 1.0),
        pose(-7.0, 5.0, -4.0, 1.0),
        pose(8.0, 8.0, 4.0, 1.0),
    ];
    const LIFTED: [Pose; 3] = [
        pose(0.0, -14.0, 0.0, 1.0),
        pose(-13.0, -6.0, -7.0, 1.0),
        pose(14.0, -3.0, 7.0, 1.0),
    ];
    let fanned = match (count, index) {
        (1, _) => pose(4.0, -134.0, 3.0, FANNED_SCALE),
        (2, 0) => pose(40.0, -134.0, 7.0, FANNED_SCALE),
        (2, _) => pose(-40.0, -122.0, -6.0, FANNED_SCALE),
        (_, 0) => pose(64.0, -138.0, 8.0, FANNED_SCALE),
        (_, 1) => pose(2.0, -130.0, -2.0, FANNED_SCALE),
        _ => pose(-66.0, -116.0, -9.0, FANNED_SCALE),
    };
    let slot = index.min(2);
    [RESTING[slot], LIFTED[slot], fanned]
}

/// What kind of file `entry` is, in a word, for when the theme has no picture of it.
fn kind_name(entry: &Entry) -> String {
    match entry.kind {
        Kind::Folder => "Folder".to_owned(),
        _ => entry.path.extension().map_or_else(
            || "File".to_owned(),
            |extension| extension.to_string_lossy().to_uppercase(),
        ),
    }
}

/// A file that is not a photo: the theme's picture of its type, or its type
/// in a word, over its name.
fn document(entry: &Entry, icon: Option<&Rc<Pixels>>, scale: Scale) -> AnyView {
    let ink = rgb(DOCUMENT_INK);
    let name = text(entry.name.clone())
        .text_size(scale.px(11.0))
        .line_clamp(2)
        .text_color(ink.with_alpha_factor(0.55));
    let paper = match icon {
        Some(icon) => vstack((
            spacer(),
            image(icon.edge, icon.edge, icon.rgba.clone())
                .w(scale.px(DOCUMENT_ICON))
                .h(scale.px(DOCUMENT_ICON)),
            spacer(),
            name.text_center(),
        ))
        .items_center()
        .boxed(),
        None => vstack((
            text(kind_name(entry))
                .text_size(scale.px(22.0))
                .font_weight(700)
                .tracking(scale.px(-0.4))
                .text_color(ink),
            spacer(),
            name,
        ))
        .boxed(),
    };
    vstack(paper)
        .w_full()
        .h_full()
        .p(scale.px(12.0))
        .bg(rgb(DOCUMENT))
        .boxed()
}

fn face(entry: &Entry, look: Look, scale: Scale) -> AnyView {
    match look {
        Look::Photo(print) => image(
            print.picture.width,
            print.picture.height,
            print.picture.rgba.clone(),
        )
        .w_full()
        .h_full()
        .boxed(),
        Look::Pending if entry.kind == Kind::Photo => {
            vstack(()).w_full().h_full().bg(rgb(WAITING)).boxed()
        }
        Look::Icon(icon) => document(entry, Some(&icon), scale),
        Look::Pending | Look::Missing => document(entry, None, scale),
    }
}

fn paper(
    index: usize,
    count: usize,
    entry: Entry,
    looks: Signal<Looks>,
    state: State,
    scale: Scale,
) -> impl View {
    let [resting, lifted, fanned] = poses(index, count);
    let factor = scale.factor();
    let now = move |runtime: &mut Runtime| {
        if state.opened.get(runtime) {
            fanned
        } else if state.hovered.get(runtime) || state.receiving(runtime) {
            lifted
        } else {
            resting
        }
    };
    let shown = entry.clone();
    let wanted = entry.path.clone();
    let face = keyed(
        move |runtime| {
            looks.with(runtime, |looks| {
                looks
                    .get(index)
                    .filter(|(path, _)| *path == wanted)
                    .map_or(Look::Pending, |(_, look)| look.clone())
            })
        },
        move |_, look| face(&shown, look, scale),
    )
    .w_full()
    .h_full();
    let path = entry.path.clone();
    vstack(face)
        .absolute()
        .left(scale.px(PAPER.0))
        .top(scale.px(PAPER.1))
        .w(scale.px(PAPER.2))
        .h(scale.px(PAPER.3))
        .rounded(scale.px(CORNER))
        .overflow_hidden()
        .border(1.0)
        .border_color(Color::new(1.0, 1.0, 1.0, 0.6))
        .shadow(Shadow::new(
            Color::new(0.0, 0.0, 0.0, 0.16),
            14.0,
            Vector::new(0.0, 6.0),
        ))
        .translate(crownui::prelude::bind(move |runtime| {
            let (x, y) = now(runtime).offset;
            Vector::new(x, y) * factor.get(runtime)
        }))
        .rotate(crownui::prelude::bind(move |runtime| now(runtime).turn))
        .scale(crownui::prelude::bind(move |runtime| now(runtime).scale))
        .transition(Spring::snappy())
        .pointer_events_auto()
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .label(entry.name)
        .on_click(move |cx, event| {
            event.stop_propagation();
            if state.opened.get(cx) {
                folder::open(&path);
            } else {
                state.opened.set(cx, true);
            }
        })
}

/// The papers, newest on top.
pub fn view(
    newest: &[Entry],
    looks: Signal<Looks>,
    state: State,
    scale: Scale,
) -> impl View + use<> {
    let count = newest.len();
    let papers: Vec<_> = newest
        .iter()
        .enumerate()
        .rev()
        .map(|(index, entry)| paper(index, count, entry.clone(), looks, state, scale))
        .collect();
    vstack(papers).absolute().inset(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_paper_leads_the_hand_on_the_right() {
        let [newest, middle, oldest] = [0, 1, 2].map(|index| poses(index, 3)[2]);
        assert!(oldest.offset.0 < middle.offset.0 && middle.offset.0 < newest.offset.0);
        assert!(oldest.turn < 0.0 && newest.turn > 0.0);
        assert!(newest.offset.1 < oldest.offset.1);
    }

    #[test]
    fn spread_papers_clear_the_front() {
        for count in 1..=3 {
            for index in 0..count {
                let [_, _, fanned] = poses(index, count);
                let half_height = PAPER.3 * fanned.scale / 2.0;
                let bottom = PAPER.1 + PAPER.3 / 2.0 + fanned.offset.1 + half_height;
                assert!(bottom < super::super::FRONT.1, "{index} of {count}");
            }
        }
    }
}
