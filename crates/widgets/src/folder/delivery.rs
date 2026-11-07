//! Files arriving: their names hang over the folder, drop into it, and it
//! lights up from inside.

use std::{path::PathBuf, rc::Rc, time::Duration};

use crownui::{
    ext::{Brush, DrawCommand, GradientStop, Paint},
    prelude::{
        Color, Curve, Cx, Memo, Point, Shadow, Signal, Size, Spring, Styled, Vector, View, bind,
        canvas, hstack, image, keyed, text, vstack,
    },
};

use super::{
    DESIGN, FRONT, State, art_pixels,
    front::{self, Piece, TAB_HEIGHT, Tone},
    refresh, shown_when,
};
use crate::{art, scale::Scale};
use crownhome_services::folder::{self, Listing};
use crownhome_util::raster::Pixels;

/// What a drag carries when it carries files.
pub const URI_LIST: &str = "text/uri-list";
/// How long names hang over the folder before dropping in: long enough to be
/// read after the file chooser closes, a moment after a drop.
pub const PICKED_HOVER: Duration = Duration::from_millis(700);
pub const DROPPED_HOVER: Duration = Duration::from_millis(160);
const SINKING: Duration = Duration::from_millis(360);
const GLOWING: Duration = Duration::from_millis(1100);
const GLOW_FADE: u64 = 520;

const GLOW_ART: &[u8] = include_bytes!("../../assets/folder/glow.svg");
/// The light's frame, and how far down it the folder's mouth lies.
const GLOW: (f32, f32) = (250.0, 150.0);
const GLOW_MOUTH: f32 = 200.0 / 240.0;

const SHOWN_NAMES: usize = 3;
const CHIP_TOP: f32 = 30.0;
const CHIP_HEIGHT: f32 = 34.0;
const CHIP_MAX_WIDTH: f32 = 172.0;
const CHIP_ICON: f32 = 18.0;
const CHIP_CORNER: f32 = 10.0;
/// Where a name comes in from, and how far it drops to be out of sight in the folder.
const CHIP_ENTRY: (f32, f32, f32) = (70.0, -70.0, 14.0);
const CHIP_SINK: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Hovering,
    Sinking,
    Glowing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Delivery {
    serial: u64,
    names: Vec<String>,
    pub stage: Stage,
}

impl Delivery {
    /// Whether the folder is still taking the files in, held open for them.
    pub fn receiving(&self) -> bool {
        self.stage != Stage::Glowing
    }
}

fn after(cx: &mut Cx, wait: Duration, then: impl FnOnce(&mut Cx) + 'static) {
    cx.spawn(
        async move { tokio::time::sleep(wait).await },
        move |cx, ()| {
            then(cx);
        },
    );
}

/// Moves delivery `serial` on to `stage`, or ends it; false when a newer one replaced it.
fn advance(
    cx: &mut Cx,
    delivery: Signal<Option<Delivery>>,
    serial: u64,
    stage: Option<Stage>,
) -> bool {
    let current = delivery.with(cx, |delivery| {
        delivery
            .as_ref()
            .is_some_and(|delivery| delivery.serial == serial)
    });
    if current {
        match stage {
            Some(stage) => delivery.update(cx, |delivery| {
                if let Some(delivery) = delivery {
                    delivery.stage = stage;
                }
            }),
            None => delivery.set(cx, None),
        }
    }
    current
}

/// Plays `names` arriving in `dir`, reading it again as they land so they show among the papers.
pub fn receive(
    cx: &mut Cx,
    state: State,
    dir: PathBuf,
    listing: Signal<Option<Listing>>,
    names: Vec<String>,
    hover: Duration,
) {
    if names.is_empty() {
        return;
    }
    let delivery = state.delivery;
    let serial = delivery.with(cx, |delivery| {
        delivery.as_ref().map_or(0, |delivery| delivery.serial + 1)
    });
    if state.opened.get(cx) {
        state.opened.set(cx, false);
    }
    delivery.set(
        cx,
        Some(Delivery {
            serial,
            names,
            stage: Stage::Hovering,
        }),
    );
    after(cx, hover, move |cx| {
        if !advance(cx, delivery, serial, Some(Stage::Sinking)) {
            return;
        }
        after(cx, SINKING, move |cx| {
            if !advance(cx, delivery, serial, Some(Stage::Glowing)) {
                return;
            }
            refresh(cx, dir, listing);
            after(cx, GLOWING, move |cx| {
                advance(cx, delivery, serial, None);
            });
        });
    });
}

fn chip_fill(tone: Tone) -> Color {
    match tone {
        Tone::Light => Color::new(1.0, 1.0, 1.0, 0.9),
        Tone::Dark => Color::new(0.52, 0.53, 0.57, 0.92),
    }
}

/// Where name `index` hangs over the folder at `stage`: an offset, a turn and a scale.
fn chip_pose(index: usize, stage: Stage) -> (Vector, f32, f32) {
    let step = index as f32;
    match stage {
        Stage::Hovering => (
            Vector::new(18.0 - 16.0 * step, -9.0 * step),
            4.0 - 4.0 * step,
            1.0,
        ),
        Stage::Sinking | Stage::Glowing => (Vector::new(0.0, CHIP_SINK), 0.0, 0.92),
    }
}

fn chip(
    cx: &mut Cx,
    index: usize,
    name: String,
    dir: &std::path::Path,
    delivery: Signal<Option<Delivery>>,
    tone: Memo<Tone>,
    scale: Scale,
) -> impl View + use<> {
    let glyph = cx.signal(None::<Rc<Pixels>>);
    let edge = art_pixels(cx, CHIP_ICON, scale);
    cx.spawn(
        folder::icon(dir.join(&name), false, edge),
        move |cx, icon| {
            glyph.set(cx, icon.map(Rc::new));
        },
    );
    let icon = keyed(
        move |runtime| glyph.get(runtime),
        move |_, icon| match icon {
            Some(icon) => image(icon.edge, icon.edge, icon.rgba.clone())
                .w_full()
                .h_full()
                .boxed(),
            None => vstack(()).boxed(),
        },
    )
    .w(scale.px(CHIP_ICON))
    .h(scale.px(CHIP_ICON))
    .shrink(0.0);
    let label = text(name)
        .text_size(scale.px(12.5))
        .font_weight(500)
        .line_clamp(1)
        .shrink(1.0)
        .min_w(0.0)
        .text_color(bind(move |runtime| {
            tone.get(runtime).ink().with_alpha_factor(0.8)
        }));
    let pill = hstack((icon, label))
        .items_center()
        .gap(scale.px(7.0))
        .px(scale.px(10.0))
        .h(scale.px(CHIP_HEIGHT))
        .max_w(scale.px(CHIP_MAX_WIDTH))
        .rounded(scale.px(CHIP_CORNER))
        .bg(bind(move |runtime| chip_fill(tone.get(runtime))))
        .border(1.0)
        .border_color(Color::new(1.0, 1.0, 1.0, 0.45))
        .shadow(Shadow::new(
            Color::new(0.0, 0.0, 0.0, 0.18),
            16.0,
            Vector::new(0.0, 6.0),
        ));
    let factor = scale.factor();
    let now = move |runtime: &mut crownui::prelude::Runtime| {
        let stage = delivery.with(runtime, |delivery| {
            delivery
                .as_ref()
                .map_or(Stage::Glowing, |delivery| delivery.stage)
        });
        chip_pose(index, stage)
    };
    let (entry_x, entry_y, entry_turn) = CHIP_ENTRY;
    let entry = Vector::new(entry_x, entry_y) * factor.get(cx);
    hstack(pill)
        .absolute()
        .left(0.0)
        .top(scale.px(CHIP_TOP))
        .w(scale.px(DESIGN.0))
        .justify_center()
        .translate(bind(move |runtime| now(runtime).0 * factor.get(runtime)))
        .rotate(bind(move |runtime| now(runtime).1))
        .scale(bind(move |runtime| now(runtime).2))
        .opacity(1.0)
        .enter(move |style| style.translate(entry).rotate(entry_turn).opacity(0.0))
        .transition(Spring::snappy())
}

/// The names of arriving files, over the folder and dropping into it.
pub fn chips(state: State, dir: PathBuf, tone: Memo<Tone>, scale: Scale) -> impl View {
    let delivery = state.delivery;
    keyed(
        move |runtime| {
            delivery.with(runtime, |delivery| {
                delivery
                    .as_ref()
                    .map(|delivery| (delivery.serial, delivery.names.clone()))
            })
        },
        move |cx, arriving| {
            let Some((_, names)) = arriving else {
                return vstack(()).boxed();
            };
            let chips: Vec<_> = names
                .into_iter()
                .take(SHOWN_NAMES)
                .enumerate()
                .rev()
                .map(|(index, name)| chip(cx, index, name, &dir, delivery, tone, scale))
                .collect();
            vstack(chips).absolute().inset(0.0).boxed()
        },
    )
    .absolute()
    .inset(0.0)
}

fn glowing(delivery: Signal<Option<Delivery>>) -> impl Fn(&mut crownui::prelude::Runtime) -> f32 {
    move |runtime| {
        shown_when(delivery.with(runtime, |delivery| {
            delivery
                .as_ref()
                .is_some_and(|delivery| delivery.stage == Stage::Glowing)
        }))
    }
}

/// Light welling out of the folder's mouth, drawn behind its front.
pub fn light_inside(cx: &mut Cx, state: State, scale: Scale) -> impl View + use<> {
    let factor = scale.factor();
    let size = cx.memo(move |runtime| {
        let factor = factor.get(runtime);
        Size::new(GLOW.0 * factor, GLOW.1 * factor)
    });
    vstack(art::backdrop(cx, GLOW_ART, size))
        .absolute()
        .left(scale.px(FRONT.0 + (FRONT.2 - GLOW.0) / 2.0))
        .top(scale.px(FRONT.1 + TAB_HEIGHT - GLOW_MOUTH * GLOW.1))
        .w(scale.px(GLOW.0))
        .h(scale.px(GLOW.1))
        .pointer_events_none()
        .opacity(bind(glowing(state.delivery)))
        .transition(Curve::ease_in_out(GLOW_FADE))
}

/// The same light coming through the front pane, brightest along its top.
pub fn light_through(state: State, tone: Memo<Tone>, scale: Scale) -> impl View + Styled {
    let factor = scale.factor();
    canvas(move |runtime, size, list| {
        let strength = match tone.get(runtime) {
            Tone::Light => 0.8,
            Tone::Dark => 0.6,
        };
        let stops = [(0.0, strength), (0.55, 0.0)]
            .into_iter()
            .map(|(offset, alpha)| GradientStop {
                offset,
                color: Color::new(0.95, 0.96, 1.0, alpha),
            })
            .collect();
        let light = Paint {
            brush: Brush::LinearGradient {
                start: Point::new(0.0, TAB_HEIGHT * factor.get(runtime)),
                end: Point::new(0.0, size.height),
                stops,
            },
            blur: None,
        };
        let outline = front::outline(size, factor.get(runtime), Piece::Whole);
        list.push(DrawCommand::FillPath(outline, light));
    })
    .pointer_events_none()
    .opacity(bind(glowing(state.delivery)))
    .transition(Curve::ease_in_out(GLOW_FADE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_drop_out_of_sight_into_the_folder() {
        let (hanging, _, _) = chip_pose(0, Stage::Hovering);
        let (sunk, _, _) = chip_pose(0, Stage::Sinking);
        assert!(CHIP_TOP + hanging.y + CHIP_HEIGHT < FRONT.1 + TAB_HEIGHT);
        assert!(CHIP_TOP + sunk.y > FRONT.1 + TAB_HEIGHT);
    }

    #[test]
    fn the_folder_is_held_open_until_the_light() {
        let mut delivery = Delivery {
            serial: 0,
            names: vec!["a.pdf".to_owned()],
            stage: Stage::Hovering,
        };
        assert!(delivery.receiving());
        delivery.stage = Stage::Glowing;
        assert!(!delivery.receiving());
    }
}
