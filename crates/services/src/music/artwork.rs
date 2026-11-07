use std::{path::Path, sync::LazyLock};

use crownhome_util::raster::{self, Pixels};

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(crownos_accounts::agent);

fn download(url: &str) -> Option<Vec<u8>> {
    if let Some(path) = url.strip_prefix("file://") {
        return std::fs::read(Path::new(path)).ok();
    }
    AGENT
        .get(url)
        .call()
        .and_then(|mut response| response.body_mut().read_to_vec())
        .map_err(|e| log::debug!("cover {url}: {e}"))
        .ok()
}

/// The cover at `url`, square, `edge` pixels wide.
pub async fn fetch_cover(url: String, edge: u32) -> Option<Pixels> {
    tokio::task::spawn_blocking(move || raster::cover_square(&download(&url)?, edge))
        .await
        .ok()
        .flatten()
}
