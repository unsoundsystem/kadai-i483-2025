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
    FreeRtos::delay_ms(500);
    let dps_coef = dps310::read_coefficients(&mut i2c)?;

    loop {
        FreeRtos::delay_ms(1000);

        // DPS310
        let rawtmp = dps310::read_temprature(&mut i2c)?;
        println!("tmp: {} °C, rawtmp: {}", dps310::comp_temp_val(rawtmp, &dps_coef), rawtmp);
        FreeRtos::delay_ms(500);
        let rawprs = dps310::read_pressure(&mut i2c)?;
        println!("prs: {} hPa, rawprs: {}\n", dps310::comp_prs_val(rawprs, rawtmp, &dps_coef) / 100f32, rawprs);

        // BH1750
        FreeRtos::delay_ms(500);
        let rawlx = bh1750::perform_measurement(&mut i2c)?;
        println!("rawlx(bh): {}, lux: {}\n", rawlx, bh1750::calc_lux(rawlx));

        // rpr0521rs
        FreeRtos::delay_ms(500);
        let rawlx2 = rpr0521rs::perform_measurement(&mut i2c)?;
        println!("rawlx(rpr): {}, lux: {}\n", rawlx2, rawlx2);
    }
    Ok(())
}
