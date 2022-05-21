use tokio;
use std::{error::Error, sync::Arc};
use hap::{
    server::{IpServer, Server},
    storage::FileStorage,
};

mod bluetooth_service;
mod utils;
mod volcano_factory;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

use crate::bluetooth_service::BluetoothService;


#[tokio::main]
async fn main() -> hap::Result<()> {
    let service = Arc::new(BluetoothService::new()
                                .await
                                .expect("shit's fucked yo!"));
    let volcano = volcano_factory::create_volcano(Arc::clone(&service)).unwrap();
    let mut storage = FileStorage::current_dir().await?;
    let config = volcano_factory::get_volcano_config_from_storage(&mut storage)
                                 .await.unwrap();
    let server = IpServer::new(config, storage).await?;
    let volcano = server.add_accessory(volcano).await?;

    let background_service = Arc::clone(&service);
    tokio::spawn(async move {
        volcano_factory::char_update_loop(background_service, volcano).await;
    });

    let handle = server.run_handle();
//    std::env::set_var("RUST_LOG", "hap=debug");
//    env_logger::init();

    let _ = handle.await;
    let _ = service.disconnect().await;
    println!("goodbye!");
    Ok(())
}
