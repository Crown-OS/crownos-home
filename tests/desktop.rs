use crownos_home::{board::BoardSpec, ui};
use crowntest::{Role, TestApp, role};
use crownui::prelude::Point;
use input::{PointerButton, PointerEvent};
use windowing::WindowEvent;

const SURFACE: (f32, f32) = (1600.0, 1000.0);
const EMPTY_DESKTOP: Point = Point::new(150.0, 300.0);
/// The remove badge astride the top-left corner of the first widget, the
/// grid being centred on the surface.
const FIRST_BADGE: Point = Point::new(410.0, 243.0);

fn desktop() -> TestApp {
    let mut app = TestApp::with_size(ui::root(BoardSpec::default(), None), SURFACE.0, SURFACE.1);
    app.settle();
    app
}

fn right_click(app: &mut TestApp, at: Point) {
    let press = PointerEvent {
        button: Some(PointerButton::Secondary),
        ..PointerEvent::mouse(at)
    };
    app.pointer_move(at.x, at.y)
        .deliver(WindowEvent::PointerDown(press))
        .deliver(WindowEvent::PointerUp(press))
        .settle();
}

fn open_gallery(app: &mut TestApp) {
    right_click(app, EMPTY_DESKTOP);
    app.click(role(Role::MenuItem).name("Add Widgets…"))
        .settle();
}

#[test]
fn a_right_click_on_the_desktop_offers_to_add_widgets() {
    let mut app = desktop();
    assert!(!app.exists(role(Role::MenuItem).name("Add Widgets…")));
    right_click(&mut app, EMPTY_DESKTOP);
    assert!(app.exists(role(Role::MenuItem).name("Add Widgets…")));
    assert!(app.exists(role(Role::MenuItem).name("Change Wallpaper…")));
}

#[test]
fn add_widgets_opens_the_gallery_and_done_closes_it() {
    let mut app = desktop();
    open_gallery(&mut app);
    assert!(
        !app.exists(role(Role::MenuItem).name("Change Wallpaper…")),
        "the menu closes"
    );
    assert!(app.exists(role(Role::Dialog).name("Add Widgets")));
    assert!(app.exists(role(Role::Button).name("Clock")));
    app.click(role(Role::Button).name("Done")).settle();
    assert!(!app.exists(role(Role::Button).name("Clock")));
}

#[test]
fn a_click_on_a_gallery_card_adds_its_widget_where_there_is_room() {
    let mut app = desktop();
    open_gallery(&mut app);
    let before = app.count(role(Role::Button).name("Remove Widget"));
    app.click(role(Role::Button).name("Clock")).settle();
    assert_eq!(
        app.count(role(Role::Button).name("Remove Widget")),
        before + 1
    );
}

#[test]
fn editing_shows_a_remove_badge_on_every_widget_and_one_removes_its_widget() {
    let mut app = desktop();
    right_click(&mut app, EMPTY_DESKTOP);
    app.click(role(Role::MenuItem).name("Edit Widgets"))
        .settle();
    let widgets = BoardSpec::default().widgets.len();
    assert_eq!(app.count(role(Role::Button).name("Remove Widget")), widgets);
    app.pointer_move(FIRST_BADGE.x, FIRST_BADGE.y)
        .pointer_down(FIRST_BADGE.x, FIRST_BADGE.y)
        .pointer_up(FIRST_BADGE.x, FIRST_BADGE.y)
        .settle();
    assert_eq!(
        app.count(role(Role::Button).name("Remove Widget")),
        widgets - 1
    );
}
