//! One app's icon. A click opens it, through the compositor when it knows the
//! icon so the window grows out of it.

use std::path::{Path, PathBuf};

use crownui::prelude::{
    Color, Cx, Listeners, Memo, Runtime, Size, Styled, View, bind, image, keyed, rgb, rgba, text,
    vstack, zstack,
};
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape,
    art::{self, ArtName},
};
use crownhome_board::WidgetSpec;
use crownhome_services::launch;
use crownhome_util::{icon, raster::Pixels, xdg};

pub const KIND: &str = "app";

/// How much of the tile the icon fills, and how round its corners are.
pub const ICON_FILL: f32 = 0.89;
pub const ICON_ROUNDING: f32 = 0.225;
const PLACEHOLDER: Color = rgba(0xFFFFFF59);
const WEEKDAY_INK: Color = rgb(0x4578CE);
const DAY_INK: Color = rgb(0x101216);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// A desktop id such as `org.gnome.Nautilus`, or the path of an entry.
    pub app: String,
    /// Built-in artwork to show instead of the theme's icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<ArtName>,
}

impl Settings {
    pub fn of_entry(desktop_file: &Path) -> Self {
        Self {
            app: xdg::desktop_id(desktop_file),
            art: None,
        }
    }

    /// The id the compositor knows the app's windows by.
    pub fn app_id(&self) -> &str {
        let name = Path::new(&self.app)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.app);
        name.strip_suffix(".desktop").unwrap_or(name)
    }
}

/// Opens the app: the compositor animates it out of its icon when it can,
/// and asks back through `on_launch_app`; otherwise it is started directly.
pub fn open(cx: &mut Cx, settings: &Settings) {
    if cx.activate_app_icon(settings.app_id()) {
        return;
    }
    match xdg::find_desktop_entry(&settings.app) {
        Some(entry) => launch::start(entry),
        None => log::warn!("{} is not installed", settings.app),
    }
}

async fn load(desktop_file: PathBuf, edge: u32) -> Option<Pixels> {
    tokio::task::spawn_blocking(move || icon::of_app(&desktop_file, edge))
        .await
        .ok()
        .flatten()
}

/// The theme's icon for `app`, decoded off the UI thread at the size it covers.
fn themed(cx: &mut Cx, app: String, edge: Memo<f32>) -> impl View {
    let device_scale = cx.scale_factor();
    keyed(
        move |runtime| art::pixels(edge.get(runtime), device_scale.get(runtime)),
        move |cx, pixels| {
            let loaded = cx.signal(None::<Pixels>);
            if let Some(entry) = xdg::find_desktop_entry(&app) {
                cx.spawn(load(entry, pixels), move |cx, decoded| {
                    loaded.set(cx, decoded)
                });
            }
            keyed(
                move |runtime| loaded.with(runtime, Option::is_some),
                move |cx, ready| match ready.then(|| loaded.with(cx, Clone::clone)).flatten() {
                    Some(picture) => image(picture.edge, picture.edge, picture.rgba)
                        .w_full()
                        .h_full()
                        .boxed(),
                    None => vstack(())
                        .w_full()
                        .h_full()
                        .rounded(bind(move |runtime: &mut Runtime| {
                            edge.get(runtime) * ICON_ROUNDING
                        }))
                        .bg(PLACEHOLDER)
                        .boxed(),
                },
            )
            .w_full()
            .h_full()
        },
    )
    .w(edge)
    .h(edge)
    .shrink(0.0)
}

/// The weekday and the date, drawn live over the blank calendar page.
fn calendar_page(now: crownui::prelude::Signal<Zoned>, edge: Memo<f32>) -> impl View {
    let weekday = text(bind(move |runtime: &mut Runtime| {
        now.with(runtime, |now| now.strftime("%a").to_string().to_uppercase())
    }))
    .text_size(bind(move |runtime: &mut Runtime| {
        (edge.get(runtime) * 0.2).round()
    }))
    .font_weight(620)
    .text_color(WEEKDAY_INK);
    let day = text(bind(move |runtime: &mut Runtime| {
        now.with(runtime, |now| now.day().to_string())
    }))
    .text_size(bind(move |runtime: &mut Runtime| {
        (edge.get(runtime) * 0.58).round()
    }))
    .text_color(DAY_INK);
    vstack((weekday, day))
        .absolute()
        .inset(0.0)
        .items_center()
        .pt(bind(move |runtime: &mut Runtime| edge.get(runtime) * 0.07))
}

pub fn shape(size: Size) -> Shape {
    let edge = size.width.min(size.height) * ICON_FILL;
    Shape::centred_square(size, edge, edge * ICON_ROUNDING)
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let edge = cx.memo(move |runtime| {
        let size = env.size.get(runtime);
        size.width.min(size.height) * ICON_FILL
    });
    let picture = match settings.art {
        Some(name) => {
            let svg = name.svg();
            let artwork = art::picture(cx, move |_| svg, edge).boxed();
            match name {
                ArtName::Calendar => zstack((artwork, calendar_page(env.ambient.now, edge)))
                    .relative()
                    .boxed(),
                _ => artwork,
            }
        }
        None => themed(cx, settings.app.clone(), edge).boxed(),
    };
    let tile = vstack(picture)
        .w_full()
        .h_full()
        .items_center()
        .justify_center();
    if !env.is_live() {
        return Ok(tile.boxed());
    }
    Ok(tile.on_click(move |cx, _| open(cx, &settings)).boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_id_drops_the_directory_and_the_extension() {
        let by_path = Settings {
            app: "/opt/tools/org.example.Tool.desktop".into(),
            art: None,
        };
        assert_eq!(by_path.app_id(), "org.example.Tool");
        let by_id = Settings {
            app: "org.gnome.Nautilus".into(),
            art: None,
        };
        assert_eq!(by_id.app_id(), "org.gnome.Nautilus");
    }
}
