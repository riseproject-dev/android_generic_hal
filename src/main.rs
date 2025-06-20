mod cool_device;
mod thermal;
mod thermal_zone;
mod utils;

use binder;
use log::info;

use android_hardware_thermal::aidl::android::hardware::thermal::IThermal::{BnThermal, IThermal};
use binder::BinderFeatures;
use thermal::ThermalService;

const LOG_TAG: &str = "thermal_hal";

fn main() {
    // TODO: Change max_level depending on build type.
    // Tried to use android::base::GetProperty(BUILD_TYPE_PROPERTY, "") as in C++ but libbase is
    // not available for Rust.
    // Maybe use https://source.android.com/docs/setup/build/rust/building-rust-modules/android-rust-patterns#rust-cpp-interop-using-cxx
    logger::init(
        logger::Config::default()
            .with_tag_on_device(LOG_TAG)
            .with_max_level(log::LevelFilter::Info),
    );

    binder::ProcessState::set_thread_pool_max_thread_count(0);

    let thermal_service = ThermalService::load_devices()
        .expect("Failed to load thermal devices and zones from sysfs");
    let thermal_service_binder = BnThermal::new_binder(thermal_service, BinderFeatures::default());
    let thermal_service_name = format!("{}/default", ThermalService::get_descriptor());

    binder::add_service(&thermal_service_name, thermal_service_binder.as_binder())
        .expect("Failed to register service");

    info!("Service started");

    binder::ProcessState::join_thread_pool();
}
