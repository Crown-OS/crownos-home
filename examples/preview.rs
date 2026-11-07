//! Renders the desktop offscreen through Impeller, over a wallpaper, to a PNG,
//! with the compositor's blur done in-app. Pointer actions run in order first,
//! so the menu, the gallery and the editing gestures can be looked at too.
//!
//! `cargo run --example preview -- <out.png> [wallpaper.jpg] [--dark]
//!     [--board widgets.toml] [--hover x y] [--click x y] [--right-click x y]
//!     [--drag x0 y0 x1 y1]`

use std::{
    any::Any,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use crownos_home::{board::BoardSpec, ui};
use crownui::{
    ext::TextSystem,
    kit,
    prelude::{ColorScheme, CornerShape, Point, Size, Styled, View, image, zstack},
};
use impeller::{ImpellerBackend, Offscreen};
use input::{PointerButton, PointerEvent};
use runtime::InlineApp;
use taffy_layout::TaffyEngine;
use view::{TaskExecutor, TaskFuture, TaskId};
use windowing::WindowEvent;

const SURFACE: Size = Size::new(1600.0, 1000.0);
const SCALE: f32 = 1.0;
const SETTLE_ROUNDS: usize = 30;
const DRAG_STEPS: usize = 12;

type App = InlineApp<ImpellerBackend, TaffyEngine>;
type Finished = (TaskId, Box<dyn Any + Send>);

/// Runs the desktop's `cx.spawn` futures, which an offscreen app leaves unstarted.
struct Tasks {
    runtime: tokio::runtime::Runtime,
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

enum Action {
    Hover(Point),
    Click(Point, PointerButton),
    Drag(Point, Point),
}

fn wallpaper(path: Option<&PathBuf>) -> Result<Option<impl View>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let picture = ::image::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .resize_to_fill(
            SURFACE.width as u32,
            SURFACE.height as u32,
            ::image::imageops::FilterType::Triangle,
        )
        .into_rgba8();
    let (width, height) = picture.dimensions();
    Ok(Some(
        image(width, height, picture.into_raw())
            .absolute()
            .inset(0.0),
    ))
}

fn pointer(app: &App, at: Point, button: Option<PointerButton>) -> PointerEvent {
    PointerEvent {
        button,
        timestamp: app.now(),
        ..PointerEvent::mouse(at)
    }
}

fn settle(app: &mut App, tasks: &Receiver<Finished>) -> Result<()> {
    for _ in 0..SETTLE_ROUNDS {
        while let Ok((id, output)) = tasks.try_recv() {
            app.ui().complete_task(id, output);
        }
        app.advance(Duration::from_millis(16))?;
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn perform(app: &mut App, tasks: &Receiver<Finished>, action: &Action) -> Result<()> {
    match *action {
        Action::Hover(at) => {
            app.handle(&WindowEvent::PointerMove(pointer(app, at, None)))?;
        }
        Action::Click(at, button) => {
            let press = pointer(app, at, Some(button));
            app.handle(&WindowEvent::PointerMove(pointer(app, at, None)))?;
            app.handle(&WindowEvent::PointerDown(press))?;
            app.handle(&WindowEvent::PointerUp(pointer(app, at, Some(button))))?;
        }
        Action::Drag(from, to) => {
            app.handle(&WindowEvent::PointerMove(pointer(app, from, None)))?;
            app.handle(&WindowEvent::PointerDown(pointer(
                app,
                from,
                Some(PointerButton::Primary),
            )))?;
            for step in 1..=DRAG_STEPS {
                let progress = step as f32 / DRAG_STEPS as f32;
                let at = Point::new(
                    from.x + (to.x - from.x) * progress,
                    from.y + (to.y - from.y) * progress,
                );
                app.advance(Duration::from_millis(16))?;
                app.handle(&WindowEvent::PointerMove(pointer(app, at, None)))?;
            }
            app.handle(&WindowEvent::PointerUp(pointer(
                app,
                to,
                Some(PointerButton::Primary),
            )))?;
        }
    }
    settle(app, tasks)
}

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let (mut positional, mut actions, mut dark) = (Vec::new(), Vec::new(), false);
    let mut board = None;
    let number = |arguments: &mut dyn Iterator<Item = String>| -> Result<f32> {
        Ok(arguments.next().context("a coordinate")?.parse()?)
    };
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dark" => dark = true,
            "--board" => board = Some(PathBuf::from(arguments.next().context("a board file")?)),
            "--click" | "--right-click" => {
                let at = Point::new(number(&mut arguments)?, number(&mut arguments)?);
                let button = if argument == "--click" {
                    PointerButton::Primary
                } else {
                    PointerButton::Secondary
                };
                actions.push(Action::Click(at, button));
            }
            "--hover" => {
                let at = Point::new(number(&mut arguments)?, number(&mut arguments)?);
                actions.push(Action::Hover(at));
            }
            "--drag" => {
                let from = Point::new(number(&mut arguments)?, number(&mut arguments)?);
                let to = Point::new(number(&mut arguments)?, number(&mut arguments)?);
                actions.push(Action::Drag(from, to));
            }
            flag if flag.starts_with("--") => bail!("unknown flag {flag}"),
            _ => positional.push(PathBuf::from(argument)),
        }
    }
    let output = positional
        .first()
        .cloned()
        .unwrap_or_else(|| "home.png".into());

    let spec = board
        .as_deref()
        .map_or_else(BoardSpec::default, BoardSpec::load);
    let desktop = ui::root(spec, None);
    let view = zstack((wallpaper(positional.get(1))?, desktop))
        .w_full()
        .h_full();
    let surface = Offscreen {
        width: (SURFACE.width * SCALE) as u32,
        height: (SURFACE.height * SCALE) as u32,
        scale: SCALE,
    };
    let mut app = App::with_surface(view, TaffyEngine::new(), SURFACE, SCALE, &surface)
        .context("creating the offscreen renderer")?;
    app.ui().set_emulate_materials(true);
    if dark {
        app.ui().set_color_scheme(ColorScheme::Dark);
    }
    app.set_corner_shape(CornerShape::Squircle);
    app.text_cache()
        .system_mut()
        .register_font(kit::inter_font().to_vec(), Some(kit::INTER_FAMILY))?;
    let (finished, tasks) = channel();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    app.ui()
        .set_task_executor(Box::new(Tasks { runtime, finished }));
    settle(&mut app, &tasks)?;
    for action in &actions {
        perform(&mut app, &tasks, action)?;
    }
    let pixels = app.renderer_mut().read_pixels()?;
    std::fs::write(&output, impeller::encode_png(&pixels)?)
        .with_context(|| format!("writing {}", output.display()))?;
    println!("{}", output.display());
    Ok(())
}
