//! A folder on disk as a frosted card: its newest files stand in it as
//! papers, rise when the pointer comes over, and spread out to be opened.
//! Files dropped on it, or added with its button, fall in and light it up.

mod delivery;
mod front;
mod label;
mod papers;

use std::{path::PathBuf, rc::Rc};

use crownui::prelude::{
    CursorIcon, Cx, Listeners, Memo, Rect, Role, Runtime, Semantic, Signal, Size, Spring, Styled,
    View, bind, canvas, keyed, vstack, zstack,
};
use serde::{Deserialize, Serialize};

use crate::{Built, Env, Shape, art, scale::Scale};
use crownhome_board::WidgetSpec;
use crownhome_services::folder::{self, Entry, Kind, Listing};
use crownhome_util::{photo::Print, raster::Pixels, uri};
use delivery::Delivery;
use front::{Piece, Tone};

pub const KIND: &str = "folder";

/// The widget is drawn in a 2×2 tile, in design points.
const DESIGN: (f32, f32) = (237.5, 241.0);
/// The newest paper at rest: left, top, width and height.
const PAPER: (f32, f32, f32, f32) = (16.0, 34.0, 206.0, 138.0);
/// The front pane: left, top, width and height.
const FRONT: (f32, f32, f32, f32) = (4.0, 76.0, 229.5, 165.0);
/// Where the outline the edit handles hug begins.
const OUTLINE_TOP: f32 = 26.0;
/// How far spread papers and arriving names reach past the sides and the top.
const SPILL: (f32, f32) = (90.0, 140.0);
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
/// spread open, and the files arriving in it.
#[derive(Clone, Copy)]
pub(crate) struct State {
    pub hovered: Signal<bool>,
    pub dragging: Signal<bool>,
    pub opened: Signal<bool>,
    pub delivery: Signal<Option<Delivery>>,
}

impl State {
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

/// The front pane, frosted over the newest photo's haze.
fn pane(cx: &mut Cx, newest: Option<Rc<Print>>, piece: Piece, scale: Scale) -> impl View + use<> {
    let tone = Tone::of(newest.as_ref().is_some_and(|print| print.dark));
    let haze = newest.map(|print| {
        let haze = &print.haze;
        let id = cx.register_image(haze.width, haze.height, haze.rgba.clone());
        cx.release_image_on_cleanup(id);
        id
    });
    let factor = scale.factor();
    canvas(move |runtime, size, list| {
        front::paint(list, size, factor.get(runtime), piece, tone, haze);
    })
    .w_full()
    .h_full()
}

/// The newest photo among the papers, which the front is frosted over.
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

/// Keeps the widget drawn above its neighbours while anything of it reaches over them.
fn raise_while_spilling(cx: &mut Cx, state: State, raised: Signal<bool>) {
    cx.effect(move |runtime| {
        let spilling = state.opened.get(runtime) || state.delivery.with(runtime, Option::is_some);
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
        delivery: cx.signal(None),
    };
    raise_while_spilling(cx, state, env.raised);
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
    let reach = vstack((
        loader,
        shown,
        delivery::chips(state, dir.clone(), tone, scale),
        delivery::light_inside(cx, state, scale),
    ))
    .absolute()
    .left(scale.px(SPILL.0))
    .top(scale.px(SPILL.1))
    .w(scale.px(DESIGN.0))
    .h(scale.px(DESIGN.1));
    let papers = vstack(reach)
        .absolute()
        .left(scale.px(-SPILL.0))
        .top(scale.px(-SPILL.1))
        .w(scale.px(DESIGN.0 + 2.0 * SPILL.0))
        .h(scale.px(SPILL.1 + FRONT.1 + front::TAB_HEIGHT))
        .overflow_hidden()
        .pointer_events_none();
    let opened_dir = dir.clone();
    let body = place(
        vstack(
            keyed(newest_print(looks), move |cx, print| {
                pane(cx, print, Piece::Whole, scale)
            })
            .w_full()
            .h_full(),
        ),
        FRONT,
        scale,
    )
    .cursor(CursorIcon::Pointer)
    .role(Role::Button)
    .label(format!("Open {title}"))
    .on_click(move |cx, event| {
        event.stop_propagation();
        if state.opened.get(cx) {
            folder::open(&opened_dir);
        } else {
            state.opened.set(cx, true);
        }
    });
    let tab = place(
        vstack(
            keyed(newest_print(looks), move |cx, print| {
                pane(cx, print, Piece::Tab, scale)
            })
            .w_full()
            .h_full(),
        )
        .pointer_events_none()
        .opacity(bind(move |runtime| shown_when(!state.opened.get(runtime))))
        .transition(Spring::snappy()),
        FRONT,
        scale,
    );
    let solid = place(
        canvas(move |runtime, size, list| {
            front::paint_solid(list, size, factor.get(runtime), tone.get(runtime));
        })
        .pointer_events_none()
        .opacity(bind(move |runtime| shown_when(state.opened.get(runtime))))
        .transition(Spring::snappy()),
        FRONT,
        scale,
    );
    let light = place(delivery::light_through(state, tone, scale), FRONT, scale);
    let count = cx.memo(move |runtime| {
        listing.with(runtime, |listing| {
            listing.as_ref().map(|listing| listing.count)
        })
    });
    let caption = place(
        label::view(title, count, dir.clone(), listing, tone, state, scale),
        FRONT,
        scale,
    );
    let stage = zstack((body, papers, tab, solid, light, caption))
        .relative()
        .w(scale.px(DESIGN.0))
        .h(scale.px(DESIGN.1))
        .role(Role::Group)
        .label("Folder")
        .on_pointer_enter(move |cx, _| state.hovered.set(cx, true))
        .on_pointer_leave(move |cx, _| state.hovered.set(cx, false));
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
