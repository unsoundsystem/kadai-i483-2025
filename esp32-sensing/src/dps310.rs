use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::{FreeRtos, BLOCK}, i2c::I2cDriver},
};

const DPS310_ADDR: u8 = 0x77;

enum Dps310Regs {
    Status = 0x8,
    Reset = 0xc,
    PrsCfg = 0x6,
    TmpCfg = 0x7,
}

pub fn setup(bus: &mut I2cDriver) {
    // soft reset
    bus.write(DPS310_ADDR, &[Dps310Regs::Status as u8, 0b1001], BLOCK);

    // coefficient source
    bus.write(DPS310_ADDR, &[0x28, 0], BLOCK);

    //println!("dps310 status: {:x?}", get_status(bus));
    //println!("-- COEFFICIENTS --\n{:#?}", read_coefficients(bus));

    // interrupt & FIFO config for oversampling
    bus.write(DPS310_ADDR, &[0x9, 0b1100], BLOCK);

    while !get_status(bus).unwrap().sens_ready {
        FreeRtos::delay_ms(1);
    }

    // calibration source (set to use external temperature sensor)
    // NOTE: internal temperature sensor doesn't work well
    bus.write(DPS310_ADDR, &[0x28, 1u8 << 7], BLOCK);

    // pressure oversampling (rate is 64 times)
    bus.write(DPS310_ADDR, &[0x6, 0b0110], BLOCK);

    // temperature oversampling (rate is 64 times)
    // And set to use external temperature sensor
    bus.write(DPS310_ADDR, &[0x7, 0b0110 | (1u8 << 7)], BLOCK);
}

pub fn get_status(bus: &mut I2cDriver) -> Result<Dps310Status> {
    let mut status = [0u8; 1];
    bus.write_read(0x77, &[Dps310Regs::Status as u8], &mut status, BLOCK)?;
    Ok(Dps310Status::from_u8(status[0]))
}

#[derive(Debug)]
struct Dps310Status {
    coef_ready: bool,
    sens_ready: bool,
    temp_ready: bool,
    prs_ready: bool,
    ctr: u8,
}

impl Dps310Status {
    fn from_u8(data: u8) -> Self {
        Self {
            coef_ready: data & (1 << 7) != 0,
            sens_ready: data & (1 << 6) != 0,
            temp_ready: data & (1 << 5) != 0,
            prs_ready: data & (1 << 4) != 0,
            ctr: data & 0b111
        }
    }
}

#[derive(Debug)]
pub struct Dps310Coefficients {
    c0: i16, // 12bit
    c1: i16, // 12bit
    c00: i32, // 20bit
    c10: i32, // 20bit
    c01: i16, // 16bit
    c11: i16, // 16bit
    c20: i16, // 16bit
    c21: i16, // 16bit
    c30: i16, // 16bit
}

pub fn read_coefficients(bus: &mut I2cDriver) -> Result<Dps310Coefficients> {
    while !get_status(bus)?.coef_ready {
        FreeRtos::delay_ms(1);
    }

    let mut c0_11_4 = [0u8; 1];
    let mut c0_3_0__c1_11_8 = [0u8; 1];
    let mut c1_7_0 = [0u8; 1];
    let mut c00_19_12 =  [0u8; 1];
    let mut c00_11_4 =  [0u8; 1];
    let mut c00_3_0__c10_19_16 =  [0u8; 1];
    let mut c10_15_8 =  [0u8; 1];
    let mut c10_7_0 =  [0u8; 1];
    let mut c01_15_8 =  [0u8; 1];
    let mut c01_7_0 =  [0u8; 1];
    let mut c11_15_8 =  [0u8; 1];
    let mut c11_7_0 =  [0u8; 1];
    let mut c20_15_8 =  [0u8; 1];
    let mut c20_7_0 =  [0u8; 1];
    let mut c21_15_8 =  [0u8; 1];
    let mut c21_7_0 =  [0u8; 1];
    let mut c30_15_8 =  [0u8; 1];
    let mut c30_7_0 =  [0u8; 1];
    bus.write_read(DPS310_ADDR, &[0x10], &mut c0_11_4, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x11], &mut c0_3_0__c1_11_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x12], &mut c1_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x13], &mut c00_19_12, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x14], &mut c00_11_4, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x15], &mut c00_3_0__c10_19_16, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x16], &mut c10_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x17], &mut c10_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x18], &mut c01_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x19], &mut c01_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1a], &mut c11_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1b], &mut c11_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1c], &mut c20_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1d], &mut c20_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1e], &mut c21_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1f], &mut c21_7_0, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x20], &mut c30_15_8, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x21], &mut c30_7_0, BLOCK);

    //println!("c1: {:b} {:b}", c0_3_0__c1_11_8[0] & 0xf, c1_7_0[0]);
    Ok(Dps310Coefficients {
        c0: sign_ext_12bit(((c0_11_4[0] as u16) << 4)
                | ((((c0_3_0__c1_11_8[0] & 0xf0) as u16) >> 4) & 0xf)),
        c1: sign_ext_12bit((((c0_3_0__c1_11_8[0] & 0x0f) as u16) << 8)
                | c1_7_0[0] as u16),
        c00: sign_ext_20bit(((c00_19_12[0] as u32) << 12)
                | ((c00_11_4[0] as u32) << 4)
                | (((c00_3_0__c10_19_16[0] as u32) >> 4) & 0xf)),
        c10: sign_ext_20bit((((c00_3_0__c10_19_16[0] & 0xf) as u32) << 16)
            | ((c10_15_8[0] as u32) << 8)
            | (c10_7_0[0] as u32)),
        c01: (((c01_15_8[0] as u16) << 8) | (c01_7_0[0] as u16)) as i16,
        c11: (((c11_15_8[0] as u16) << 8) | (c11_7_0[0] as u16)) as i16,
        c20: (((c20_15_8[0] as u16) << 8) | (c20_7_0[0] as u16)) as i16,
        c21: (((c21_15_8[0] as u16) << 8) | (c21_7_0[0] as u16)) as i16,
        c30: (((c30_15_8[0] as u16) << 8) | (c30_7_0[0] as u16)) as i16,
    })
}

pub fn read_pressure(bus: &mut I2cDriver) -> Result<i32> {
    bus.write(DPS310_ADDR, &[Dps310Regs::Status as u8, 0b001], BLOCK)?;

    let mut x = [0u8;1];
    bus.write_read(DPS310_ADDR, &[0x6], &mut x, BLOCK)?;
    //println!("PRS_CFG: {:b}", x[0]);


    while !get_status(bus)?.prs_ready {
        FreeRtos::delay_ms(1);
    }

    let mut prs_b0 = [0u8; 1];
    let mut prs_b1 = [0u8; 1];
    let mut prs_b2 = [0u8; 1];

    bus.write_read(DPS310_ADDR, &[0x0], &mut prs_b2, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x1], &mut prs_b1, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x2], &mut prs_b0, BLOCK);

    Ok(sign_ext_24bit(((prs_b2[0] as u32) << 16)
            | ((prs_b1[0] as u32) << 8)
            | (prs_b0[0] as u32)))
}

pub fn read_temprature(bus: &mut I2cDriver) -> Result<i32> {
    bus.write(DPS310_ADDR, &[Dps310Regs::Status as u8, 0b010], BLOCK)?;

    let mut x = [0u8;1];
    bus.write_read(DPS310_ADDR, &[0x7], &mut x, BLOCK)?;
    //println!("TMP_CFG: {:b}", x[0]);

    while !get_status(bus)?.temp_ready {
        FreeRtos::delay_ms(1);
    }

    let mut tmp_b0 = [0u8; 1];
    let mut tmp_b1 = [0u8; 1];
    let mut tmp_b2 = [0u8; 1];

    bus.write_read(DPS310_ADDR, &[0x3], &mut tmp_b2, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x4], &mut tmp_b1, BLOCK);
    bus.write_read(DPS310_ADDR, &[0x5], &mut tmp_b0, BLOCK);

    let tmp = ((tmp_b2[0] as u32) << 16)
            | ((tmp_b1[0] as u32) << 8)
            | (tmp_b0[0] as u32);
    //println!("raw tmp: {:b}, sign extended: {:b}", tmp, sign_ext_24bit(tmp));
    Ok(sign_ext_24bit(((tmp_b2[0] as u32) << 16)
            | ((tmp_b1[0] as u32) << 8)
            | (tmp_b0[0] as u32)))
}

#[inline]
fn sign_ext_12bit(x: u16) -> i16 {
    if x & (1u16 << 11) != 0 {
        (x | (u16::MAX << 11)) as i16
    } else {
        x as i16
    }
}

#[inline]
fn sign_ext_20bit(x: u32) -> i32 {
    if x & (1u32 << 19) != 0 {
        (x | (u32::MAX << 19)) as i32
    } else {
        x as i32
    }
}

#[inline]
fn sign_ext_24bit(x: u32) -> i32 {
    if x & (1u32 << 23) != 0 {
        (x | (u32::MAX << 23)) as i32
    } else {
        x as i32
    }
}

const COMP_SCALE_128: f32 = 2088960.0_f32;
const COMP_SCALE_64: f32 = 1040384.0_f32;
const COMP_SCALE_1: f32 = 524288.0_f32;

pub fn comp_temp_val(x: i32, coef: &Dps310Coefficients) -> f32 {
    (coef.c0 as f32 / 2f32) + (scale_temp_val(x) * coef.c1 as f32)
}

pub fn scale_temp_val(x: i32) -> f32 {
    x as f32 / COMP_SCALE_64
}

pub fn comp_prs_val(rawprs: i32, rawtemp: i32, coef: &Dps310Coefficients) -> f32 {
    let p_raw_sc = rawprs as f32 / COMP_SCALE_64;
    let t_raw_sc = scale_temp_val(rawtemp);

    // c00 + Praw_sc*(c10 + Praw_sc *(c20+ Praw_sc *c30)) + Traw_sc *c01 +
    // Traw_sc *Praw_sc *(c11+Praw_sc*c21)
    coef.c00 as f32
        + p_raw_sc as f32
            * (coef.c10 as f32 + p_raw_sc as f32 * (coef.c20 as f32 + p_raw_sc as f32 * coef.c30 as f32))
        + t_raw_sc as f32 * coef.c01 as f32
        + t_raw_sc as f32 * p_raw_sc as f32
            * (coef.c11 as f32 + p_raw_sc as f32 * coef.c21 as f32)
}
