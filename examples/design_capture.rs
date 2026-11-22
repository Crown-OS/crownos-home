//! Captures every state of the desktop for the CrownOS-Design Figma file. Run both appearances
//! with `../crownui/capture/scripts/capture-app.sh .`; output lands in `$CROWN_DESIGN_DIR`.

use std::{
    any::Any,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use anyhow::{Context, Result};
use capture::{CaptureApp, Carrier};
use crownhome_services::plugins::Plugins;
use crownhome_widgets::{Ambient, Finish};
use crownos_home::{app_config, board::BoardSpec, ui};
use crownplugin_proto::{LauncherSize, LauncherSurface, Node, Tree};
use crowntest::{Query, Role, role};
use crownui::prelude::{Point, Rect};
use input::{PointerButton, PointerEvent};
use tokio::runtime::Handle;
use view::{TaskExecutor, TaskFuture, TaskId};
use windowing::{DndActions, WindowEvent};

const OUTPUT: (f32, f32) = (1920.0, 1080.0);
const ZONE: &str = "Europe/Berlin";
const SHOWN_TIME: (i8, i8) = (9, 41);
const EMPTY_DESKTOP: Point = Point::new(260.0, 540.0);
const DRAG_STEPS: usize = 12;
const HOME_FOLDERS: [&str; 3] = ["Pictures", "Documents", "Downloads"];
const PUMP_FRAMES: usize = 90;
const PUMP_PAUSE: Duration = Duration::from_millis(10);
/// A port nothing listens on, so live feeds fail fast instead of reaching the network.
const DEAD_PROXY: &str = "http://127.0.0.1:9";
const DEAD_BUS: &str = "unix:path=/nonexistent/bus";
const URI_LIST: &str = "text/uri-list";

const EVERY_WIDGET: &str = r#"
[[widget]]
kind = "music"
column = 0
row = 0
columns = 4
rows = 2

[[widget]]
kind = "weather"
column = 4
row = 0
columns = 2
rows = 2
[widget.settings]
name = "Munich"

[[widget]]
kind = "calendar"
column = 0
row = 2
columns = 4
rows = 2

[[widget]]
kind = "clock"
column = 4
row = 2
columns = 2
rows = 2
[widget.settings]
timezone = "Europe/Berlin"
"#;

const MORE_WIDGETS: &str = r#"
[[widget]]
kind = "earbuds"
column = 0
row = 0
columns = 2
rows = 3

[[widget]]
kind = "vinyl"
column = 2
row = 0
columns = 2
rows = 2

[[widget]]
kind = "folder"
column = 4
row = 0
columns = 2
rows = 2
[widget.settings]
path = "~/Documents"

[[widget]]
kind = "weather"
column = 2
row = 2
columns = 2
[widget.settings]
name = "Paris"

[[widget]]
kind = "weather"
column = 4
row = 2
columns = 2
[widget.settings]
name = "Tokyo"

[[widget]]
kind = "plugin"
column = 0
row = 3
columns = 2
[widget.settings]
source = "plugin:dev.crownos.system/CPU"
"#;

const LIVE_FOLDER: &str = r#"
[[widget]]
kind = "folder"
column = 2
row = 1
columns = 2
rows = 2
[widget.settings]
path = "~/Documents"
"#;

const CALENDAR_LOADING: &str = r#"
[[widget]]
kind = "calendar"
column = 0
row = 0
columns = 4
rows = 2

[[widget]]
kind = "clock"
column = 4
row = 0
columns = 2
rows = 2
[widget.settings]
timezone = "Europe/Berlin"
"#;

const UNAVAILABLE_WIDGETS: &str = r#"
[[widget]]
kind = "sticky-notes"
column = 0
row = 0
columns = 2
rows = 2

[[widget]]
kind = "plugin"
column = 2
row = 0
columns = 2
rows = 2
[widget.settings]
source = "plugin:dev.example.stocks/Portfolio"

[[widget]]
kind = "clock"
column = 4
row = 0
columns = 2
rows = 2
[widget.settings]
timezone = "Europe/Berlin"
"#;

/// Where the widgets' data comes from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Feeds {
    /// The widgets' own sample data, as on a design sheet.
    Sample,
    /// Live, with every feed failing: offline, signed out, nothing paired or playing.
    Offline,
    /// Live, before any feed has answered.
    Loading,
}

type Finished = (TaskId, Box<dyn Any + Send>);

/// Runs the desktop's `cx.spawn` futures, which a headless app leaves unstarted.
struct Tasks {
    runtime: Handle,
    finished: Sender<Finished>,
}

impl TaskExecutor for Tasks {
    fn spawn(&mut self, id: TaskId, future: TaskFuture) {
        let finished = self.finished.clone();
        self.runtime.spawn(async move {
            let _ = finished.send((id, future.await));
        });
    }

    fn cancel(&mut self, _id: TaskId) {}
}

struct Desktop {
    app: CaptureApp,
    finished: Option<Receiver<Finished>>,
}

impl Desktop {
    /// Plays frames while finished tasks land, so artwork and feeds settle in.
    fn pump(&mut self) {
        let Some(finished) = &self.finished else {
            return;
        };
        for _ in 0..PUMP_FRAMES {
            while let Ok((id, output)) = finished.try_recv() {
                self.app.ui().complete_task(id, output);
            }
            self.app.advance_ms(16);
            std::thread::sleep(PUMP_PAUSE);
        }
        self.app.settle();
    }
}

fn board(source: &str) -> Result<BoardSpec> {
    let mut board: BoardSpec = toml::from_str(source).context("parse a fixture board")?;
    board.grid = BoardSpec::default().grid;
    Ok(board)
}

fn shown_time() -> Result<jiff::Zoned> {
    let (hour, minute) = SHOWN_TIME;
    Ok(jiff::Zoned::now()
        .with()
        .hour(hour)
        .minute(minute)
        .second(0)
        .subsec_nanosecond(0)
        .build()?)
}

fn text(text: &str) -> Node {
    Node::Text {
        style: Vec::new(),
        text: text.to_owned(),
    }
}

fn plugin_tiles() -> Vec<LauncherSurface> {
    let cpu = Node::VStack {
        style: Vec::new(),
        children: vec![
            Node::HStack {
                style: Vec::new(),
                children: vec![
                    Node::Icon {
                        style: Vec::new(),
                        name: "cpu".to_owned(),
                    },
                    text("CPU"),
                    Node::Spacer,
                    Node::Badge {
                        text: "23%".to_owned(),
                        tone: Default::default(),
                    },
                ],
            },
            Node::Spacer,
            Node::Progress {
                style: Vec::new(),
                value: 0.23,
            },
        ],
    };
    vec![LauncherSurface {
        plugin: "dev.crownos.system".to_owned(),
        widget: "CPU".to_owned(),
        size: LauncherSize::Wide,
        tree: Tree { rev: 1, root: cpu },
    }]
}

fn open(spec: BoardSpec, feeds: Feeds, runtime: &Handle) -> Result<Desktop> {
    let now = shown_time()?;
    let finish = match feeds {
        Feeds::Sample => Finish::Showcase,
        Feeds::Offline | Feeds::Loading => Finish::Live,
    };
    let desktop = ui::desktop(spec, None, finish, move |cx, palette| {
        let (plugins, _) = Plugins::idle(cx);
        if feeds == Feeds::Sample {
            plugins.tiles.set(cx, plugin_tiles());
        }
        Ambient {
            now: cx.signal(now),
            palette,
            plugins,
        }
    });
    let mut app = capture::mount_app(desktop, &app_config(), Some(OUTPUT))?;
    let finished = (feeds != Feeds::Loading).then(|| {
        let (finished, landed) = channel();
        app.ui().set_task_executor(Box::new(Tasks {
            runtime: runtime.clone(),
            finished,
        }));
        landed
    });
    app.pointer_move(1.0, 1.0).settle();
    let mut desktop = Desktop { app, finished };
    desktop.pump();
    Ok(desktop)
}

fn bounds(app: &mut CaptureApp, query: Query) -> Rect {
    app.find(query).bounds
}

/// The lowest of the nodes `query` matches.
fn lowest(app: &mut CaptureApp, query: Query) -> Option<Rect> {
    let tree = app.ui().semantic_tree();
    query
        .find_all(tree)
        .into_iter()
        .map(|node| node.bounds)
        .max_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
}

fn centre(rect: Rect) -> Point {
    Point::new(
        rect.origin.x + rect.size.width / 2.0,
        rect.origin.y + rect.size.height / 2.0,
    )
}

fn right_click(app: &mut CaptureApp, at: Point) {
    let press = PointerEvent {
        button: Some(PointerButton::Secondary),
        timestamp: app.inline().now(),
        ..PointerEvent::mouse(at)
    };
    app.pointer_move(at.x, at.y)
        .deliver(WindowEvent::PointerDown(press))
        .deliver(WindowEvent::PointerUp(press))
        .settle();
}

fn choose(app: &mut CaptureApp, item: &str) {
    right_click(app, EMPTY_DESKTOP);
    app.click(role(Role::MenuItem).name(item)).settle();
}

/// Presses at `from` and carries the pointer to `to` without letting go.
fn hold_drag(app: &mut CaptureApp, from: Point, to: Point) {
    app.pointer_move(from.x, from.y)
        .pointer_down(from.x, from.y);
    for step in 1..=DRAG_STEPS {
        let progress = step as f32 / DRAG_STEPS as f32;
        app.advance_ms(16).pointer_move(
            from.x + (to.x - from.x) * progress,
            from.y + (to.y - from.y) * progress,
        );
    }
    app.settle();
}

struct Session {
    carrier: Carrier,
    runtime: tokio::runtime::Runtime,
}

impl Session {
    /// Mounts `spec` fed by `feeds`, brings it to a state with `reach` and captures it.
    fn shot(
        &mut self,
        (spec, feeds): (BoardSpec, Feeds),
        (group, name, description): (&str, &str, &str),
        reach: impl FnOnce(&mut CaptureApp) -> Result<()>,
    ) -> Result<()> {
        let mut desktop = open(spec, feeds, self.runtime.handle())?;
        reach(&mut desktop.app)?;
        desktop.pump();
        let group = format!("Current build · {group}");
        self.carrier
            .capture_state(&mut desktop.app, &group, name, description)?;
        Ok(())
    }

    fn desktop(&mut self) -> Result<()> {
        let group = "Desktop";
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Default board",
                "The built-in board: two weather pills and a clock, at 09:41",
            ),
            |_| Ok(()),
        )?;
        self.shot(
            (board(EVERY_WIDGET)?, Feeds::Sample),
            (
                group,
                "Music, weather, calendar, clock",
                "A full board: music with lyrics, the weather card, today's calendar and the clock",
            ),
            |_| Ok(()),
        )?;
        self.shot(
            (board(MORE_WIDGETS)?, Feeds::Sample),
            (
                group,
                "Earbuds, vinyl, folder, plugin",
                "Earbuds, the vinyl case, a folder, two weather pills and a plugin tile",
            ),
            |_| Ok(()),
        )
    }

    fn folder(&mut self) -> Result<()> {
        let group = "Widget states";
        self.shot(
            (board(MORE_WIDGETS)?, Feeds::Sample),
            (
                group,
                "Folder hovered",
                "The pointer over a folder lifts its newest papers",
            ),
            |app| {
                let at = centre(bounds(app, role(Role::Group).name("Folder")));
                app.pointer_move(at.x, at.y).settle();
                Ok(())
            },
        )?;
        self.shot(
            (board(MORE_WIDGETS)?, Feeds::Sample),
            (
                group,
                "Folder opened",
                "A click spreads the newest files out of the folder",
            ),
            |app| {
                app.click(role(Role::Button).name("Open Documents"))
                    .settle();
                Ok(())
            },
        )?;
        self.shot(
            (board(LIVE_FOLDER)?, Feeds::Offline),
            (
                group,
                "Folder with files",
                "A live folder: its newest files drawn with the icon theme's pictures",
            ),
            |_| Ok(()),
        )?;
        self.shot(
            (board(LIVE_FOLDER)?, Feeds::Offline),
            (
                group,
                "Files dragged over folder",
                "A drag of files held over the folder opens it to take them",
            ),
            |app| {
                let at = centre(bounds(app, role(Role::Group).name("Folder")));
                app.drag_enter(at.x, at.y, &[URI_LIST], DndActions::COPY_MOVE)
                    .drag_over(at.x, at.y + 4.0)
                    .settle();
                Ok(())
            },
        )
    }

    fn menus(&mut self) -> Result<()> {
        let group = "Menus";
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (group, "Desktop menu", "Right-click on the empty desktop"),
            |app| {
                right_click(app, EMPTY_DESKTOP);
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Desktop menu, row highlighted",
                "The pill highlight under the pointer",
            ),
            |app| {
                right_click(app, EMPTY_DESKTOP);
                app.hover(role(Role::MenuItem).name("Edit Widgets"))
                    .settle();
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Widget menu",
                "Right-click on a widget offers to remove it",
            ),
            |app| {
                let clock = centre(bounds(app, role(Role::ListItem).name("clock")));
                right_click(app, clock);
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Menu while editing",
                "Done Editing replaces Edit Widgets",
            ),
            |app| {
                choose(app, "Edit Widgets");
                right_click(app, EMPTY_DESKTOP);
                Ok(())
            },
        )
    }

    fn editing(&mut self) -> Result<()> {
        let group = "Editing";
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Edit mode",
                "Every widget shows its rim, remove badge and resize grip",
            ),
            |app| {
                choose(app, "Edit Widgets");
                Ok(())
            },
        )?;
        self.shot(
            (board(EVERY_WIDGET)?, Feeds::Sample),
            (
                group,
                "Edit mode, full board",
                "The handles on a full board",
            ),
            |app| {
                choose(app, "Edit Widgets");
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Moving a widget",
                "The clock lifted and carried; the landing cells glow where it fits",
            ),
            |app| {
                choose(app, "Edit Widgets");
                let clock = centre(bounds(app, role(Role::ListItem).name("clock")));
                hold_drag(app, clock, Point::new(clock.x + 290.0, clock.y + 20.0));
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Move blocked",
                "Over another widget the landing turns red",
            ),
            |app| {
                choose(app, "Edit Widgets");
                let clock = centre(bounds(app, role(Role::ListItem).name("clock")));
                hold_drag(app, clock, Point::new(clock.x + 150.0, clock.y - 100.0));
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Resizing a widget",
                "The grip dragged out; the landing shows the new span",
            ),
            |app| {
                choose(app, "Edit Widgets");
                let grip = lowest(app, role(Role::Button).name("Resize Widget"))
                    .context("no resize grip")?;
                let corner = Point::new(
                    grip.origin.x + grip.size.width * 0.7,
                    grip.origin.y + grip.size.height * 0.7,
                );
                hold_drag(app, corner, Point::new(corner.x + 140.0, corner.y + 110.0));
                Ok(())
            },
        )
    }

    fn gallery(&mut self) -> Result<()> {
        let group = "Widget gallery";
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Add Widgets",
                "The gallery along the right edge, the board in edit mode",
            ),
            |app| {
                choose(app, "Add Widgets…");
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Gallery scrolled",
                "Folders and plugin tiles further down",
            ),
            |app| {
                choose(app, "Add Widgets…");
                app.scroll(role(Role::Button).name("Clock"), 600.0).settle();
                Ok(())
            },
        )?;
        self.shot(
            (BoardSpec::default(), Feeds::Sample),
            (
                group,
                "Carrying a widget in",
                "A gallery card dragged onto the board, its landing marked",
            ),
            |app| {
                choose(app, "Add Widgets…");
                let card = centre(bounds(app, role(Role::Button).name("Vinyl")));
                hold_drag(app, card, Point::new(1040.0, 560.0));
                Ok(())
            },
        )
    }

    fn empty_and_error(&mut self) -> Result<()> {
        let group = "Empty & error states";
        self.shot(
            (board(EVERY_WIDGET)?, Feeds::Offline),
            (group, "Offline and signed out", "Live widgets with no network or account: no forecast, nothing playing, calendar signed out"),
            |_| Ok(()),
        )?;
        let empty_folder = MORE_WIDGETS.replace("~/Documents", "~/Downloads");
        self.shot(
            (board(&empty_folder)?, Feeds::Offline),
            (
                group,
                "Nothing paired or playing",
                "No earbuds paired, the vinyl idle, an empty folder and no plugin daemon",
            ),
            |_| Ok(()),
        )?;
        self.shot(
            (board(CALENDAR_LOADING)?, Feeds::Loading),
            (
                group,
                "Calendar loading",
                "The calendar before its first answer arrives",
            ),
            |_| Ok(()),
        )?;
        self.shot(
            (board(UNAVAILABLE_WIDGETS)?, Feeds::Sample),
            (
                group,
                "Unavailable widgets",
                "An unknown kind and a missing plugin keep their space as empty glass",
            ),
            |_| Ok(()),
        )
    }
}

/// A believable home with the usual folders, a fixed zone, no buses and no network, so nothing
/// of this machine or its accounts shows.
fn stage_environment() -> Result<()> {
    let home = std::env::var_os("CROWN_DESIGN_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(".home")
        .join("alex");
    for folder in HOME_FOLDERS {
        std::fs::create_dir_all(home.join(folder))?;
    }
    let documents = home.join("Documents");
    for file in ["Itinerary.pdf", "Notes.txt", "Budget.csv"] {
        std::fs::write(documents.join(file), b"")?;
    }
    // SAFETY: set once at start-up, before any other thread exists.
    unsafe {
        std::env::set_var("HOME", &home);
        for (variable, dir) in [
            ("XDG_CONFIG_HOME", ".config"),
            ("XDG_DATA_HOME", ".local/share"),
            ("XDG_CACHE_HOME", ".cache"),
            ("XDG_STATE_HOME", ".local/state"),
        ] {
            std::env::set_var(variable, home.join(dir));
        }
        std::env::set_var("TZ", ZONE);
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", DEAD_BUS);
        std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", DEAD_BUS);
        for proxy in [
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ] {
            std::env::set_var(proxy, DEAD_PROXY);
        }
        std::env::remove_var("NO_PROXY");
        std::env::remove_var("no_proxy");
    }
    Ok(())
}

fn main() -> Result<()> {
    stage_environment()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let mut session = Session {
        runtime,
        carrier: Carrier::new(
            "Home",
            "crownos-home: the desktop's widget board over the wallpaper at a 1920×1080 output, with its menus, edit mode and widget gallery, captured headlessly from the current source.",
        ),
    };
    session.desktop()?;
    session.folder()?;
    session.menus()?;
    session.editing()?;
    session.gallery()?;
    session.empty_and_error()?;
    let path = session.carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}
