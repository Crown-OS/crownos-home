use crownui::{
    ext::{Brush, GradientStop, Paint},
    prelude::{
        CornerRadii, Cx, Memo, Point, Rect, Styled, View, canvas, image, keyed, rgb, vstack, zstack,
    },
};

use crate::{Env, art, scale::Scale};
use crownhome_services::music::{Track, fetch_cover};
use crownhome_util::raster::Pixels;

const EDGE: f32 = 112.0;
const ROUNDING: f32 = 16.0;
const BADGE: f32 = 22.0;
const BADGE_INSET: f32 = 6.0;
const SPOTIFY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="12" fill="#121212"/><g fill="none" stroke="#fff" stroke-linecap="round"><path d="M6.4 9.3c3.7-1.1 7.8-.8 11.2 1" stroke-width="1.9"/><path d="M7.1 12.5c3.1-.8 6.3-.5 9 .9" stroke-width="1.6"/><path d="M7.8 15.5c2.4-.6 4.8-.4 6.9.7" stroke-width="1.3"/></g></svg>"##;

/// A soft gradient standing in for a cover that has not loaded, or has none.
fn blank() -> impl View {
    canvas(|_, size, list| {
        let paint = Paint {
            brush: Brush::LinearGradient {
                start: Point::new(0.0, 0.0),
                end: Point::new(size.width, size.height),
                stops: [(0.0, rgb(0x8E8E96)), (1.0, rgb(0x46464E))]
                    .into_iter()
                    .map(|(offset, color)| GradientStop { offset, color })
                    .collect(),
            },
            blur: None,
        };
        list.fill(
            Rect::from_xywh(0.0, 0.0, size.width, size.height),
            CornerRadii::uniform(0.0),
            paint,
        );
    })
    .w_full()
    .h_full()
}

fn picture(cx: &mut Cx, url: Option<String>, pixels: u32, live: bool) -> impl View + use<> {
    let loaded = cx.signal(None::<Pixels>);
    if let Some(url) = url.filter(|_| live) {
        cx.spawn(fetch_cover(url, pixels), move |cx, cover| {
            loaded.set(cx, cover);
        });
    }
    keyed(
        move |runtime| loaded.with(runtime, Option::is_some),
        move |cx, ready| match ready.then(|| loaded.with(cx, Clone::clone)).flatten() {
            Some(cover) => image(cover.edge, cover.edge, cover.rgba)
                .w_full()
                .h_full()
                .boxed(),
            None => blank().boxed(),
        },
    )
    .w_full()
    .h_full()
}

pub fn view(cx: &mut Cx, track: Memo<Option<Track>>, env: Env, scale: Scale) -> impl View + use<> {
    let edge = scale.length(cx, EDGE);
    let badge_edge = scale.length(cx, BADGE);
    let device_scale = cx.scale_factor();
    let live = env.is_live();
    let cover = keyed(
        move |runtime| {
            let url = track.with(runtime, |track| track.as_ref()?.cover_url.clone());
            (
                url,
                art::pixels(edge.get(runtime), device_scale.get(runtime)),
            )
        },
        move |cx, (url, pixels)| picture(cx, url, pixels, live),
    )
    .w_full()
    .h_full()
    .rounded(scale.px(ROUNDING))
    .overflow_hidden();
    let badge = vstack(art::picture(cx, |_| SPOTIFY, badge_edge))
        .absolute()
        .right(scale.px(BADGE_INSET))
        .bottom(scale.px(BADGE_INSET));
    zstack((cover, badge))
        .relative()
        .w(edge)
        .h(edge)
        .shrink(0.0)
}
