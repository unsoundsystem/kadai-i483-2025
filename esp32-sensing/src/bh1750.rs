use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::{FreeRtos, BLOCK}, i2c::I2cDriver},
};

const BH1750_ADDR: u8 = 0x23;

enum Bh1750Opecode {
    PowerOn = 0x1,
    OneTimeHRes2 = 0x21,
    // Append high/low bits of the MTReg we want to set using OR to those values
    ChangeMeasurementTimeHigh = 0b0100_0000,
    ChangeMeasurementTimeLow = 0b0110_0000,
}

const BH1750_MTREG: u8 = 0xfe;

pub fn setup(bus: &mut I2cDriver) {
    bus.write(BH1750_ADDR, &[Bh1750Opecode::PowerOn as u8], BLOCK);
    // NOTE: times taking until data will available will be (default MTReg / MTReg) times longer
    bus.write(BH1750_ADDR, &[Bh1750Opecode::ChangeMeasurementTimeHigh as u8 | (BH1750_MTREG >> 5)], BLOCK);
    bus.write(BH1750_ADDR, &[Bh1750Opecode::ChangeMeasurementTimeLow as u8 | (BH1750_MTREG & 0x1f)], BLOCK);
    FreeRtos::delay_ms(120 * 4);
}

pub fn perform_measurement(bus: &mut I2cDriver) -> Result<u16> {
    let mut buf = [0u8; 2];
    bus.write(BH1750_ADDR, &[Bh1750Opecode::PowerOn as u8], BLOCK)?;
    bus.write(BH1750_ADDR, &[Bh1750Opecode::OneTimeHRes2 as u8], BLOCK)?;
    bus.read(BH1750_ADDR, &mut buf, BLOCK)?;

    Ok((buf[0] as u16) << 8 | buf[1] as u16)
}

pub fn calc_lux(data: u16) -> f64 {
    (data as f64)
        * (69f64 / BH1750_MTREG as f64)
        / 2.0 // mode res2
        / 1.2 // factor
}
