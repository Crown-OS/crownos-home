//! The link to crownplugind: plugin tiles in, the user's clicks on them out.
//!
//! The board shows the tiles plugins contribute to the launcher. The daemon
//! may start after the desktop or restart under it; while it is away there are
//! no plugin tiles and the link tries again on a timer.

use std::time::Duration;

use crownos_ipc::adapter::tokio::AsyncClient;
use crownplugin_proto::{LauncherSnapshot, LauncherSurface, SurfaceKey, UiEvent, plugind};
use crownui::prelude::{Cx, Signal};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    watch,
};

const RETRY: Duration = Duration::from_secs(30);

/// The user acted on revision `rev` of a plugin's tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    pub surface: SurfaceKey,
    pub rev: u32,
    pub event: UiEvent,
}

#[derive(Clone, Copy)]
pub struct Plugins {
    /// Every tile the daemon has, empty while it is away.
    pub tiles: Signal<Vec<LauncherSurface>>,
    commands: Signal<UnboundedSender<Interaction>>,
}

impl Plugins {
    /// Connects to crownplugind for as long as the desktop runs.
    pub fn start(cx: &mut Cx) -> Self {
        let (plugins, commands) = Self::idle(cx);
        let (publish, published) = watch::channel(Vec::new());
        cx.spawn(link(publish, commands), |_, ()| {});
        follow(cx, published, plugins.tiles);
        plugins
    }

    /// No daemon: for previews and tests.
    pub fn idle(cx: &mut Cx) -> (Self, UnboundedReceiver<Interaction>) {
        let (sender, commands) = mpsc::unbounded_channel();
        let plugins = Self {
            tiles: cx.signal(Vec::new()),
            commands: cx.signal(sender),
        };
        (plugins, commands)
    }

    /// Where a plugin view sends what the user does to it.
    pub fn sender(self, cx: &mut Cx) -> UnboundedSender<Interaction> {
        self.commands.with(cx, Clone::clone)
    }
}

fn follow(
    cx: &mut Cx,
    mut published: watch::Receiver<Vec<LauncherSurface>>,
    tiles: Signal<Vec<LauncherSurface>>,
) {
    let changed = async move { published.changed().await.ok().map(|()| published) };
    cx.spawn(changed, move |cx, published| {
        let Some(mut published) = published else {
            return;
        };
        let fresh = published.borrow_and_update().clone();
        tiles.set(cx, fresh);
        follow(cx, published, tiles);
    });
}

async fn link(
    publish: watch::Sender<Vec<LauncherSurface>>,
    mut commands: UnboundedReceiver<Interaction>,
) {
    loop {
        if let Some((client, snapshot)) = connect().await
            && !session(&publish, &mut commands, client, snapshot).await
        {
            return;
        }
        publish.send_if_modified(|tiles| !std::mem::take(tiles).is_empty());
        tokio::time::sleep(RETRY).await;
        while commands.try_recv().is_ok() {}
    }
}

/// Subscribes first, then attaches, so no change lands between the two unseen.
async fn connect() -> Option<(AsyncClient, LauncherSnapshot)> {
    let client = plugind::Client::connect()
        .map_err(|e| log::debug!("crownplugind: {e}"))
        .ok()?;
    let mut client = AsyncClient::new(client.into_inner()).ok()?;
    client.subscribe::<plugind::LauncherChanged>().await.ok()?;
    let snapshot = client
        .call::<plugind::attach_launcher>(&plugind::attach_launcher {}, Vec::new())
        .await
        .ok()?;
    Some((client, snapshot))
}

/// Pumps one connection until it breaks. Returns whether the desktop is
/// still listening.
async fn session(
    publish: &watch::Sender<Vec<LauncherSurface>>,
    commands: &mut UnboundedReceiver<Interaction>,
    mut client: AsyncClient,
    attached: LauncherSnapshot,
) -> bool {
    let mut revision = attached.revision;
    publish.send_replace(attached.tiles);
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(Interaction { surface, rev, event }) = command else {
                    return false;
                };
                let sent = client
                    .notify::<plugind::ui_event>(&plugind::ui_event { surface, rev, event }, Vec::new())
                    .await;
                if sent.is_err() {
                    return true;
                }
            }
            pumped = client.pump() => {
                if pumped.is_err() || client.client().is_closed() {
                    return true;
                }
                let newest = std::iter::from_fn(|| {
                    client.client_mut().next_event::<plugind::LauncherChanged>()
                })
                .last()
                .map(|changed| changed.snapshot)
                .filter(|snapshot| snapshot.revision > revision);
                if let Some(snapshot) = newest {
                    revision = snapshot.revision;
                    publish.send_replace(snapshot.tiles);
                }
            }
        }
    }
}
