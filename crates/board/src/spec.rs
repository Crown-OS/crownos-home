//! What the board shows and where, as `$XDG_CONFIG_HOME/crownos-home/widgets.toml`.
//!
//! Each widget's own settings stay an opaque table here and are decoded by its
//! kind, so a kind this build does not know survives a load and a save
//! untouched.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crownhome_util::{atomic, xdg};

use crate::{
    BoardError, Result,
    grid::{Area, GridSpec},
};

const FILE_NAME: &str = "widgets.toml";
const DEFAULT_BOARD: &str = include_str!("../assets/widgets.toml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardSpec {
    #[serde(default)]
    pub grid: GridSpec,
    #[serde(default, rename = "widget")]
    pub widgets: Vec<WidgetSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetSpec {
    pub kind: String,
    pub column: i32,
    pub row: i32,
    #[serde(default = "one")]
    pub columns: u32,
    #[serde(default = "one")]
    pub rows: u32,
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    pub settings: toml::Table,
}

const fn one() -> u32 {
    1
}

impl WidgetSpec {
    pub fn area(&self) -> Area {
        Area {
            column: self.column,
            row: self.row,
            columns: self.columns.max(1),
            rows: self.rows.max(1),
        }
    }

    pub fn placed(mut self, area: Area) -> Self {
        self.column = area.column;
        self.row = area.row;
        self.columns = area.columns;
        self.rows = area.rows;
        self
    }

    /// The settings decoded as `T`, every missing field taking its default.
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        toml::Value::Table(self.settings.clone())
            .try_into()
            .map_err(|source| BoardError::WidgetSettings {
                kind: self.kind.clone(),
                source,
            })
    }
}

impl Default for BoardSpec {
    fn default() -> Self {
        toml::from_str(DEFAULT_BOARD).unwrap_or_else(|e| {
            log::error!("the built-in board does not parse: {e}");
            Self {
                grid: GridSpec::default(),
                widgets: Vec::new(),
            }
        })
    }
}

impl BoardSpec {
    /// The saved board, or the built-in one when there is none. A malformed
    /// file is reported and left alone, so fixing a typo does not cost the
    /// rest of the arrangement.
    pub fn load(path: &Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&contents).unwrap_or_else(|e| {
            log::warn!("{}: {e}; showing the default board", path.display());
            Self::default()
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        Ok(atomic::write(path, &toml::to_string_pretty(self)?)?)
    }

    pub fn path() -> Result<PathBuf> {
        Ok(xdg::state_dir()?.join(FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_board_fits_its_centred_block_without_overlaps() {
        let board: BoardSpec = toml::from_str(DEFAULT_BOARD).expect("the built-in board parses");
        assert!(!board.widgets.is_empty());
        for (index, widget) in board.widgets.iter().enumerate() {
            let area = widget.area();
            assert!(area.column >= 0 && area.row >= 0, "{widget:?}");
            assert!(
                area.column + area.columns as i32 <= board.grid.columns as i32,
                "{widget:?}"
            );
            assert!(
                area.row + area.rows as i32 <= board.grid.rows as i32,
                "{widget:?}"
            );
            for other in &board.widgets[index + 1..] {
                assert!(!area.overlaps(&other.area()), "{widget:?} {other:?}");
            }
        }
    }

    #[test]
    fn an_unknown_kind_round_trips_with_its_settings() {
        let source = "[[widget]]\nkind = \"mystery\"\ncolumn = 1\nrow = 2\n[widget.settings]\nsource = \"x\"\n";
        let board: BoardSpec = toml::from_str(source).expect("parses");
        let saved = toml::to_string_pretty(&board).expect("encodes");
        let again: BoardSpec = toml::from_str(&saved).expect("parses again");
        assert_eq!(board, again);
    }
}
