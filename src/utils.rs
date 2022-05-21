use bytes::{Bytes,
            Buf,
            BufMut};


const HEAT_ENABLED_BYTE: u8 = 0x23;
const AIR_ENABLED_BYTE: u8 = 0x03;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum HeatingCoolingState {
    Off,
    Heating,
    Cooling,
}

impl HeatingCoolingState {

    pub fn from_device_val(vec: Vec<u8>) -> HeatingCoolingState {
        if vec.len() < 2 {
            return HeatingCoolingState::Off;
        }

        let is_heat_on = vec[0] == HEAT_ENABLED_BYTE;
        let is_air_on = (vec[1] >> 4) == AIR_ENABLED_BYTE;

        if is_air_on && is_heat_on {
            return HeatingCoolingState::Cooling;
        } else if is_heat_on {
            return HeatingCoolingState::Heating;
        }

        HeatingCoolingState::Off
    }

    pub fn from_homekit_val(val: u8) -> HeatingCoolingState {
        match val {
            1 => HeatingCoolingState::Heating,
            2 => HeatingCoolingState::Cooling,
            _ => HeatingCoolingState::Off,
        }
    }

    pub fn homekit_val(&self) -> u8 {
        match self {
            HeatingCoolingState::Off => 0,
            HeatingCoolingState::Heating => 1,
            HeatingCoolingState::Cooling => 2,
        }
    }
}

const TEMP_OFFSET_C: f32 = 172.2222222;
const TARG_MIN_TEMP_C: f32 = 10.0;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Temperature {
    cel_val: f32
}

impl Temperature {

    pub fn zero() -> Temperature {
        Temperature { cel_val: 0.0 }
    }

    pub fn from_device_val(vec: Vec<u8>) -> Temperature {
        let temp_val = Bytes::from(vec).get_i16_le();
        Temperature { cel_val: f32::from(temp_val) / 10.0 }
    }

    pub fn from_homekit_val(raw_val: f32, should_scale: bool) -> Temperature {
        // gotta add the temp offset val
        let scale_factor = if should_scale { TEMP_OFFSET_C } 
                           else { 0.0 };
        let scaled_val = raw_val + scale_factor;
        Temperature { cel_val: scaled_val }
    }

    pub fn device_val(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);
        let int_val = (self.cel_val * 10.0).round() as i16;
        bytes.put_i16_le(int_val);
        bytes
    }

    pub fn homekit_val(&self, should_scale: bool) -> f32 {
        let scale_factor = if should_scale { TEMP_OFFSET_C } 
                           else { 0.0 };
        let scaled_temp = self.cel_val - scale_factor;
        if scaled_temp < TARG_MIN_TEMP_C {
            return TARG_MIN_TEMP_C;
        }
        scaled_temp
    }
}
