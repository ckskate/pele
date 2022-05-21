use tokio::{self, sync};
use bluer::gatt::remote::Characteristic;

use crate::{
    utils::HeatingCoolingState,
    bluetooth_service::Message,
};


pub struct HeatAirStateWorker {
    heat_or_air_enabled_char: Characteristic, 
    start_heat_char: Characteristic,
    stop_heat_char: Characteristic,
    start_air_char: Characteristic,
    stop_air_char: Characteristic,
    rx: sync::mpsc::Receiver<Message>,
    curr_heating_cooling_state: HeatingCoolingState,
    is_freshly_set: bool,
}

// handles reading the heat/air state from the volcano
impl HeatAirStateWorker {

    pub fn new(heat_or_air_enabled_char: Characteristic, 
               start_heat_char: Characteristic,
               stop_heat_char: Characteristic,
               start_air_char: Characteristic,
               stop_air_char: Characteristic,
               rx: sync::mpsc::Receiver<Message>) -> HeatAirStateWorker {
        HeatAirStateWorker { 
            heat_or_air_enabled_char,
            start_heat_char,
            stop_heat_char,
            start_air_char,
            stop_air_char,
            rx, 
            curr_heating_cooling_state: HeatingCoolingState::Off,
            is_freshly_set: false,
        }
    }

    pub async fn run_loop(&mut self) {
        while let Some(message) = self.rx.recv().await {
            match message {
                Message::GetHeatAirState { resp_tx } => {
                    // if this is the first read since updating the
                    // val, trust the written val, bc this one takes
                    // a little while to catch up
                    if self.is_freshly_set {
                        self.is_freshly_set = false;
                        let _ = resp_tx.send(self.curr_heating_cooling_state);
                        continue;
                    }

                    let heat_air_state = self.get_heat_air_state()
                                             .await
                                             .unwrap_or(self.curr_heating_cooling_state);
                    self.curr_heating_cooling_state = heat_air_state;
                    let _ = resp_tx.send(heat_air_state);
                },
                Message::SetHeatAirState { state, resp_tx } => {
                    self.curr_heating_cooling_state = state;
                    self.is_freshly_set = true;
                    let success = self.write_heat_air_state(state).await;
                    let _ = resp_tx.send(success);
                },
                _ => (),
            }
        }
    }

    async fn get_heat_air_state(&self) -> Option<HeatingCoolingState> {
        return self.heat_or_air_enabled_char
                   .read()
                   .await
                   .ok()
                   .map(|raw_state| {
                       HeatingCoolingState::from_device_val(raw_state)
                   });
    }

    async fn write_heat_air_state(&self,
                                 state: HeatingCoolingState) -> bluer::Result<()> 
    {
        match state {
            HeatingCoolingState::Heating => {
                let _ = tokio::join!(self.start_heat_char.write(&[1]),
                                    self.stop_air_char.write(&[1]));
            },
            HeatingCoolingState::Cooling => {
                let _ = tokio::join!(self.start_heat_char.write(&[1]),
                                    self.start_air_char.write(&[1]));
            },
            HeatingCoolingState::Off => {
                let _ = tokio::join!(self.stop_heat_char.write(&[1]),
                                    self.stop_air_char.write(&[1]));
            },
        };
        Ok(())
    }
}
