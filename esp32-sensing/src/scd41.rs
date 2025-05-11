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
    for i in 0..3 {
        if !check_crc(&res[i*3..i*3+3]) {
            panic!("CRC check failed!");
        }
    }
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


const CRC8_POLYNOMIAL: u8 =  0x31;
const CRC8_INT: u8 = 0xff;
// CRC calculation routine, as found in the scd41 datasheet
//uint8_t sensirion_common_generate_crc(const uint8_t *data, uint16_t count) {
  //uint16_t current_byte;
  //uint8_t crc = CRC8_INT;
  //uint8_t crc_bit;
  //for (current_byte = 0; current_byte < count; ++current_byte) {
    //crc ^= (data[current_byte]);
    //for (crc_bit = 8; crc_bit > 0; --crc_bit) {
      //if (crc & 0x80)
        //crc = (crc << 1) ^ CRC8_POLYNOMIAL;
      //else
        //crc = (crc << 1);
    //}
  //}
  //return crc;
//}

fn check_crc(data: &[u8]) -> bool {
    let mut crc = CRC8_INT;
    for i in 0..data.len() {
        crc ^= data[i];
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ CRC8_POLYNOMIAL;
            } else {
                crc = crc << 1;
            }
        }
    }
    crc & 0xff == 0
}
