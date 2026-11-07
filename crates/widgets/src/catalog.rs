//! The widgets the gallery offers, each with the settings and size it starts at.

use crownplugin_proto::{LauncherSize, LauncherSurface};
use jiff::tz::TimeZone;
use serde::Serialize;

use crate::{calendar, clock, earbuds, folder, music, plugin, vinyl, weather};
use crownhome_board::WidgetSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub title: String,
    pub spec: WidgetSpec,
}

fn template(
    title: &str,
    kind: &str,
    (columns, rows): (u32, u32),
    settings: &impl Serialize,
) -> Template {
    let settings = match toml::Value::try_from(settings) {
        Ok(toml::Value::Table(table)) => table,
        _ => toml::Table::new(),
    };
    Template {
        title: title.to_owned(),
        spec: WidgetSpec {
            kind: kind.to_owned(),
            column: 0,
            row: 0,
            columns,
            rows,
            settings,
        },
    }
}

/// The city the system's time zone is named after, `Kolkata` for `Asia/Kolkata`.
fn local_city(zone: Option<&str>) -> String {
    zone.and_then(|name| name.rsplit('/').next())
        .filter(|city| !city.is_empty() && *city != "UTC")
        .map_or_else(|| "London".to_owned(), |city| city.replace('_', " "))
}

/// The home folders offered as folder cards, where they exist.
const FOLDERS: [&str; 3] = ["Pictures", "Documents", "Downloads"];

pub fn builtins() -> Vec<Template> {
    let system = TimeZone::system();
    let zone = system.iana_name();
    let mut templates = vec![
        template(
            "Weather",
            weather::KIND,
            (2, 2),
            &weather::Settings {
                name: local_city(zone),
                latitude: None,
                longitude: None,
                unit: Default::default(),
            },
        ),
        template(
            "Clock",
            clock::KIND,
            (2, 2),
            &clock::Settings::in_zone(zone.map(str::to_owned)),
        ),
        template("Music", music::KIND, (4, 2), &music::Settings::default()),
        template("Vinyl", vinyl::KIND, (2, 2), &vinyl::Settings::default()),
        template(
            "Earbuds",
            earbuds::KIND,
            (2, 3),
            &earbuds::Settings::default(),
        ),
        template(
            "Calendar",
            calendar::KIND,
            (4, 2),
            &calendar::Settings::default(),
        ),
    ];
    templates.extend(folders());
    templates
}

/// A folder card for each of the usual folders the home directory has.
fn folders() -> impl Iterator<Item = Template> {
    FOLDERS.into_iter().filter_map(|name| {
        let path = format!("~/{name}");
        crownhome_services::folder::expand(&path)
            .is_dir()
            .then(|| template(name, folder::KIND, (2, 2), &folder::Settings::of(&path)))
    })
}

/// A tile a plugin contributes, sized as the launcher would show it.
pub fn of_plugin(tile: &LauncherSurface) -> Template {
    let span = match tile.size {
        LauncherSize::Small => (1, 1),
        LauncherSize::Wide => (2, 1),
        LauncherSize::Square => (1, 2),
        LauncherSize::Large => (2, 2),
    };
    template(
        &tile.widget,
        plugin::KIND,
        span,
        &plugin::Settings::of(tile),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_local_city_comes_from_the_zone_name() {
        assert_eq!(local_city(Some("Asia/Kolkata")), "Kolkata");
        assert_eq!(local_city(Some("America/New_York")), "New York");
        assert_eq!(local_city(Some("UTC")), "London");
        assert_eq!(local_city(None), "London");
    }

    #[test]
    fn every_built_in_template_has_settings_its_kind_reads() {
        for template in builtins() {
            let spec = &template.spec;
            let decoded = match spec.kind.as_str() {
                weather::KIND => spec.decode::<weather::Settings>().map(drop),
                clock::KIND => spec.decode::<clock::Settings>().map(drop),
                music::KIND => spec.decode::<music::Settings>().map(drop),
                vinyl::KIND => spec.decode::<vinyl::Settings>().map(drop),
                earbuds::KIND => spec.decode::<earbuds::Settings>().map(drop),
                folder::KIND => spec.decode::<folder::Settings>().map(drop),
                calendar::KIND => spec.decode::<calendar::Settings>().map(drop),
                other => panic!("unexpected kind {other}"),
            };
            assert!(decoded.is_ok(), "{}: {decoded:?}", template.title);
        }
    }
}
