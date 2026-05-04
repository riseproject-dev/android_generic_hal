use android_hardware_thermal::aidl::android::hardware::thermal::{
    Temperature::Temperature, TemperatureThreshold::TemperatureThreshold,
    TemperatureType::TemperatureType, ThrottlingSeverity::ThrottlingSeverity,
};

use std::str::FromStr;

use crate::config::ThermalZoneConfig;
use crate::utils;

#[derive(Default)]
pub struct ThermalZone {
    dir_name: String,
    threshold: TemperatureThreshold,
    temp: Temperature,
}

impl ThermalZone {

    pub fn from_config(name: &str, config: &ThermalZoneConfig) -> Self {
        let mut tz = Self {
            dir_name: name.to_string(),
            ..Default::default()
        };

        // Set type and name from config
        let temp_type = config.get_temperature_type();
        tz.threshold.name = config.name.clone();
        tz.temp.name = config.name.clone();
        tz.threshold.r#type = temp_type;
        tz.temp.r#type = temp_type;
        
        // Set hot throttling thresholds from config
        let mut thresholds = vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
        thresholds[Self::severity_to_idx(ThrottlingSeverity::SEVERE)] = config.thresholds.hot as f32 / 1000.0;
        thresholds[Self::severity_to_idx(ThrottlingSeverity::CRITICAL)] = config.thresholds.critical as f32 / 1000.0;
        thresholds[Self::severity_to_idx(ThrottlingSeverity::EMERGENCY)] = config.thresholds.emergency as f32 / 1000.0;
        // Set shutdown threshold slightly below emergency to let Android handle shutdown
        thresholds[Self::severity_to_idx(ThrottlingSeverity::SHUTDOWN)] = (config.thresholds.emergency as f32 / 1000.0) - 1.0;
        tz.threshold.hotThrottlingThresholds = thresholds;
        tz.threshold.coldThrottlingThresholds = vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];

        tz
    }

    fn get_file(&self, file_name: &str) -> String {
        format!("/sys/class/thermal/{}/{}", self.dir_name, file_name)
    }


    fn severity_to_idx(severity: ThrottlingSeverity) -> usize {
        match severity {
            ThrottlingSeverity::NONE => 0,
            ThrottlingSeverity::LIGHT => 1,
            ThrottlingSeverity::MODERATE => 2,
            ThrottlingSeverity::SEVERE => 3,
            ThrottlingSeverity::CRITICAL => 4,
            ThrottlingSeverity::EMERGENCY => 5,
            ThrottlingSeverity::SHUTDOWN => 6,
            _ => panic!("Unknown severity"),
        }
    }

    fn idx_to_severity(idx: usize) -> ThrottlingSeverity {
        match idx {
            0 => ThrottlingSeverity::NONE,
            1 => ThrottlingSeverity::LIGHT,
            2 => ThrottlingSeverity::MODERATE,
            3 => ThrottlingSeverity::SEVERE,
            4 => ThrottlingSeverity::CRITICAL,
            5 => ThrottlingSeverity::EMERGENCY,
            6 => ThrottlingSeverity::SHUTDOWN,
            _ => panic!("Unknown ThrottlingSeverity index {}", idx),
        }
    }



    pub fn read_temp(&self) -> Temperature {
        let temp_path = self.get_file("temp");
        let temp = match utils::read_sysfs_file(&temp_path) {
            Ok(s) => f32::from_str(&s).unwrap_or(f32::NAN) / 1000.0,
            Err(_) => f32::NAN,
        };

        let mut throttling_status = ThrottlingSeverity::NONE;
        if !temp.is_nan() {
            for (idx, val) in self
                .threshold
                .hotThrottlingThresholds
                .iter()
                .enumerate()
                .rev()
            {
                if *val != -1.0 && temp >= *val {
                    throttling_status = Self::idx_to_severity(idx);
                    break;
                }
            }
        }

        Temperature {
            r#type: self.temp.r#type,
            name: self.temp.name.clone(),
            value: temp,
            throttlingStatus: throttling_status,
        }
    }


    pub fn get_type(&self) -> TemperatureType {
        self.threshold.r#type
    }

    pub fn clone_threshold(&self) -> TemperatureThreshold {
        let threshold = &self.threshold;
        TemperatureThreshold {
            r#type: threshold.r#type,
            name: threshold.name.clone(),
            hotThrottlingThresholds: threshold.hotThrottlingThresholds.clone(),
            coldThrottlingThresholds: threshold.coldThrottlingThresholds.clone(),
        }
    }
}
