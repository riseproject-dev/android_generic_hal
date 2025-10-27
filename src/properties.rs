use log::{debug, info, warn};
use std::process::Command;

const THERMAL_HARDWARE_PROPERTY: &str = "vendor.thermal.hardware";
const DEFAULT_CONFIG_NAME: &str = "generic";

fn get_thermal_hardware_config() -> String {
    info!("Checking property: {}", THERMAL_HARDWARE_PROPERTY);
    
    // Method 2: Android property via getprop command
    if let Some(hardware) = get_android_property_via_getprop(THERMAL_HARDWARE_PROPERTY) {
        if !hardware.is_empty() {
            info!("Found thermal hardware property {}: '{}'", THERMAL_HARDWARE_PROPERTY, hardware);
            return hardware;
        }
    }
    
    info!("Property {} not set or empty, using default: {}", THERMAL_HARDWARE_PROPERTY, DEFAULT_CONFIG_NAME);
    DEFAULT_CONFIG_NAME.to_string()
}

pub fn get_config_file_path() -> String {
    let hardware = get_thermal_hardware_config();
    
    // Try hardware-specific APEX config
    let apex_config_path = format!("/apex/com.android.hardware.thermal.rs.generic/etc/thermal/thermal_config_{}.json", hardware);
    if std::path::Path::new(&apex_config_path).exists() {
        info!("Using thermal config from APEX: {}", apex_config_path);
        return apex_config_path;
    }
    
    // Fallback to generic config in APEX
    let apex_generic_path = "/apex/com.android.hardware.thermal.rs.generic/etc/thermal/thermal_config_generic.json";
    warn!("Config file {} not found, falling back to generic APEX: {}", apex_config_path, apex_generic_path);
    apex_generic_path.to_string()
}

fn get_android_property_via_getprop(key: &str) -> Option<String> {
    let Ok(output) = Command::new("getprop").arg(key).output() else {
        debug!("Failed to execute getprop for {}", key);
        return None;
    };

    if !output.status.success() {
        debug!("getprop command failed for {}", key);
        return None;
    }

    let binding = String::from_utf8_lossy(&output.stdout);
    let value = binding.trim();

    if value.is_empty() {
        debug!("Property {} not set (getprop)", key);
        None
    } else {
        debug!("Property {} via getprop: {}", key, value);
        Some(value.to_string())
    }
}


