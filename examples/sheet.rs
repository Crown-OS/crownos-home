//! Renders a board's widgets for a design sheet: frosted over the wallpaper,
//! filled with sample data, the clock stopped at 09:41. The surface hugs the
//! board's grid with a margin all round.
//!
//! `cargo run --example sheet -- <board.toml> <out.png> [wallpaper.jpg]
//!     [--dark] [--scale 2] [--margin 48]`

use std::{
    any::Any,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use crownhome_services::plugins::Plugins;
use crownhome_widgets::{Ambient, Env, Finish, palette::Palette};
use crownos_home::board::{BoardGrid, BoardSpec};
use crownui::{
    ext::TextSystem,
    kit::{self, INTER_FAMILY, provide_configured_theme},
    prelude::{
        AnyView, ColorScheme, CornerShape, Size, Styled, View, component, image, vstack, zstack,
    },
};
use impeller::{ImpellerBackend, Offscreen};
use runtime::InlineApp;
use taffy_layout::TaffyEngine;
use view::{TaskExecutor, TaskFuture, TaskId};

const SETTLE: Duration = Duration::from_millis(2500);
const FRAME: Duration = Duration::from_millis(16);
const SHOWN_TIME: (i8, i8) = (9, 41);

type App = InlineApp<ImpellerBackend, TaffyEngine>;
type Finished = (TaskId, Box<dyn Any + Send>);

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

struct Request {
    board: BoardSpec,
    output: PathBuf,
    wallpaper: Option<PathBuf>,
    dark: bool,
    scale: f32,
    margin: f32,
}

fn parse() -> Result<Request> {
    let mut arguments = std::env::args().skip(1);
    let (mut positional, mut dark, mut scale, mut margin) = (Vec::new(), false, 1.0, 48.0);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dark" => dark = true,
            "--scale" => scale = arguments.next().context("a scale")?.parse()?,
            "--margin" => margin = arguments.next().context("a margin")?.parse()?,
            flag if flag.starts_with("--") => bail!("unknown flag {flag}"),
            _ => positional.push(PathBuf::from(argument)),
        }
    }
    let mut positional = positional.into_iter();
    let board_file = positional.next().context("a board file")?;
    let contents = std::fs::read_to_string(&board_file)
        .with_context(|| format!("read {}", board_file.display()))?;
    Ok(Request {
        board: toml::from_str(&contents).context("parse the board")?,
        output: positional.next().context("an output file")?,
        wallpaper: positional.next(),
        dark,
        scale,
        margin,
    })
}

/// The board's block of cells with `margin` round it.
fn surface_of(board: &BoardSpec, margin: f32) -> Size {
    let grid = board.grid;
    let span = |count: u32, gap: f32| count as f32 * (grid.cell + gap) - gap + 2.0 * margin;
    Size::new(
        span(grid.columns, grid.column_gap),
        span(grid.rows, grid.row_gap),
    )
}

fn wallpaper(path: &PathBuf, surface: Size, scale: f32) -> Result<impl View> {
    let picture = ::image::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .resize_to_fill(
            (surface.width * scale) as u32,
            (surface.height * scale) as u32,
            ::image::imageops::FilterType::Lanczos3,
        )
        .into_rgba8();
    let (width, height) = picture.dimensions();
    Ok(image(width, height, picture.into_raw())
        .absolute()
        .inset(0.0))
}

fn stopped_clock() -> Result<jiff::Zoned> {
    let (hour, minute) = SHOWN_TIME;
    Ok(jiff::Zoned::now()
        .with()
        .hour(hour)
        .minute(minute)
        .second(0)
        .subsec_nanosecond(0)
        .build()?)
}

fn widgets(board: BoardSpec, surface: Size, now: jiff::Zoned, dark: bool) -> impl View {
    component(move |cx| {
        let theme = provide_configured_theme(cx);
        let palette = cx.memo(move |runtime| {
            Palette::new(dark, theme.with(runtime, |theme| theme.accent.start))
        });
        let ambient = Ambient {
            now: cx.signal(now.clone()),
            palette,
            plugins: Plugins::idle(cx).0,
        };
        let grid = BoardGrid::new(board.grid, surface);
        let placed: Vec<AnyView> = board
            .widgets
            .iter()
            .map(|spec| {
                let frame = grid.frame(spec.area());
                let env = Env {
                    ambient,
                    size: cx.memo(move |_| frame.size),
                    finish: Finish::Showcase,
                    raised: cx.signal(false),
                };
                vstack(crownhome_widgets::view(cx, spec, env))
                    .absolute()
                    .left(frame.origin.x)
                    .top(frame.origin.y)
                    .w(frame.size.width)
                    .h(frame.size.height)
                    .boxed()
            })
            .collect();
        zstack(placed)
            .relative()
            .w_full()
            .h_full()
            .font_family(INTER_FAMILY)
    })
}

fn settle(app: &mut App, tasks: &Receiver<Finished>) -> Result<()> {
    let mut passed = Duration::ZERO;
    while passed < SETTLE {
        while let Ok((id, output)) = tasks.try_recv() {
            app.ui().complete_task(id, output);
        }
        app.advance(FRAME)?;
        std::thread::sleep(Duration::from_millis(8));
        passed += FRAME;
    }
    Ok(())
}

fn main() -> Result<()> {
    let request = parse()?;
    let surface = surface_of(&request.board, request.margin);
    let backdrop = request
        .wallpaper
        .as_ref()
        .map(|path| wallpaper(path, surface, request.scale))
        .transpose()?;
    let view = zstack((
        backdrop,
        widgets(request.board, surface, stopped_clock()?, request.dark),
    ))
    .w_full()
    .h_full();
    let offscreen = Offscreen {
        width: (surface.width * request.scale).round() as u32,
        height: (surface.height * request.scale).round() as u32,
        scale: request.scale,
    };
    let mut app = App::with_surface(view, TaffyEngine::new(), surface, request.scale, &offscreen)
        .context("creating the offscreen renderer")?;
    app.ui().set_emulate_materials(true);
    if request.dark {
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
    let pixels = app.renderer_mut().read_pixels()?;
    std::fs::write(&request.output, impeller::encode_png(&pixels)?)
        .with_context(|| format!("writing {}", request.output.display()))?;
    println!("{}", request.output.display());
    Ok(())
}
