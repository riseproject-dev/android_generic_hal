use android_hardware_thermal::aidl::android::hardware::thermal::{
    CoolingDevice::CoolingDevice, CoolingType::CoolingType,
    ICoolingDeviceChangedCallback::ICoolingDeviceChangedCallback, IThermal::IThermal,
    IThermalChangedCallback::IThermalChangedCallback, Temperature::Temperature,
    TemperatureThreshold::TemperatureThreshold, TemperatureType::TemperatureType,
};
use android_hardware_thermal::binder;
use binder::binder_impl::IBinderInternal;
use log::{debug, error, info, trace};
use regex;

use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::vec::Vec;
use std::iter::zip;

use crate::cool_device::CoolDevice;
use crate::thermal_zone::ThermalZone;

pub struct Thermal {
    cooling_devices: Vec<CoolDevice>,
    thermal_zones: Vec<ThermalZone>,
    thermal_callbacks: Mutex<
        Vec<(
            binder::Strong<(dyn IThermalChangedCallback + 'static)>,
            TemperatureType,
        )>,
    >,
    device_callbacks: Mutex<
        Vec<(
            binder::Strong<(dyn ICoolingDeviceChangedCallback + 'static)>,
            CoolingType,
        )>,
    >,
}

pub struct ThermalService {
    inner: Arc<Thermal>,
    _handle: JoinHandle<()>,
}

impl ThermalService {
    pub fn load_devices() -> std::io::Result<Self> {
        let inner = Arc::new(Thermal::load_devices()?);
        // Inner is an `Arc` so we are just getting another Arc not a deep clone of the undelaying
        // objet
        let _handle = Self::start_callback_handler(inner.clone());

        Ok(Self { inner, _handle })
    }

    fn start_callback_handler(inner: Arc<Thermal>) -> JoinHandle<()> {
        thread::spawn(move || {
            debug!("Callback handler thread spawned");

            let mut old_temps = inner.read_temperatures();
            loop {
                let temps = inner.read_temperatures();
                let mut thermal_callbacks = inner
                    .thermal_callbacks
                    .lock()
                    .expect("Handler: Failed to get lock on thermal callbacks");

                thermal_callbacks.retain(|(cb, _)| {
                    if !cb.as_binder().is_binder_alive() {
                        debug!("Handler: Thermal callback dead and removed");
                        false
                    } else {
                        true
                    }
                });

                for (cb, temp_type) in thermal_callbacks.iter() {
                    for (old_temp, temp) in zip(&old_temps, &temps) {
                        if *temp_type != TemperatureType::UNKNOWN && temp.r#type != *temp_type {
                            continue;
                        }

                        if old_temp.throttlingStatus == temp.throttlingStatus {
                            continue;
                        }

                        debug!(
                            "Handler: Sending notification: current value = {}",
                            temp.value
                        );

                        // By the time we actualy call the callback the object could be dead, even
                        // if we tried to catch it with `is_binder_alive` earlier. Let's error out
                        // instead of crashing with `expect` or `unwrap`
                        if let Err(error) = cb.as_ref().notifyThrottling(temp) {
                            error!("Handler: Calling a thermal callback failed with {}", error);
                        }
                    }
                }

                // We are done using the callback. Release the mutex
                drop(thermal_callbacks);

                old_temps = temps;

                // Sleep for 1 sec
                thread::sleep(std::time::Duration::from_secs(1));
            }
        })
    }
}

impl binder::Interface for ThermalService {}

impl Thermal {
    pub fn load_devices() -> std::io::Result<Self> {
        let sysfs_thermal_dir = "/sys/class/thermal/";
        let mut cooling_devices = vec![];
        let mut thermal_zones = vec![];
        let cooling_devices_regex = regex::Regex::new(r"^cooling_device[0-9]+$").unwrap();
        let thermal_zones_regex = regex::Regex::new(r"^thermal_zone[0-9]+$").unwrap();

        for entry in std::fs::read_dir(&sysfs_thermal_dir)? {
            let entry = entry?;
            let file_name = entry.file_name().into_string().unwrap();

            if cooling_devices_regex.is_match(&file_name) {
                cooling_devices.push(CoolDevice::new(&file_name));
            } else if thermal_zones_regex.is_match(&file_name) {
                let tz = ThermalZone::new(&file_name);
                if tz.get_type() != TemperatureType::UNKNOWN && tz.is_valid() {
                    thermal_zones.push(tz);
                } else {
                    info!("Ignoring {}", file_name);
                }
            }
        }

        Ok(Thermal {
            cooling_devices,
            thermal_zones,
            thermal_callbacks: Mutex::new(vec![]),
            device_callbacks: Mutex::new(vec![]),
        })
    }

    pub fn read_temperatures(&self) -> Vec<Temperature> {
        let mut temps = Vec::with_capacity(self.thermal_zones.len());

        for tz in &self.thermal_zones {
            temps.push(tz.read_temp())
        }

        temps
    }
}

impl IThermal for ThermalService {
    fn getCoolingDevices(&self) -> Result<Vec<CoolingDevice>, binder::Status> {
        trace!("getCoolingDevices called");

        let mut out_devices = Vec::with_capacity(self.inner.cooling_devices.len());

        for cool_dev in &self.inner.cooling_devices {
            let dev = cool_dev.updated_clone();

            out_devices.push(dev)
        }

        match out_devices.len() {
            0 => Err(binder::Status::new_exception_str(
                binder::ExceptionCode::ILLEGAL_STATE,
                Some("No cooling device"),
            )),
            _ => Ok(out_devices),
        }
    }

    fn getCoolingDevicesWithType(
        &self,
        in_type: CoolingType,
    ) -> Result<Vec<CoolingDevice>, binder::Status> {
        trace!("getCoolingDevicesWithType called: in_type = {:?}", in_type);

        let mut out_devices = vec![];

        for cool_dev in &self.inner.cooling_devices {
            if cool_dev.get_type() != in_type {
                continue;
            }

            let dev = cool_dev.updated_clone();

            out_devices.push(dev)
        }

        match out_devices.len() {
            0 => Err(binder::Status::new_exception_str(
                binder::ExceptionCode::ILLEGAL_STATE,
                Some("No cooling device"),
            )),
            _ => Ok(out_devices),
        }
    }

    fn getTemperatures(&self) -> Result<Vec<Temperature>, binder::Status> {
        trace!("getTemperatures called");

        Ok(self.inner.read_temperatures())
    }

    fn getTemperaturesWithType(
        &self,
        in_type: TemperatureType,
    ) -> Result<Vec<Temperature>, binder::Status> {
        trace!("getTemperaturesWithType called: in_type = {:?}", in_type);

        let mut out_temps = vec![];

        for tz in &self.inner.thermal_zones {
            if tz.get_type() != in_type {
                continue;
            }

            out_temps.push(tz.read_temp());
        }

        Ok(out_temps)
    }

    fn getTemperatureThresholds(&self) -> Result<Vec<TemperatureThreshold>, binder::Status> {
        trace!("getTemperatureThresholds called");

        let mut out_thresholds = Vec::with_capacity(self.inner.thermal_zones.len());

        for tz in &self.inner.thermal_zones {
            let threshold = tz.clone_threshold();

            out_thresholds.push(threshold);
        }

        match out_thresholds.len() {
            0 => Err(binder::Status::new_exception_str(
                binder::ExceptionCode::ILLEGAL_STATE,
                Some("No temperature threshold"),
            )),
            _ => Ok(out_thresholds),
        }
    }

    fn getTemperatureThresholdsWithType(
        &self,
        in_type: TemperatureType,
    ) -> Result<Vec<TemperatureThreshold>, binder::Status> {
        trace!(
            "getTemperatureThresholdsWithType called: in_type = {:?}",
            in_type
        );

        let mut out_thresholds = vec![];

        for tz in &self.inner.thermal_zones {
            if tz.get_type() != in_type {
                continue;
            }

            let threshold = tz.clone_threshold();

            out_thresholds.push(threshold);
        }

        Ok(out_thresholds)
    }

    fn registerThermalChangedCallback(
        &self,
        in_callback: &binder::Strong<(dyn IThermalChangedCallback + 'static)>,
    ) -> Result<(), binder::Status> {
        trace!("registerThermalChangedCallback called");

        let mut thermal_callbacks = self.inner.thermal_callbacks.lock().unwrap();

        for (callback, _) in thermal_callbacks.iter() {
            if callback == in_callback {
                return Err(binder::Status::new_exception_str(
                    binder::ExceptionCode::ILLEGAL_ARGUMENT,
                    Some("Callback already registered"),
                ));
            }
        }

        thermal_callbacks.push((in_callback.clone(), TemperatureType::UNKNOWN));

        Ok(())
    }

    fn registerThermalChangedCallbackWithType(
        &self,
        in_callback: &binder::Strong<(dyn IThermalChangedCallback + 'static)>,
        in_type: TemperatureType,
    ) -> Result<(), binder::Status> {
        trace!(
            "registerThermalChangedCallbackWithType called: in_type = {:?}",
            in_type
        );

        let mut thermal_callbacks = self.inner.thermal_callbacks.lock().unwrap();

        for (callback, _) in thermal_callbacks.iter() {
            if callback == in_callback {
                return Err(binder::Status::new_exception_str(
                    binder::ExceptionCode::ILLEGAL_ARGUMENT,
                    Some("Callback already registered"),
                ));
            }
        }

        thermal_callbacks.push((in_callback.clone(), in_type));

        Ok(())
    }

    fn unregisterThermalChangedCallback(
        &self,
        in_callback: &binder::Strong<(dyn IThermalChangedCallback + 'static)>,
    ) -> Result<(), binder::Status> {
        trace!("unregisterThermalChangedCallback called");

        let mut thermal_callbacks = self.inner.thermal_callbacks.lock().unwrap();
        let old_size = thermal_callbacks.len();

        thermal_callbacks.retain(|(cb, _)| cb != in_callback);

        if old_size == thermal_callbacks.len() {
            return Err(binder::Status::new_exception_str(
                binder::ExceptionCode::ILLEGAL_ARGUMENT,
                Some("Callback wasn't registered"),
            ));
        }

        Ok(())
    }

    fn registerCoolingDeviceChangedCallbackWithType(
        &self,
        in_callback: &binder::Strong<(dyn ICoolingDeviceChangedCallback + 'static)>,
        in_type: CoolingType,
    ) -> Result<(), binder::Status> {
        trace!(
            "registerCoolingDeviceChangedCallbackWithType called: in_type = {:?}",
            in_type
        );
        let mut device_callbacks = self.inner.device_callbacks.lock().unwrap();

        for (callback, _) in device_callbacks.iter() {
            if callback == in_callback {
                return Err(binder::Status::new_exception_str(
                    binder::ExceptionCode::ILLEGAL_ARGUMENT,
                    Some("Callback already registered"),
                ));
            }
        }

        device_callbacks.push((in_callback.clone(), in_type));

        // TODO: implement callbacks for cooling devices
        error!("For now CoolingDeviceChangedCallback are never ran");

        Ok(())
    }

    fn unregisterCoolingDeviceChangedCallback(
        &self,
        in_callback: &binder::Strong<(dyn ICoolingDeviceChangedCallback + 'static)>,
    ) -> Result<(), binder::Status> {
        trace!("unregisterCoolingDeviceChangedCallback called");

        let mut device_callbacks = self.inner.device_callbacks.lock().unwrap();
        let old_size = device_callbacks.len();

        device_callbacks.retain(|(cb, _)| cb != in_callback);

        if old_size == device_callbacks.len() {
            return Err(binder::Status::new_exception_str(
                binder::ExceptionCode::ILLEGAL_ARGUMENT,
                Some("Callback wasn't registered"),
            ));
        }

        // TODO: implement callbacks for cooling devices
        error!("For now CoolingDeviceChangedCallback are never ran");

        Ok(())
    }
}
