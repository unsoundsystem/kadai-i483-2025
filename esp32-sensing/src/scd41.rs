use anyhow::Result;
use byteorder::{BigEndian, ByteOrder};
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::{FreeRtos, BLOCK}, i2c::I2cDriver},
};

const SCD41_ADDR: u8 = 0x62;

pub fn setup(bus: &mut I2cDriver) {
    stop_periodic_measurement(bus);
    FreeRtos::delay_ms(500);
    start_periodic_measurement(bus);
}

pub fn start_periodic_measurement(bus: &mut I2cDriver) -> Result<()> {
    bus.write(SCD41_ADDR, &[0x21, 0xb1], BLOCK)?;
    Ok(())
}

pub fn stop_periodic_measurement(bus: &mut I2cDriver) -> Result<()> {
    bus.write(SCD41_ADDR, &[0x3f, 0x86], BLOCK)?;
    Ok(())
}

pub fn is_data_ready(bus: &mut I2cDriver) -> Result<bool> {
    let mut res = [0u8; 3];
    bus.write_read(SCD41_ADDR, &[0xe4, 0xb8], &mut res, BLOCK)?;
    let word = ((res[0] as u16) << 8) | res[1] as u16;
    Ok(word & 0b111_1111_1111 != 0)
}

pub fn read_measurement(bus: &mut I2cDriver) -> Result<(u16, u16, u16)> {
    let mut res: [u8; 9] = [0u8; 9];
    bus.write_read(SCD41_ADDR, &[0xec, 0x5], &mut res, BLOCK)?;
    let co2: u16 = BigEndian::read_u16(&res[0..2]);
    let temp: u16 = BigEndian::read_u16(&res[3..5]);
    let humidity: u16 = BigEndian::read_u16(&res[6..8]);
    Ok((co2, temp, humidity))
}

pub fn perform_measurement(bus: &mut I2cDriver) -> Result<(u16, f64, f64)> {
    let (co2, rawtemp, rawhum) = read_measurement(bus)?;
    Ok((co2, temp_comp(rawtemp), humidity_comp(rawhum)))
}

pub fn temp_comp(data: u16) -> f64 {
     -45.0 + 175.0 * ((data as f64) / 0xffff as f64)
}

pub fn humidity_comp(data: u16) -> f64 {
    100.0 * ((data as f64) / 0xffff as f64)
}
