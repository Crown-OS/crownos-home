//! Widgets: one live glass tile per thing on the board.
//!
//! A widget is a view built from its saved settings. It sizes everything it
//! draws from the frame it is given, so the same view fills a small tile, a
//! large one, or a preview in the gallery.

pub mod app;
pub mod art;
pub mod calendar;
pub mod catalog;
pub mod clock;
pub mod countdown;
pub mod env;
pub mod event;
pub mod glass;
pub mod music;
pub mod occasion;
pub mod palette;
pub mod plugin;
pub mod scale;
pub mod search;
pub mod shape;
pub mod weather;

mod error;

use crownui::prelude::{AnyView, Cx, Size, Styled, View, vstack};

pub use env::{Ambient, Env, Finish};
pub use error::{Result, WidgetError};
pub use shape::Shape;

use crownhome_board::WidgetSpec;

/// The widget `spec` describes; an empty glass tile when it cannot be built,
/// so the space it holds stays visible and can be removed.
pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> AnyView {
    let built = match spec.kind.as_str() {
        weather::KIND => weather::view(cx, spec, env),
        clock::KIND => clock::view(cx, spec, env),
        event::KIND => event::view(cx, spec, env),
        calendar::KIND => calendar::view(cx, spec, env),
        music::KIND => music::view(cx, spec, env),
        countdown::KIND => countdown::view(cx, spec, env),
        app::KIND => app::view(cx, spec, env),
        search::KIND => search::view(cx, spec, env),
        plugin::KIND => plugin::view(cx, spec, env),
        unknown => Err(WidgetError::UnknownWidget(unknown.to_owned())),
    };
    built.unwrap_or_else(|e: WidgetError| {
        log::warn!("widget at {},{}: {e}", spec.column, spec.row);
        placeholder(env)
    })
}

/// The outline a widget of `kind` draws in a frame of `size`.
pub fn shape(kind: &str, size: Size) -> Shape {
    match kind {
        weather::KIND => weather::shape(size),
        clock::KIND => clock::shape(size),
        event::KIND => event::shape(size),
        calendar::KIND => calendar::shape(size),
        music::KIND => music::shape(size),
        countdown::KIND => countdown::shape(size),
        app::KIND => app::shape(size),
        search::KIND => search::shape(size),
        _ => Shape::filling(size, shape::DEFAULT_ROUNDING),
    }
}

fn placeholder(env: Env) -> AnyView {
    glass::glass(
        vstack(())
            .w_full()
            .h_full()
            .rounded(shape::DEFAULT_ROUNDING),
        env,
        |palette| palette.frost,
    )
    .boxed()
}

pub(crate) type Built = Result<AnyView>;
