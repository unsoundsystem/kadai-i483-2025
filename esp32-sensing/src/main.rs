mod bh1750;
mod dps310;
mod rpr0521rs;
mod scd41;
use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::FreeRtos, i2c::{I2cConfig, I2cDriver}, units::*},
  sys::link_patches,
};

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio19;

    println!("Starting I2C");

    let config = I2cConfig::new().baudrate(KiloHertz(400).into());
    let mut i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    bh1750::setup(&mut i2c);
    dps310::setup(&mut i2c);
    rpr0521rs::setup(&mut i2c);
    scd41::setup(&mut i2c);
    FreeRtos::delay_ms(500);
    let dps_coef = dps310::read_coefficients(&mut i2c)?;

    loop {
        FreeRtos::delay_ms(5000);

        // DPS310
        let rawtmp = dps310::read_temprature(&mut i2c)?;
        println!("[DPS310] tmp: {:.2} °C, rawtmp: {}", dps310::comp_temp_val(rawtmp, &dps_coef), rawtmp);
        FreeRtos::delay_ms(100);
        let rawprs = dps310::read_pressure(&mut i2c)?;
        println!("[DPS310] prs: {} hPa, rawprs: {}", dps310::comp_prs_val(rawprs, rawtmp, &dps_coef) / 100f32, rawprs);

        // BH1750
        //FreeRtos::delay_ms(500);
        let rawlx = bh1750::perform_measurement(&mut i2c)?;
        println!("[BH1750] rawlx: {}, lux: {:.2}", rawlx, bh1750::calc_lux(rawlx));

        // rpr0521rs
        let (lx, inflx) = rpr0521rs::perform_measurement(&mut i2c)?;
        println!("[RPR0521] {} lx, (inflx: {})", lx, inflx);

        // SCD41
        if scd41::is_data_ready(&mut i2c)? {
            let (co2, temp, hum) = scd41::read_measurement(&mut i2c)?;
            println!("[SCD41] co2: {}, rawtemp: {}, rawhum: {} RH", co2, temp, hum);
            println!("[SCD41] co2: {} ppm, temprature: {:.2} °C, humidity: {:.2} RH", co2, scd41::temp_comp(temp), scd41::humidity_comp(hum));
        } else {
            println!("[SCD41] data is not available yet");
        }
        println!("---------------------------------");
    }
    Ok(())
}
