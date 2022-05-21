use tokio::{self, sync};
use bluer::gatt::remote::Characteristic;

use crate::{
    bluetooth_service::Message,
    utils::Temperature,
};

// handles reading the current temp

pub struct CurrTempWorker {
    curr_temp_char: Characteristic,
    rx: sync::mpsc::Receiver<Message>,
    prev_read_val: Temperature,
}

impl CurrTempWorker {

    pub fn new(curr_temp_char: Characteristic, 
               rx: sync::mpsc::Receiver<Message>) -> CurrTempWorker {
        CurrTempWorker {
            curr_temp_char, 
            rx, 
            prev_read_val: Temperature::zero(),
        }
    }

    async fn get_curr_temp(&self) -> Option<Temperature> {
        return self.curr_temp_char
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
                Message::GetCurrTemp { resp_tx } => {
                    let curr_temp = self.get_curr_temp()
                                        .await
                                        .unwrap_or(self.prev_read_val);
                    self.prev_read_val = curr_temp;
                    let _ = resp_tx.send(curr_temp);
                },
                _ => (),
            }
        }
    }
}

