use android_hardware_thermal::aidl::android::hardware::thermal::{
    CoolingDevice::CoolingDevice, CoolingType::CoolingType,
};
use log::debug;

use std::str::FromStr;

use crate::utils;

#[derive(Default)]
pub struct CoolDevice {
    dir_name: String,
    dev: CoolingDevice,
}

impl CoolDevice {
    pub fn new(name: &str) -> Self {
        let mut cool_dev = Self {
            dir_name: name.to_string(),
            ..Default::default()
        };

        cool_dev.set_type_and_name();

        cool_dev
    }

    fn get_file(&self, file_name: &str) -> String {
        format!("/sys/class/thermal/{}/{}", self.dir_name, file_name)
    }

    fn set_type_and_name(&mut self) {
        let type_names = [
            ("Fan", "fan", CoolingType::FAN),
            ("Processor", "processor", CoolingType::CPU),
        ];
        let path = self.get_file("type");
        let dev_type = utils::read_sysfs_file(&path).unwrap();

        debug!("{}: \"{}\"", path, dev_type);

        // Unfortunately there's no UNKNOWN value for cooling type, this can lead to confusion...
        let mut r#type = CoolingType::FAN;
        for (type_cap, type_lower, cooling_type) in type_names {
            if dev_type == type_cap || dev_type == type_lower {
                r#type = cooling_type;
                break;
            }
        }

        self.dev.name = dev_type;
        self.dev.r#type = r#type;
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
