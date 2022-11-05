use async_minecraft_ping::{ConnectionConfig, ServerDescription, StatusResponse};
use futures::future::join_all;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Ip {
    #[serde(rename = "ip")]
    address: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let ips = serde_json::from_str::<Vec<Ip>>(include_str!("../scan.json")).unwrap();
    log::info!("Found {} ips", ips.len());

    let chunks = ips.chunks(ips.len() / 64).map(|chunk| chunk.to_vec());
    let handles = chunks
        .into_iter()
        .map(|chunk| tokio::spawn(async move { process_chunk(chunk).await }))
        .collect::<Vec<_>>();
    join_all(handles).await;

    Ok(())
}

async fn ping_server(address: String) -> anyhow::Result<StatusResponse> {
    let config = ConnectionConfig::build(address);
    let connection = config.connect().await?;

    let status = connection.status().await?;

    Ok(status.status)
}

async fn process_chunk(chunk: Vec<Ip>) {
    log::info!("Processing chunk of {} ips", chunk.len());

    for ip in chunk {
        log::trace!("Checking {}", ip.address);
        let Ok(status) = ping_server(ip.address.clone()).await else {
            continue;
        };

        if status.players.max != 35 {
            log::trace!("Invalid player count ({})", status.players.max);
            continue;
        }

        let ServerDescription::Object { text: motd } = &status.description else {
            log::trace!("Invalid motd");
            continue;
        };

        if motd.is_empty() || motd != "A Minecraft Server" {
            log::trace!("Invalid motd: {}", motd);
            continue;
        }

        log::info!("{} - {}", ip.address, motd);
        log::info!("{} slots available", status.players.max);
        for player in status.players.sample.unwrap_or_default() {
            log::info!("{} - {}", player.name, player.id);
        }
    }
}
