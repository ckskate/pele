use std::sync::Arc;
use hap::{
    accessory::{
        thermostat::ThermostatAccessory,
        AccessoryCategory,
        AccessoryInformation,
        HapAccessory,
    },
    storage::{FileStorage, Storage},
    futures::{FutureExt, lock::Mutex},
    characteristic::AsyncCharacteristicCallbacks, 
    serde_json::json,
    HapType,
    Config,
    MacAddress,
    Pin,
};

use crate::{
    bluetooth_service::BluetoothService,
    utils::{Temperature, HeatingCoolingState},
    Result,
};


const VOLCANO_NAME: &str = "Volcano";

pub async fn char_update_loop(bluetooth_service: Arc<BluetoothService>,
                          volcano_container: Arc<Mutex<Box<(dyn HapAccessory + 'static)>>>) {
    loop {
        // wait 2 secs
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // get access to the shared volcano
        let mut volcano = volcano_container.lock()
                                           .await;
        let volcano = volcano.get_mut_service(HapType::Thermostat)
                             .unwrap();
        
        // read the states
        let (heat_state_option,
             curr_temp_option,
             targ_temp_option) = tokio::join!(
                                    bluetooth_service.get_curr_heat_air_state(),
                                    bluetooth_service.get_curr_temp(),
                                    bluetooth_service.get_targ_temp()
                                );

        // update curr heat state
        {
            let curr_heat_char = volcano.get_mut_characteristic(HapType::CurrentHeatingCoolingState)
                                        .unwrap();
            if let Some(heat_state) = heat_state_option {
                let curr_heat_val = json!(heat_state.homekit_val());
                println!("background write homekit curr heat: {:?}", curr_heat_val);
                let _ = curr_heat_char.set_value(curr_heat_val).await;
            }
        }
        // update target heat state
        {
            let targ_heat_char = volcano.get_mut_characteristic(HapType::TargetHeatingCoolingState)
                                        .unwrap();
            if let Some(heat_state) = heat_state_option {
                let targ_heat_val = json!(heat_state.homekit_val());
                println!("background write homekit targ heat: {:?}", targ_heat_val);
                let _ = targ_heat_char.set_value(targ_heat_val).await;
            }
        }
        // update current temp state
        {
            let curr_temp_char = volcano.get_mut_characteristic(HapType::CurrentTemperature)
                                        .unwrap();
            if let Some(curr_temp) = curr_temp_option {
                let curr_temp_val = json!(curr_temp.homekit_val(true));
                println!("background write homekit curr temp: {:?}", curr_temp_val);
                let _ = curr_temp_char.set_value(curr_temp_val).await;
            }
        }
        // update target temp state
        {
            let targ_temp_char = volcano.get_mut_characteristic(HapType::TargetTemperature)
                                        .unwrap();
            if let Some(targ_temp) = targ_temp_option {
                let targ_temp_val = json!(targ_temp.homekit_val(true));
                println!("background write homekit targ temp: {:?}", targ_temp_val);
                let _ = targ_temp_char.set_value(targ_temp_val).await;
            }
        }
    }
}

pub fn create_volcano(bluetooth_service: Arc<BluetoothService>,
                      ) -> Result<ThermostatAccessory> {
    let mut volcano = ThermostatAccessory::new(1,
                                           AccessoryInformation {
                                                name: VOLCANO_NAME.into(),
                                                ..Default::default() 
                                           })?;

    let local_srv_1 = Arc::clone(&bluetooth_service);
    volcano.thermostat
           .target_heating_cooling_state
           .on_update_async(Some(move |old_val: u8, new_val: u8| {
            let local_srv_tst = Arc::clone(&local_srv_1);
            async move {
                if old_val == new_val { return Ok(()); }
                let local_srv = Arc::clone(&local_srv_tst);
                let new_hc_state = HeatingCoolingState::from_homekit_val(new_val);
                let _ = local_srv.set_curr_heat_air_state(new_hc_state)
                                 .await;
                Ok(())
            }.boxed()
    }));

    let local_srv_1 = Arc::clone(&bluetooth_service);
    volcano.thermostat
           .target_temperature
           .on_update_async(Some(move |old_val: f32, new_val: f32| {
            let local_srv_tst = Arc::clone(&local_srv_1);
            async move {
                let local_srv = Arc::clone(&local_srv_tst);
                if old_val == new_val {
                    return Ok(());
                }
                let curr_device_temp = local_srv.get_targ_temp()
                                                .await
                                                .unwrap_or(Temperature::from_homekit_val(old_val, 
                                                                                         true));
                let new_temp = Temperature::from_homekit_val(new_val, true);

                // only write it if these match (device isn't updating itself)
                if curr_device_temp.homekit_val(true) == old_val {
                    let _ = local_srv.set_temp(new_temp)
                                     .await;
                }
                Ok(())
            }.boxed()
    }));

    Ok(volcano)
}

pub async fn get_volcano_config_from_storage(storage: &mut FileStorage) -> Result<Config> {
    match storage.load_config().await {
        Ok(mut config) => {
            config.redetermine_local_ip();
            storage.save_config(&config).await?;
            Ok(config)
        },
        Err(_) => {
            let config = Config {
                pin: Pin::new([7, 1, 0, 6, 9, 4, 2, 0])?,
                name: VOLCANO_NAME.into(),
                device_id: MacAddress::new([20, 20, 30, 40, 50, 60]),
                category: AccessoryCategory::Thermostat,
                ..Default::default()
            };
            storage.save_config(&config).await?;
            Ok(config)
        },
    }
}

