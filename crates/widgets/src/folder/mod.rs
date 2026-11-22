//! A folder on disk as a frosted card: its newest files stand in it as
//! papers, rise when the pointer comes over, and spread out over it when it
//! is clicked, its front tipping toward the viewer. Files dropped on it, or
//! added with its button, fall in and light it up.

mod delivery;
mod front;
mod label;
mod papers;
mod tipping;

use std::{path::PathBuf, rc::Rc};

use crownui::prelude::{
    CursorIcon, Cx, Listeners, Memo, Rect, Role, Runtime, Semantic, Signal, Size, Styled, Vector,
    View, bind, canvas, keyed, vstack, zstack,
};
use serde::{Deserialize, Serialize};

use crate::{Built, Env, Shape, art, scale::Scale};
use crownhome_board::WidgetSpec;
use crownhome_services::folder::{self, Entry, Kind, Listing};
use crownhome_util::{photo::Print, raster::Pixels, uri};
use delivery::Delivery;
use front::{Tip, Tone};

pub const KIND: &str = "folder";

/// The widget is drawn in a 2×2 tile, in design points.
const DESIGN: (f32, f32) = (237.5, 241.0);
/// The newest paper at rest: left, top, width and height.
const PAPER: (f32, f32, f32, f32) = (24.0, 76.0, 189.5, 133.0);
/// The front pane: left, top, width and height.
const FRONT: (f32, f32, f32, f32) = (4.0, 98.0, 229.5, 143.0);
/// The back pane, whose tab stands a little higher than the front's.
const BACK: (f32, f32, f32, f32) = (4.0, 75.0, 229.5, 166.0);
/// Where the outline the edit handles hug begins.
const OUTLINE_TOP: f32 = BACK.1;
/// The edge of the theme's picture on a paper.
const ICON: f32 = 64.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The folder, `~/` for the home directory.
    pub path: String,
    /// What the card is called; the folder's own name when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Settings {
    pub fn of(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            name: None,
        }
    }

    fn title(&self, dir: &std::path::Path) -> String {
        self.name.clone().unwrap_or_else(|| {
            dir.file_name().map_or_else(
                || self.path.clone(),
                |name| name.to_string_lossy().into_owned(),
            )
        })
    }
}

/// Whether the pointer or a drag of files is over the folder, whether it is
/// spread open, and the files arriving in it. Closed under the pointer, the
/// papers stay down until the pointer leaves.
#[derive(Clone, Copy)]
pub(crate) struct State {
    pub hovered: Signal<bool>,
    pub dragging: Signal<bool>,
    pub opened: Signal<bool>,
    pub closed_under_pointer: Signal<bool>,
    pub delivery: Signal<Option<Delivery>>,
}

/// How the papers stand in the folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stance {
    Resting,
    Lifted,
    Spread,
}

impl State {
    pub fn stance(self, runtime: &mut Runtime) -> Stance {
        let pointed_at = self.hovered.get(runtime) && !self.closed_under_pointer.get(runtime);
        if self.opened.get(runtime) {
            Stance::Spread
        } else if pointed_at || self.receiving(runtime) {
            Stance::Lifted
        } else {
            Stance::Resting
        }
    }

    pub fn open(self, cx: &mut Cx) {
        self.opened.set(cx, true);
    }

    pub fn close(self, cx: &mut Cx) {
        self.opened.set(cx, false);
        if self.hovered.get(cx) {
            self.closed_under_pointer.set(cx, true);
        }
    }

    fn toggle(self, cx: &mut Cx) {
        if self.opened.get(cx) {
            self.close(cx);
        } else {
            self.open(cx);
        }
    }

    fn point_at(self, cx: &mut Cx, inside: bool) {
        self.hovered.set(cx, inside);
        if !inside {
            self.closed_under_pointer.set(cx, false);
        }
    }

    /// Whether files are on their way in, so the folder holds itself open.
    pub fn receiving(self, runtime: &mut Runtime) -> bool {
        self.dragging.get(runtime)
            || self.delivery.with(runtime, |delivery| {
                delivery.as_ref().is_some_and(Delivery::receiving)
            })
    }
}

/// What a paper shows.
#[derive(Clone, PartialEq)]
pub(crate) enum Look {
    Pending,
    Photo(Rc<Print>),
    /// The icon theme's picture of the file's type.
    Icon(Rc<Pixels>),
    /// Neither a photo nor a picture of its type could be had.
    Missing,
}

/// The pixels a paper's photo and icon are drawn at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sizes {
    photo: (u32, u32),
    icon: u32,
}

impl Look {
    /// Whether this was drawn at `sizes`, so it need not be drawn again.
    fn fits(&self, sizes: Sizes) -> bool {
        match self {
            Self::Pending => false,
            Self::Photo(print) => (print.picture.width, print.picture.height) == sizes.photo,
            Self::Icon(icon) => icon.edge == sizes.icon,
            Self::Missing => true,
        }
    }
}

/// What each of the newest files shows, by its path.
pub(crate) type Looks = Vec<(PathBuf, Look)>;

fn measure(size: Size) -> f32 {
    (size.width / DESIGN.0).min(size.height / DESIGN.1)
}

pub fn shape(size: Size) -> Shape {
    let factor = measure(size);
    let origin = (
        (size.width - DESIGN.0 * factor) / 2.0,
        (size.height - DESIGN.1 * factor) / 2.0,
    );
    Shape {
        bounds: Rect::from_xywh(
            origin.0 + FRONT.0 * factor,
            origin.1 + OUTLINE_TOP * factor,
            FRONT.2 * factor,
            (DESIGN.1 - OUTLINE_TOP) * factor,
        ),
        radius: front::BOTTOM_CORNER * factor,
    }
}

fn sample() -> Listing {
    let entry = |name: &str| Entry {
        path: PathBuf::from(name),
        name: name.to_owned(),
        kind: Kind::File,
    };
    Listing {
        count: 12,
        newest: vec![
            entry("Itinerary.pdf"),
            entry("Notes.txt"),
            entry("Budget.csv"),
        ],
        changed: None,
    }
}

/// `points` design points of the widget in device pixels, as they are now.
pub(crate) fn art_pixels(cx: &mut Cx, points: f32, scale: Scale) -> u32 {
    let (factor, device) = (scale.factor().get(cx), cx.scale_factor().get(cx));
    art::pixels(points * factor, device)
}

/// Reads `dir` again at once, so what just arrived shows without waiting for the poll.
pub(crate) fn refresh(cx: &mut Cx, dir: PathBuf, listing: Signal<Option<Listing>>) {
    cx.spawn(folder::read(dir), move |cx, fresh| {
        if fresh.is_some() {
            listing.set(cx, fresh);
        }
    });
}

/// A paper's look as drawn off the UI thread.
enum Drawn {
    Photo(Print),
    Icon(Option<Pixels>),
}

impl From<Drawn> for Look {
    fn from(drawn: Drawn) -> Self {
        match drawn {
            Drawn::Photo(print) => Self::Photo(Rc::new(print)),
            Drawn::Icon(Some(icon)) => Self::Icon(Rc::new(icon)),
            Drawn::Icon(None) => Self::Missing,
        }
    }
}

async fn draw(path: PathBuf, kind: Kind, sizes: Sizes) -> Drawn {
    if kind == Kind::Photo {
        let (width, height) = sizes.photo;
        if let Some(print) = folder::print(path.clone(), width, height).await {
            return Drawn::Photo(print);
        }
    }
    Drawn::Icon(folder::icon(path, kind == Kind::Folder, sizes.icon).await)
}

/// Draws what each of `newest` shows at `sizes`: a print of a photo, the
/// theme's picture of anything else. What already shows stays until its
/// redraw arrives, which waits for a resize to settle.
fn load_looks(cx: &mut Cx, newest: &[Entry], sizes: Sizes, looks: Signal<Looks>) {
    let carried: Looks = looks.with(cx, |previous| {
        newest
            .iter()
            .map(|entry| {
                let look = previous
                    .iter()
                    .find(|(path, _)| *path == entry.path)
                    .map_or(Look::Pending, |(_, look)| look.clone());
                (entry.path.clone(), look)
            })
            .collect()
    });
    for (index, (entry, (_, look))) in newest.iter().zip(&carried).enumerate() {
        if look.fits(sizes) {
            continue;
        }
        let settle = *look != Look::Pending;
        let path = entry.path.clone();
        cx.spawn(
            art::after_settling(settle, draw(path.clone(), entry.kind, sizes)),
            move |cx, drawn| {
                looks.update(cx, |looks| {
                    if let Some((shown, slot)) = looks.get_mut(index)
                        && *shown == path
                    {
                        *slot = drawn.into();
                    }
                });
            },
        );
    }
    looks.set(cx, carried);
}

/// The newest photo among the papers, which sets how dark the panes are.
fn newest_print(looks: Signal<Looks>) -> impl Fn(&mut Runtime) -> Option<Rc<Print>> {
    move |runtime| {
        looks.with(runtime, |looks| {
            looks.iter().find_map(|(_, look)| match look {
                Look::Photo(print) => Some(Rc::clone(print)),
                _ => None,
            })
        })
    }
}

pub(crate) fn shown_when(shown: bool) -> f32 {
    if shown { 1.0 } else { 0.0 }
}

fn place(view: impl View + Styled, rect: (f32, f32, f32, f32), scale: Scale) -> impl View + Styled {
    view.absolute()
        .left(scale.px(rect.0))
        .top(scale.px(rect.1))
        .w(scale.px(rect.2))
        .h(scale.px(rect.3))
}

/// Takes files dragged over the folder, copying them in as they drop.
fn take_drops(
    stage: impl Listeners + View,
    state: State,
    dir: PathBuf,
    listing: Signal<Option<Listing>>,
) -> impl Listeners + View {
    stage
        .on_drag_over(move |cx, event| {
            if event.offers(delivery::URI_LIST) {
                event.accept(delivery::URI_LIST);
                if !state.dragging.get(cx) {
                    state.dragging.set(cx, true);
                }
            }
        })
        .on_drag_leave(move |cx, _| state.dragging.set(cx, false))
        .on_drop(move |cx, event| {
            state.dragging.set(cx, false);
            let files = uri::file_paths(&String::from_utf8_lossy(&event.data));
            let into = dir.clone();
            cx.spawn(folder::copy_in(dir.clone(), files), move |cx, names| {
                delivery::receive(cx, state, into, listing, names, delivery::DROPPED_HOVER);
            });
        })
}

/// Keeps the widget drawn above its neighbours while anything of it reaches
/// over them, until spread papers have settled back in.
fn raise_while_spilling(cx: &mut Cx, state: State, tip: Signal<Tip>, raised: Signal<bool>) {
    cx.effect(move |runtime| {
        let settling_back = tip.get(runtime).0 > tipping::LIFTED + 0.01;
        let spilling = state.opened.get(runtime)
            || settling_back
            || state.delivery.with(runtime, Option::is_some);
        if raised.get(runtime) != spilling {
            raised.set(runtime, spilling);
        }
    });
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let dir = folder::expand(&settings.path);
    let title = settings.title(&dir);
    let listing = cx.signal(None::<Listing>);
    if env.is_live() {
        folder::follow(cx, dir.clone(), listing);
    } else {
        listing.set(cx, Some(sample()));
    }
    let state = State {
        hovered: cx.signal(false),
        dragging: cx.signal(false),
        opened: cx.signal(false),
        closed_under_pointer: cx.signal(false),
        delivery: cx.signal(None),
    };
    let (tip, tipper) = tipping::follow(cx, state);
    raise_while_spilling(cx, state, tip, env.raised);
    let scale = Scale::new(cx, env.size, measure);
    let looks = cx.signal(Looks::new());
    let device_scale = cx.scale_factor();
    let factor = scale.factor();
    let newest: Memo<Vec<Entry>> = cx.memo(move |runtime| {
        listing.with(runtime, |listing| {
            listing
                .as_ref()
                .map(|listing| listing.newest.clone())
                .unwrap_or_default()
        })
    });
    let loader = keyed(
        move |runtime| {
            let scale = factor.get(runtime);
            let device = device_scale.get(runtime);
            let sizes = Sizes {
                photo: (
                    art::pixels(PAPER.2 * scale, device),
                    art::pixels(PAPER.3 * scale, device),
                ),
                icon: art::pixels(ICON * scale, device),
            };
            (newest.get(runtime), sizes)
        },
        move |cx, (entries, sizes)| {
            if env.is_live() {
                load_looks(cx, &entries, sizes, looks);
            }
            vstack(())
        },
    )
    .absolute();
    let shown = keyed(
        move |runtime| newest.get(runtime),
        move |_, entries| papers::view(&entries, looks, state, scale),
    );
    let tone = cx.memo(move |runtime| {
        Tone::of(newest_print(looks)(runtime).is_some_and(|print| print.dark))
    });
    let back = place(
        canvas(move |runtime, size, list| {
            let factor = factor.get(runtime);
            let front = (
                Vector::new(FRONT.0 - BACK.0, FRONT.1 - BACK.1) * factor,
                Size::new(FRONT.2 * factor, FRONT.3 * factor),
                tip.get(runtime),
            );
            front::paint_back(list, size, factor, tone.get(runtime), front);
        })
        .pointer_events_none(),
        BACK,
        scale,
    );
    let inside = vstack((
        loader,
        tipper,
        shown,
        delivery::chips(state, dir.clone(), tone, scale),
        delivery::light_inside(cx, state, scale),
    ))
    .absolute()
    .inset(0.0)
    .pointer_events_none();
    let opened_dir = dir.clone();
    let pane = place(
        canvas(move |runtime, size, list| {
            front::paint_front(
                list,
                size,
                factor.get(runtime),
                tone.get(runtime),
                tip.get(runtime),
            );
        }),
        FRONT,
        scale,
    )
    .cursor(CursorIcon::Pointer)
    .role(Role::Button)
    .label(format!("Open {title}"))
    .on_click(move |cx, event| {
        event.stop_propagation();
        if event.count == 1 {
            state.toggle(cx);
        }
    })
    .on_double_click(move |cx, event| {
        event.stop_propagation();
        state.close(cx);
        folder::open(&opened_dir);
    });
    let light = place(
        delivery::light_through(state, tone, tip, scale),
        FRONT,
        scale,
    );
    let count = cx.memo(move |runtime| {
        listing.with(runtime, |listing| {
            listing.as_ref().map(|listing| listing.count)
        })
    });
    let caption = place(
        label::view(title, count, dir.clone(), listing, tone, state, scale),
        FRONT,
        scale,
    )
    .translate(bind(move |runtime| {
        Vector::new(0.0, tip.get(runtime).drop() * factor.get(runtime))
    }));
    let stage = zstack((back, inside, pane, light, caption))
        .relative()
        .w(scale.px(DESIGN.0))
        .h(scale.px(DESIGN.1))
        .role(Role::Group)
        .label("Folder")
        .on_pointer_enter(move |cx, _| state.point_at(cx, true))
        .on_pointer_leave(move |cx, _| state.point_at(cx, false));
    let stage = if env.is_live() {
        take_drops(stage, state, dir, listing).boxed()
    } else {
        stage.boxed()
    };
    Ok(vstack(stage)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .boxed())
}
