//! Artwork, rasterized at exactly the pixels it covers.
//!
//! Pictures are drawn off the UI thread. Once one shows, a new size waits
//! until the frame has held still for a moment, so a widget being resized
//! stretches its last picture instead of drawing one at every step.

use std::{future::Future, rc::Rc, time::Duration};

use crownui::prelude::{
    AnyView, Cx, Memo, Runtime, Size, Styled, View, image, keyed, vstack, zstack,
};

use crownhome_util::raster::{self, Picture};

/// Rasterized sizes are rounded up to this many pixels, so a widget being
/// resized is not redrawn for every pixel it grows.
const PIXEL_STEP: u32 = 8;
/// How long a frame holds still before what is shown in it is drawn again.
const SETTLE: Duration = Duration::from_millis(160);

pub fn pixels(points: f32, device_scale: f32) -> u32 {
    let wanted = (points * device_scale).ceil().max(1.0) as u32;
    wanted.div_ceil(PIXEL_STEP) * PIXEL_STEP
}

/// Runs `job`, after the frame has had time to settle when `settle` is set.
pub async fn after_settling<T>(settle: bool, job: impl Future<Output = T>) -> T {
    if settle {
        tokio::time::sleep(SETTLE).await;
    }
    job.await
}

/// A drawn picture, or the word that there is none, compared by identity so
/// swapping one in never compares pixels.
#[derive(Clone)]
struct Drawn(Option<Rc<Picture>>);

impl PartialEq for Drawn {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(shown), Some(other)) => Rc::ptr_eq(shown, other),
            (shown, other) => shown.is_none() && other.is_none(),
        }
    }
}

/// What `make` draws for the latest `key`, stretched over its frame; `blank`
/// stands in before the first picture and when there is none to draw.
pub fn rendered<KeyFn, Key, MakeFn, Job>(
    cx: &mut Cx,
    key: KeyFn,
    make: MakeFn,
    blank: fn() -> AnyView,
) -> impl View + Styled + use<KeyFn, Key, MakeFn, Job>
where
    KeyFn: FnMut(&mut Runtime) -> Key + 'static,
    Key: Clone + PartialEq + 'static,
    MakeFn: Fn(Key) -> Job + 'static,
    Job: Future<Output = Option<Picture>> + Send + 'static,
{
    let drawn = cx.signal(None::<Drawn>);
    let request = keyed(key, move |cx, key| {
        let settle = drawn.with(cx, Option::is_some);
        let job = make(key);
        cx.spawn(after_settling(settle, job), move |cx, picture| {
            drawn.set(cx, Some(Drawn(picture.map(Rc::new))))
        });
        vstack(())
    })
    .absolute();
    let shown = keyed(
        move |runtime| drawn.get(runtime).and_then(|drawn| drawn.0),
        move |_, picture| match picture {
            Some(picture) => image(picture.width, picture.height, picture.rgba.clone())
                .w_full()
                .h_full()
                .boxed(),
            None => blank(),
        },
    )
    .w_full()
    .h_full();
    zstack((request, shown))
}

fn nothing() -> AnyView {
    vstack(()).boxed()
}

async fn rasterized(work: impl FnOnce() -> Option<Picture> + Send + 'static) -> Option<Picture> {
    tokio::task::spawn_blocking(work).await.ok().flatten()
}

/// The SVG `source` picks, filling a square `edge` points wide.
pub fn picture<Source>(cx: &mut Cx, mut source: Source, edge: Memo<f32>) -> impl View + use<Source>
where
    Source: FnMut(&mut Runtime) -> &'static [u8] + 'static,
{
    let device_scale = cx.scale_factor();
    rendered(
        cx,
        move |runtime| {
            let svg = source(runtime);
            (svg, pixels(edge.get(runtime), device_scale.get(runtime)))
        },
        |(svg, pixels)| rasterized(move || raster::rasterize_svg(svg, pixels).map(Picture::from)),
        nothing,
    )
    .w(edge)
    .h(edge)
    .shrink(0.0)
}

/// `svg` filling a frame of `size`, centred and cropped to it.
pub fn backdrop(cx: &mut Cx, svg: &'static [u8], size: Memo<Size>) -> impl View + use<> {
    let device_scale = cx.scale_factor();
    rendered(
        cx,
        move |runtime| {
            let (size, scale) = (size.get(runtime), device_scale.get(runtime));
            (pixels(size.width, scale), pixels(size.height, scale))
        },
        move |(width, height)| {
            rasterized(move || raster::rasterize_svg_covering(svg, width, height))
        },
        nothing,
    )
    .w_full()
    .h_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_round_up_to_whole_steps_of_device_pixels() {
        assert_eq!(pixels(61.0, 1.0), 64);
        assert_eq!(pixels(61.0, 2.0), 128);
        assert_eq!(pixels(0.0, 1.0), 8);
    }
}
