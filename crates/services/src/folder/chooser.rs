//! Picking files through the desktop portal's file chooser.

use std::{
    collections::HashMap,
    path::PathBuf,
    pin::pin,
    sync::atomic::{AtomicU32, Ordering},
};

use zbus::{
    Connection, Proxy,
    export::futures_core::Stream,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

use crownhome_util::uri;

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const CHOOSER: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
const ACCEPTED: u32 = 0;

static NEXT_TOKEN: AtomicU32 = AtomicU32::new(0);

/// Where the portal will answer a request sent with `token`.
fn request_path(bus: &Connection, token: &str) -> String {
    let sender = bus
        .unique_name()
        .map(|name| name.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    format!("{PORTAL_PATH}/request/{sender}/{token}")
}

fn chosen_paths(results: &HashMap<String, OwnedValue>) -> Vec<PathBuf> {
    let Some(Value::Array(uris)) = results.get("uris").map(|value| &**value) else {
        return Vec::new();
    };
    uris.iter()
        .filter_map(|uri| match uri {
            Value::Str(uri) => uri::file_path(uri.as_str()),
            _ => None,
        })
        .collect()
}

/// The files the person picks, none if they cancel.
pub async fn pick_files(title: &str) -> zbus::Result<Vec<PathBuf>> {
    let bus = Connection::session().await?;
    let token = format!("crownhome{}", NEXT_TOKEN.fetch_add(1, Ordering::Relaxed));
    let request = Proxy::new(&bus, PORTAL, request_path(&bus, &token), REQUEST).await?;
    let mut responses = pin!(request.receive_signal("Response").await?);
    let chooser = Proxy::new(&bus, PORTAL, PORTAL_PATH, CHOOSER).await?;
    let options: HashMap<&str, Value<'_>> = HashMap::from([
        ("handle_token", Value::from(token.as_str())),
        ("multiple", Value::from(true)),
    ]);
    let _: OwnedObjectPath = chooser.call("OpenFile", &("", title, options)).await?;
    let Some(answer) = std::future::poll_fn(|cx| responses.as_mut().poll_next(cx)).await else {
        return Ok(Vec::new());
    };
    let (response, results): (u32, HashMap<String, OwnedValue>) = answer.body().deserialize()?;
    Ok(if response == ACCEPTED {
        chosen_paths(&results)
    } else {
        Vec::new()
    })
}
