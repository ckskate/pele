use tokio::{self, sync};
use hap::futures::StreamExt;
use bluer::{
    AdapterEvent,
    Device
};

use crate::{
    Result,
    utils::{Temperature, HeatingCoolingState},
    bluetooth_service::worker::Worker,
};

mod worker;
mod targ_temp_worker;
mod curr_temp_worker;
mod heat_air_worker;


#[derive(Debug)]
pub enum Message {
    GetCurrTemp { resp_tx: sync::oneshot::Sender<Temperature> },
    GetTargTemp  { resp_tx: sync::oneshot::Sender<Temperature> },
    GetHeatAirState { resp_tx: sync::oneshot::Sender<HeatingCoolingState> },
    SetTargTemp { temp: Temperature, resp_tx: sync::oneshot::Sender<bluer::Result<()>> },
    SetHeatAirState { state: HeatingCoolingState, resp_tx: sync::oneshot::Sender<bluer::Result<()>> },
    Disconnect { resp_tx: sync::oneshot::Sender<bluer::Result<()>> },
}


pub struct BluetoothService {
    tx: sync::mpsc::Sender<Message>,
}

impl BluetoothService {

    pub async fn new() -> Result<BluetoothService> {
        let volcano = Self::discover_volcano()
                            .await?
                            .expect("couldn't find the volcano!");
        println!("{:?}", volcano.name().await);
        let (tx, rx) = sync::mpsc::channel(32);
        let mut worker = Worker::new(volcano, rx).await?;
        tokio::spawn(async move {
            worker.run_loop().await;
        });
        let service = BluetoothService { tx };
        Ok(service)
    }

    pub async fn disconnect(&self) -> Result<()> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::Disconnect { resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::new(err)),
        }
    }

    pub async fn get_curr_temp(&self) -> Option<Temperature> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::GetCurrTemp { resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(curr_temp) => Some(curr_temp),
            Err(_) => None,
        }
    }

    pub async fn get_targ_temp(&self) -> Option<Temperature> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::GetTargTemp { resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(targ_temp) => Some(targ_temp),
            Err(_) => None,
        }
    }

    pub async fn set_temp(&self, temp: Temperature) -> Option<()> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::SetTargTemp { temp, resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(_) => Some(()),
            Err(_) => None,
        }
    }

    pub async fn get_curr_heat_air_state(&self) -> Option<HeatingCoolingState> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::GetHeatAirState { resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(heat_air_state) => Some(heat_air_state),
            Err(_) => None,
        }
    }

    pub async fn set_curr_heat_air_state(&self, state: HeatingCoolingState) -> Option<()> {
        let (resp_tx, resp_rx) = sync::oneshot::channel();
        let message = Message::SetHeatAirState { state, resp_tx };
        self.tx.send(message).await.expect("couldn't send message");
        match resp_rx.await {
            Ok(_) => Some(()),
            Err(_) => None,
        }
    }
}

impl BluetoothService {

    async fn discover_volcano() -> Result<Option<Device>> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;
        {
            let mut device_stream = adapter.discover_devices().await?;
            while let Some(evt) = device_stream.next().await {
                match evt {
                    AdapterEvent::DeviceAdded(addr) => {
                        let device = adapter.device(addr)?;
                        if device.name()
                                 .await?
                                 .unwrap_or("".into())
                                 .contains("VOLCANO") {
                            return Ok(Some(device));
                        }
                        
                    },
                    _ => (),
                }
            }
        }
        Ok(None)
    }
}
