use tokio::{self, sync};
use bluer::gatt::remote::Characteristic;

use crate::{
    utils::Temperature,
    bluetooth_service::Message,
};

// handles reading/writing the target temp

pub struct TargTempWorker {
    targ_temp_char: Characteristic,
    rx: sync::mpsc::Receiver<Message>,
    prev_read_val: Temperature,
    is_freshly_set: bool,
}

impl TargTempWorker {

    pub fn new(targ_temp_char: Characteristic, 
               rx: sync::mpsc::Receiver<Message>) -> TargTempWorker {
        TargTempWorker { 
            targ_temp_char,
            rx, 
            prev_read_val: Temperature::zero(),
            is_freshly_set: false,
        }
    }

    async fn write_targ_temp(&self, temp: Temperature) -> bluer::Result<()> {
        let device_val = temp.device_val();
        println!("writing targ temp val: {:?}", device_val);
        self.targ_temp_char
            .write(&device_val)
            .await
    }
    
    async fn get_targ_temp(&self) -> Option<Temperature> {
        return self.targ_temp_char
                    .read()
                    .await
                    .ok()
                    .map(|raw_temp| {
                        Temperature::from_device_val(raw_temp)
                    });
    }

    pub async fn run_loop(&mut self) {
        while let Some(message) = self.rx.recv().await {
            match message {
                Message::GetTargTemp { resp_tx } => {
                    if self.is_freshly_set {
                        self.is_freshly_set = false;
                        let _ = resp_tx.send(self.prev_read_val);
                        continue;
                    }

                    let targ_temp = self.get_targ_temp()
                                        .await
                                        .unwrap_or(self.prev_read_val);
                    self.prev_read_val = targ_temp;
                    let _ = resp_tx.send(targ_temp);
                },
                Message::SetTargTemp { temp, resp_tx } => {
                    self.is_freshly_set = true;
                    self.prev_read_val = temp;
                    let success = self.write_targ_temp(temp)
                                      .await;
                    let _ = resp_tx.send(success);
                },
                _ => (),
            }
        }
    }
}

