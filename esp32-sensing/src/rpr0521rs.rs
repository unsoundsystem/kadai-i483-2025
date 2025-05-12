use byteorder::{ByteOrder, LittleEndian};
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
    Als1Lsb = 0x48,
    Als1Msb = 0x49,
}

const ALS_GAIN_x1: u8 = 0b00_00_00_00;
const ALS_GAIN_x64: u8 = 0b00_10_10_00;
const ALS_LED_CURRENT_200: u8 = 0b00_00_00_11;

pub fn setup(bus: &mut I2cDriver) {
    // soft reset
    bus.write(RPR_ADDR, &[RprRegs::SystemControl as u8, 0x80], BLOCK);
    // Mode setting: ALS ON, 400ms
    bus.write(RPR_ADDR, &[RprRegs::ModeControl as u8, 0x8a], BLOCK);
    // ALS setting: All gain set to x1, 200mA LED current
    bus.write(RPR_ADDR, &[RprRegs::AlsPsControl as u8, ALS_GAIN_x1 | ALS_LED_CURRENT_200], BLOCK);
    // ALS setting: All gain set to x128, 200mA LED current
    //bus.write(RPR_ADDR, &[RprRegs::AlsPsControl as u8, 0b00_10_11_11], BLOCK);
}

pub fn perform_measurement(bus: &mut I2cDriver) -> Result<(f64, f64)> {
    let mut res = [0u8; 4];
    let mut als0_lsb_buf = [0u8; 1];
    let mut als0_msb_buf = [0u8; 1];
    bus.write_read(RPR_ADDR, &[RprRegs::Als0Lsb as u8], &mut res, BLOCK)?;
    //bus.write_read(RPR_ADDR, &[RprRegs::Als0Lsb as u8], &mut als0_lsb_buf, BLOCK)?;
    //bus.write_read(RPR_ADDR, &[RprRegs::Als0Msb as u8], &mut als0_msb_buf, BLOCK)?;

    //Ok(((als0_msb_buf[0] as u16) << 8) | als0_lsb_buf[0] as u16)

    let als0_data = LittleEndian::read_u16(&res[0..2]);
    let als1_data = LittleEndian::read_u16(&res[2..4]);

    let mut ctr_res = [0u8; 1];
    bus.write_read(RPR_ADDR, &[RprRegs::AlsPsControl as u8], &mut ctr_res, BLOCK)?;
    let mut als0_lx: f64 = 0.0;
    let mut als1_lx: f64 = 0.0;

    // data * (100 / measurement time) / gain
    if ctr_res[0] & ALS_GAIN_x1 == ALS_GAIN_x1 {
        als0_lx = (als0_data as f64) * (100.0 / 400.0);
        als1_lx = (als1_data as f64) * (100.0 / 400.0);
    } else if ctr_res[0] & ALS_GAIN_x64 == ALS_GAIN_x64 {
        als0_lx = (als0_data as f64) * (100.0 / 400.0) / 64.0;
        als1_lx = (als1_data as f64) * (100.0 / 400.0) / 64.0;
    }

    //let mut lx = als0_lx;
    //let d1_d0 = als1_lx / als0_lx;

    //if (d1_d0 < 0.595) {
      //lx = (1.682 * als0_lx - 1.877 * als1_lx);
    //} else if (d1_d0 < 1.015) {
      //lx = (0.644 * als0_lx - 0.132 * als1_lx);
    //} else if (d1_d0 < 1.352) {
      //lx = (0.756 * als0_lx - 0.243 * als1_lx);
    //} else if (d1_d0 < 3.053) {
      //lx = (0.766 * als0_lx - 0.25 * als1_lx);
    //} else {
      //lx = 0.0;
    //}
    Ok((als0_lx, als1_lx))
}
