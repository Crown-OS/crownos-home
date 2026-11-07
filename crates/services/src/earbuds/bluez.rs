use std::collections::HashMap;

use tokio::sync::OnceCell;
use zbus::{
    Connection,
    fdo::{ManagedObjects, ObjectManagerProxy},
    zvariant::{OwnedValue, Value},
};

use super::Earbuds;

const BLUEZ: &str = "org.bluez";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";
const EARBUD_ICONS: [&str; 2] = ["audio-headset", "audio-headphones"];

static SYSTEM_BUS: OnceCell<Connection> = OnceCell::const_new();

type Properties = HashMap<String, OwnedValue>;

fn text<'a>(properties: &'a Properties, key: &str) -> Option<&'a str> {
    match &**properties.get(key)? {
        Value::Str(text) => Some(text.as_str()),
        _ => None,
    }
}

fn flag(properties: &Properties, key: &str) -> bool {
    matches!(
        properties.get(key).map(|value| &**value),
        Some(Value::Bool(true))
    )
}

fn percent(properties: &Properties) -> Option<u8> {
    match &**properties.get("Percentage")? {
        Value::U8(percent) => Some(*percent),
        _ => None,
    }
}

fn earbuds(objects: ManagedObjects) -> Vec<Earbuds> {
    objects
        .into_values()
        .filter_map(|interfaces| {
            let mut by_name: HashMap<&str, &Properties> = interfaces
                .iter()
                .map(|(name, properties)| (name.as_str(), properties))
                .collect();
            let device = by_name.remove(DEVICE)?;
            let icon = text(device, "Icon")?;
            EARBUD_ICONS.contains(&icon).then(|| Earbuds {
                name: text(device, "Alias")
                    .or_else(|| text(device, "Name"))
                    .unwrap_or_default()
                    .to_owned(),
                connected: flag(device, "Connected"),
                battery: by_name.get(BATTERY).and_then(|battery| percent(battery)),
            })
        })
        .collect()
}

/// Every pair of earbuds BlueZ knows about.
pub async fn paired() -> zbus::Result<Vec<Earbuds>> {
    let bus = SYSTEM_BUS.get_or_try_init(Connection::system).await?;
    let manager = ObjectManagerProxy::builder(bus)
        .destination(BLUEZ)?
        .path("/")?
        .build()
        .await?;
    Ok(earbuds(manager.get_managed_objects().await?))
}
