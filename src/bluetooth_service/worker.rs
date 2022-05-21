use tokio::{self, sync};
use bluer::{
    gatt::remote::Characteristic,
    Device
};

use crate::{
    Result,
    bluetooth_service::{
        heat_air_worker::HeatAirStateWorker,
        targ_temp_worker::TargTempWorker,
        curr_temp_worker::CurrTempWorker,
        Message,
    },
};

const SERVICE1_UUID: &str = "10100000-5354-4f52-5a26-4249434b454c";
const SERVICE2_UUID: &str = "10110000-5354-4f52-5a26-4249434b454c";
const FIRMWARE_CHAR_UUID: &str = "10100005-5354-4f52-5a26-4249434b454c";
const SERIAL_CHAR_UUID: &str = "10100008-5354-4f52-5a26-4249434b454c";
const MODEL_CHAR_UUID: &str = "10100007-5354-4f52-5a26-4249434b454c";
const CURR_TEMP_CHAR_UUID: &str = "10110001-5354-4f52-5a26-4249434b454c";
const TARG_TEMP_CHAR_UUID: &str = "10110003-5354-4f52-5a26-4249434b454c";
const IS_HEATORAIR_ENABLED_CHAR_UUID: &str = "1010000c-5354-4f52-5a26-4249434b454c";
const START_HEAT_CHAR_UUID: &str = "1011000f-5354-4f52-5a26-4249434b454c";
const STOP_HEAT_CHAR_UUID: &str = "10110010-5354-4f52-5a26-4249434b454c";
const START_AIR_CHAR_UUID: &str = "10110013-5354-4f52-5a26-4249434b454c";
const STOP_AIR_CHAR_UUID: &str = "10110014-5354-4f52-5a26-4249434b454c";

#[allow(dead_code)]
pub struct Worker {
    volcano: Device,
    firmware_char: Characteristic,
    model_char: Characteristic,
    serial_char: Characteristic,
    rx: sync::mpsc::Receiver<Message>,
    curr_temp_tx: sync::mpsc::Sender<Message>,
    targ_temp_tx: sync::mpsc::Sender<Message>,
    heat_air_state_tx: sync::mpsc::Sender<Message>,
}

impl Worker {
    pub async fn run_loop(&mut self) {
        while let Some(message) = self.rx.recv().await {
            Self::connect_to_volcano_if_needed(&self.volcano)
                .await
                .expect("can't connect to the volcano");

            match message {
                Message::GetCurrTemp { resp_tx } => {
                    let _ = self.curr_temp_tx
                                .send(Message::GetCurrTemp { resp_tx })
                                .await;
                },
                Message::GetTargTemp { resp_tx } => {
                    let _ = self.targ_temp_tx
                                .send(Message::GetTargTemp { resp_tx })
                                .await;
                },
                Message::SetTargTemp { temp, resp_tx } => {
                    let _ = self.targ_temp_tx
                                .send(Message::SetTargTemp { temp, resp_tx })
                                .await;
                },
                Message::GetHeatAirState { resp_tx } => {
                    let _ = self.heat_air_state_tx
                                .send(Message::GetHeatAirState { resp_tx })
                                .await;
                },
                Message::SetHeatAirState { state, resp_tx } => {
                    let _ = self.heat_air_state_tx
                                .send(Message::SetHeatAirState { state, resp_tx })
                                .await;
                },
                Message::Disconnect { resp_tx } => {
                    let success = self.disconnect_from_volcano_if_needed()
                                      .await;
                    let _ = resp_tx.send(success);
                    println!("closing run loop");
                    return;
                },
            }
        }
    }

    pub async fn new(volcano: Device,
                 rx: sync::mpsc::Receiver<Message>) -> Result<Worker> {
        let mut firmware_char: Option<Characteristic> = None;
        let mut model_char: Option<Characteristic> = None;
        let mut serial_char: Option<Characteristic> = None;
        let mut curr_temp_char: Option<Characteristic> = None;
        let mut targ_temp_char: Option<Characteristic> = None;
        let mut heat_or_air_enabled_char: Option<Characteristic> = None;
        let mut start_heat_char: Option<Characteristic> = None;
        let mut stop_heat_char: Option<Characteristic> = None;
        let mut start_air_char: Option<Characteristic> = None;
        let mut stop_air_char: Option<Characteristic> = None;

        // make the initial connection and wait a little for it to settle
        Worker::connect_to_volcano_if_needed(&volcano).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // discover the characteristics we care about
        for service in volcano.services().await? {
            let service_uuid_string = service.uuid().await?.to_string();
            if !service_uuid_string.eq(&SERVICE1_UUID) 
                && !service_uuid_string.eq(&SERVICE2_UUID) {
                continue
            }
            for characterictic in service.characteristics().await? {
                let char_uuid_string = characterictic.uuid().await?.to_string();
                match char_uuid_string.as_str() {
                    FIRMWARE_CHAR_UUID => firmware_char = Some(characterictic),
                    MODEL_CHAR_UUID => model_char = Some(characterictic),
                    SERIAL_CHAR_UUID => serial_char = Some(characterictic),
                    CURR_TEMP_CHAR_UUID => curr_temp_char = Some(characterictic),
                    TARG_TEMP_CHAR_UUID => targ_temp_char = Some(characterictic),
                    IS_HEATORAIR_ENABLED_CHAR_UUID => heat_or_air_enabled_char = Some(characterictic),
                    START_HEAT_CHAR_UUID => start_heat_char = Some(characterictic),
                    STOP_HEAT_CHAR_UUID => stop_heat_char = Some(characterictic),
                    START_AIR_CHAR_UUID => start_air_char = Some(characterictic),
                    STOP_AIR_CHAR_UUID => stop_air_char = Some(characterictic),
                    _ => (),
                };
            }
        }
        
        // spin up the targ temp worker
        let (targ_temp_tx, targ_temp_rx) = sync::mpsc::channel(32);
        let mut targ_temp_worker = TargTempWorker::new(targ_temp_char.unwrap(),
                                                       targ_temp_rx);
        tokio::spawn(async move {
            targ_temp_worker.run_loop().await;
        });

        // heat air state reader worker
        let (heat_air_tx, heat_air_rx) = sync::mpsc::channel(32);
        let mut heat_air_worker = HeatAirStateWorker::new(
                                    heat_or_air_enabled_char.unwrap(),
                                    start_heat_char.unwrap(),
                                    stop_heat_char.unwrap(),
                                    start_air_char.unwrap(),
                                    stop_air_char.unwrap(),
                                    heat_air_rx);
        tokio::spawn(async move {
            heat_air_worker.run_loop().await;
        });
        
        // and also the curr temp worker
        let (curr_temp_tx, curr_temp_rx) = sync::mpsc::channel(32);
        let mut curr_temp_worker = CurrTempWorker::new(curr_temp_char.unwrap(),
                                                       curr_temp_rx);
        tokio::spawn(async move {
            curr_temp_worker.run_loop().await;
        });
        
        Ok(Worker {
            volcano,
            firmware_char: firmware_char.unwrap(),
            model_char: model_char.unwrap(),
            serial_char: serial_char.unwrap(),
            rx,
            curr_temp_tx,
            targ_temp_tx,
            heat_air_state_tx: heat_air_tx,
        })
    }

    async fn connect_to_volcano_if_needed(volcano: &Device) -> Result<()> {
        if !volcano.is_connected().await? {
            let mut retries = 2;
            loop {
                match volcano.connect().await {
                    Ok(()) => break,
                    Err(err) if retries > 0 => {
                        println!("Connect error: {}", &err);
                        retries -= 1;
                    }
                    Err(err) => return Err(Box::new(err)),
                }
            }
            return Ok(());
        }
        Ok(())
    }

    async fn disconnect_from_volcano_if_needed(&self) -> bluer::Result<()> {
        if !self.volcano.is_connected().await? {
            return Ok(());
        }
        self.volcano.disconnect().await
    }
}
