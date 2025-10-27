use android_hardware_thermal::aidl::android::hardware::thermal::{
    CoolingType::CoolingType, TemperatureType::TemperatureType,
};
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThermalThresholds {
    pub hot: i32,
    pub critical: i32,
    pub emergency: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThermalZoneConfig {
    pub sysfs_name: String,
    pub name: String,
    pub r#type: String,
    pub multiplier: f32,
    pub thresholds: ThermalThresholds,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CoolingDeviceConfig {
    pub sysfs_name: String,
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThermalConfig {
    pub thermal_zones: Vec<ThermalZoneConfig>,
    #[serde(default)]
    pub cooling_devices: Vec<CoolingDeviceConfig>,
}

impl ThermalConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ThermalConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn get_zone_config(&self, sysfs_name: &str) -> Option<&ThermalZoneConfig> {
        self.thermal_zones
            .iter()
            .find(|zone| zone.sysfs_name == sysfs_name)
    }

    pub fn get_cooling_device_config(&self, sysfs_name: &str) -> Option<&CoolingDeviceConfig> {
        self.cooling_devices
            .iter()
            .find(|device| device.sysfs_name == sysfs_name)
    }

}

impl ThermalZoneConfig {
    pub fn get_temperature_type(&self) -> TemperatureType {
        match self.r#type.as_str() {
            "CPU" => TemperatureType::CPU,
            "GPU" => TemperatureType::GPU,
            "BATTERY" => TemperatureType::BATTERY,
            "SKIN" => TemperatureType::SKIN,
            "USB_PORT" => TemperatureType::USB_PORT,
            "POWER_AMPLIFIER" => TemperatureType::POWER_AMPLIFIER,
            "BCL_VOLTAGE" => TemperatureType::BCL_VOLTAGE,
            "BCL_CURRENT" => TemperatureType::BCL_CURRENT,
            "BCL_PERCENTAGE" => TemperatureType::BCL_PERCENTAGE,
            "NPU" => TemperatureType::NPU,
            "TPU" => TemperatureType::TPU,
            "DISPLAY" => TemperatureType::DISPLAY,
            "MODEM" => TemperatureType::MODEM,
            "SOC" => TemperatureType::SOC,
            _ => {
                warn!("Unknown temperature type: {}, defaulting to UNKNOWN", self.r#type);
                TemperatureType::UNKNOWN
            }
        }
    }
}

impl CoolingDeviceConfig {
    pub fn get_cooling_type(&self) -> CoolingType {
        match self.r#type.as_str() {
            "FAN" => CoolingType::FAN,
            "BATTERY" => CoolingType::BATTERY,
            "CPU" => CoolingType::CPU,
            "GPU" => CoolingType::GPU,
            "MODEM" => CoolingType::MODEM,
            "NPU" => CoolingType::NPU,
            "COMPONENT" => CoolingType::COMPONENT,
            "TPU" => CoolingType::TPU,
            "POWER_AMPLIFIER" => CoolingType::POWER_AMPLIFIER,
            "DISPLAY" => CoolingType::DISPLAY,
            "SPEAKER" => CoolingType::SPEAKER,
            "WIFI" => CoolingType::WIFI,
            "CAMERA" => CoolingType::CAMERA,
            _ => {
                warn!("Unknown cooling type: {}, defaulting to COMPONENT", self.r#type);
                CoolingType::COMPONENT
            }
        }
    }
}