use android_hardware_thermal::aidl::android::hardware::thermal::{
    Temperature::Temperature, TemperatureThreshold::TemperatureThreshold,
    TemperatureType::TemperatureType, ThrottlingSeverity::ThrottlingSeverity,
};
use log::debug;

use std::str::FromStr;

use crate::utils;

#[derive(Default)]
pub struct ThermalZone {
    dir_name: String,
    threshold: TemperatureThreshold,
    temp: Temperature,
}

impl ThermalZone {
    pub fn new(name: &str) -> Self {
        let mut tz = Self {
            dir_name: name.to_string(),
            ..Default::default()
        };

        tz.set_type_and_name();
        tz.set_hot_throttling_thresholds();

        tz
    }

    fn get_file(&self, file_name: &str) -> String {
        format!("/sys/class/thermal/{}/{}", self.dir_name, file_name)
    }

    fn set_type_and_name(&mut self) {
        let type_names = [
            ("cpu", TemperatureType::CPU),
            ("gpu", TemperatureType::GPU)
        ];
        let path = self.get_file("type");
        let dev_type = utils::read_sysfs_file(&path).unwrap();

        debug!("{}: \"{}\"", path, dev_type);

        let mut temp_type = TemperatureType::UNKNOWN;
        for (name, r#type) in type_names {
            if dev_type.contains(name) {
                temp_type = r#type;
                break;
            }
        }

        self.threshold.name = dev_type.clone();
        self.temp.name = dev_type;
        self.threshold.r#type = temp_type;
        self.temp.r#type = temp_type;
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

    fn set_hot_throttling_thresholds(&mut self) {
        let mut thresholds = vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
        let path = format!("/sys/class/thermal/{}", self.dir_name);
        let trip_point_type_regex = regex::Regex::new(r"^trip_point_([0-9]+)_type$").unwrap();

        for entry in std::fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let type_file_name = entry.file_name().into_string().unwrap();
            let Some(idx) = trip_point_type_regex.captures(&type_file_name) else {
                continue;
            };
            let idx = u32::from_str(idx.get(1).unwrap().as_str()).unwrap();

            let temp_path = self.get_file(&format!("trip_point_{}_temp", idx));
            let type_path = entry.path();

            let threshold_type = utils::read_sysfs_file(type_path.to_str().unwrap()).unwrap();
            let threshold_temp = utils::read_sysfs_file(&temp_path).unwrap();
            let threshold_temp = f32::from_str(&threshold_temp).unwrap() / 1000.0;

            match threshold_type.as_str() {
                // Here, the association between a trip point type('active', 'passive', ...) and a
                // ThrottlingSeverity value is completely arbitrary so far */
                "passive" => {
                    thresholds[Self::severity_to_idx(ThrottlingSeverity::MODERATE)] = threshold_temp
                }
                "active" => {
                    thresholds[Self::severity_to_idx(ThrottlingSeverity::SEVERE)] = threshold_temp
                }
                "hot" => {
                    thresholds[Self::severity_to_idx(ThrottlingSeverity::EMERGENCY)] =
                        threshold_temp
                }
                "critical" => {
                    // -1 because we want android to shutdown the board, not Linux
                    thresholds[Self::severity_to_idx(ThrottlingSeverity::SHUTDOWN)] = threshold_temp - 1.0
                }
                _ => todo!(),
            }
        }

        self.threshold.hotThrottlingThresholds = thresholds;
        self.threshold.coldThrottlingThresholds = vec![-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
    }

    pub fn read_temp(&self) -> Temperature {
        let temp_path = self.get_file("temp");
        let temp = utils::read_sysfs_file(&temp_path).unwrap();
        let temp = f32::from_str(&temp).unwrap() / 1000.0;

        let mut throttling_status = ThrottlingSeverity::NONE;
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

        Temperature {
            r#type: self.temp.r#type,
            name: self.temp.name.clone(),
            value: temp,
            throttlingStatus: throttling_status,
        }
    }

    pub fn is_valid(&self) -> bool {
        // We assume that if the hottest threshold isn't defined, none is defined and we ignore this sensor.
        self.threshold.hotThrottlingThresholds
            [ThermalZone::severity_to_idx(ThrottlingSeverity::SHUTDOWN)]
            != -1.0
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
