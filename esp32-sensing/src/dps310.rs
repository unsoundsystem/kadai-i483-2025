use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::{FreeRtos, BLOCK}, i2c::I2cDriver},
};

const DPS310_ADDR: u8 = 0x77;

enum Dps310Regs {
    Status = 0x8,
    Reset = 0xc,
}

pub fn setup(bus: &mut I2cDriver) {
    // soft reset
    bus.write(DPS310_ADDR, &[Dps310Regs::Status as u8, 0b1001], BLOCK);

    let mut status = [0u8; 1];
    bus.write_read(0x77, &[Dps310Regs::Status as u8], &mut status, BLOCK);
    println!("dps310 status: coef({}), sensor({}), tmp({}), prs({}), ctrl is {:b}",
        status[0] & 1 << 7 != 0,
        status[0] & 1 << 6 != 0,
        status[0] & 1 << 5 != 0,
        status[0] & 1 << 4 != 0,
        status[0] & 0b111);
}
