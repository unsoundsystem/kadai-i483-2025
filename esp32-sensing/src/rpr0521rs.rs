use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::{FreeRtos, BLOCK}, i2c::I2cDriver},
};

const RPR_ADDR: u8 = 0x38;

enum RprRegs {
    SystemControl = 0x40,
    ModeControl = 0x41,
    AlsPsControl = 0x42,
    Als0Lsb = 0x46,
    Als0Msb = 0x47,
}

pub fn setup(bus: &mut I2cDriver) {
    // soft reset
    bus.write(RPR_ADDR, &[RprRegs::SystemControl as u8, 0x80], BLOCK);
    // Mode setting: ALS ON, 400ms
    bus.write(RPR_ADDR, &[RprRegs::ModeControl as u8, 0x8a], BLOCK);
    // ALS setting: All gain set to x1, 50mA LED current
    bus.write(RPR_ADDR, &[RprRegs::AlsPsControl as u8, 0x2], BLOCK);
}

pub fn perform_measurement(bus: &mut I2cDriver) -> Result<u16> {
    let mut als0_lsb_buf = [0u8; 1];
    let mut als0_msb_buf = [0u8; 1];
    bus.write_read(RPR_ADDR, &[RprRegs::Als0Lsb as u8], &mut als0_lsb_buf, BLOCK)?;
    bus.write_read(RPR_ADDR, &[RprRegs::Als0Msb as u8], &mut als0_msb_buf, BLOCK)?;

    Ok(((als0_msb_buf[0] as u16) << 8) | als0_lsb_buf[0] as u16)
}
