//! A crownplugin's tile, drawn from the tree it renders.
//!
//! Plugins declare tiles once, under `contributes.launcher`, and draw them
//! with `ui.render(id, tree)`. A board entry of kind `plugin` naming
//! `plugin:<id>/<tile>` shows that tile here, so a plugin never has to know
//! where it is placed.

use std::rc::Rc;

use crownplugin_proto::{LauncherSurface, Part, SurfaceKey, Tree};
use crownplugin_render::{EventSink, Resolver, render};
use crownui::prelude::{Cx, Styled, View, keyed, vstack};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{Built, Env, WidgetError, glass::glass};
use crownhome_board::WidgetSpec;
use crownhome_services::plugins::Interaction;

pub const KIND: &str = "plugin";

const ROUNDING: f32 = 28.0;
const PADDING: f32 = 16.0;
const SOURCE_PREFIX: &str = "plugin:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// `plugin:<plugin id>/<tile id>`.
    pub source: String,
}

impl Settings {
    pub fn of(tile: &LauncherSurface) -> Self {
        Self {
            source: format!("{SOURCE_PREFIX}{}/{}", tile.plugin, tile.widget),
        }
    }

    fn surface(&self) -> Option<SurfaceKey> {
        let (plugin, widget) = self.source.strip_prefix(SOURCE_PREFIX)?.split_once('/')?;
        Some(SurfaceKey {
            plugin: plugin.to_owned(),
            widget: widget.to_owned(),
            part: Part::Main,
        })
    }
}

struct Host {
    surface: SurfaceKey,
    rev: u32,
    sender: Option<UnboundedSender<Interaction>>,
}

impl Resolver for Host {
    fn events(&self) -> Option<EventSink> {
        let (surface, rev, sender) = (self.surface.clone(), self.rev, self.sender.clone()?);
        Some(Rc::new(move |event| {
            let _ = sender.send(Interaction {
                surface: surface.clone(),
                rev,
                event,
            });
        }))
    }
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let surface = settings
        .surface()
        .ok_or_else(|| WidgetError::UnknownWidget(settings.source.clone()))?;
    let plugins = env.ambient.plugins;
    let wanted = surface.clone();
    let tree = cx.memo(move |runtime| {
        plugins.tiles.with(runtime, |tiles| {
            tiles
                .iter()
                .find(|tile| tile.plugin == wanted.plugin && tile.widget == wanted.widget)
                .map(|tile| tile.tree.clone())
        })
    });
    let live = env.is_live();
    let content = keyed(
        move |runtime| tree.get(runtime),
        move |cx, tree: Option<Tree>| match tree {
            Some(tree) => {
                let host = Host {
                    surface: surface.clone(),
                    rev: tree.rev,
                    sender: live.then(|| plugins.sender(cx)),
                };
                render(cx, &tree, &host)
            }
            None => vstack(()).boxed(),
        },
    )
    .w_full()
    .h_full();
    let card = vstack(content)
        .w_full()
        .h_full()
        .p(PADDING)
        .rounded(ROUNDING)
        .overflow_hidden();
    Ok(glass(card, env, |palette| palette.frost).boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_names_a_plugin_and_its_tile() {
        let settings = Settings {
            source: "plugin:dev.example.cpu/graph".into(),
        };
        let surface = settings.surface().expect("a surface");
        assert_eq!(
            (surface.plugin.as_str(), surface.widget.as_str()),
            ("dev.example.cpu", "graph")
        );
        for broken in ["dev.example.cpu/graph", "plugin:no-slash"] {
            let settings = Settings {
                source: broken.into(),
            };
            assert!(settings.surface().is_none(), "{broken}");
        }
    }
}
