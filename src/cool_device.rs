use android_hardware_thermal::aidl::android::hardware::thermal::{
    CoolingDevice::CoolingDevice, CoolingType::CoolingType,
};

use std::str::FromStr;

use crate::config::CoolingDeviceConfig;
use crate::utils;

#[derive(Default)]
pub struct CoolDevice {
    dir_name: String,
    dev: CoolingDevice,
}

impl CoolDevice {

    pub fn from_config(name: &str, config: &CoolingDeviceConfig) -> Self {
        let mut cool_dev = Self {
            dir_name: name.to_string(),
            ..Default::default()
        };

        // Set type and name from config
        let cooling_type = config.get_cooling_type();
        cool_dev.dev.name = config.name.clone();
        cool_dev.dev.r#type = cooling_type;

        cool_dev
    }

    fn get_file(&self, file_name: &str) -> String {
        format!("/sys/class/thermal/{}/{}", self.dir_name, file_name)
    }



    fn update_state(&mut self) {
        let path = self.get_file("cur_state");
        let value = utils::read_sysfs_file(&path).unwrap();

        self.dev.value = i64::from_str(&value).unwrap();
    }

    pub fn updated_clone(&self) -> CoolingDevice {
        let mut dev = self.clone();

        dev.update_state();

        dev.get_cooling_device()
    }

    pub fn get_cooling_device(self) -> CoolingDevice {
        self.dev
    }

    pub fn get_type(&self) -> CoolingType {
        self.dev.r#type
    }
}

impl Clone for CoolDevice {
    fn clone(&self) -> Self {
        let dev = &self.dev;

        Self {
            dir_name: self.dir_name.clone(),
            dev: CoolingDevice {
                r#type: dev.r#type,
                name: dev.name.clone(),
                value: dev.value,
                powerLimitMw: dev.powerLimitMw,
                powerMw: dev.powerMw,
                timeWindowMs: dev.timeWindowMs,
            },
        }
    }
}
