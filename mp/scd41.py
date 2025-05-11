from machine import Pin, I2C
import time
import struct

SCD41_ADDR = 0x62

bus = I2C(0, scl=Pin(19, pull=Pin.PULL_UP), sda=Pin(21, pull=Pin.PULL_UP), freq=400000, timeout=1000000)

def start_periodic_measurement():
    bus.writeto(SCD41_ADDR, b'\x21\xb1')

def stop_periodic_measurement():
    bus.writeto(SCD41_ADDR, b'\x3f\x86')

def is_data_ready():
    bus.writeto(SCD41_ADDR, b'\xe4\xb8', False)
    res = bus.readfrom(SCD41_ADDR, 3)
    word = struct.unpack(">H", res)
    # print(f'data status: {word[0]:x}')
    return ((word[0] & 0b11111111111) != 0)

def read_measurement():
    bus.writeto(SCD41_ADDR, b'\xec\x05', False)
    res = bus.readfrom(SCD41_ADDR, 9)
    words = struct.unpack(">HBHBHB", res)
    for i in range(0, 3):
        if not crc_check(res[i*3:i*3+3]):
            print("CRC check failed!!!")

    return {'co2': words[0],
     'temp': temp_conv(words[2]),
     'humidity': humidity_conv(words[4])}

def temp_conv(data):
    return -45 + 175 * (data / 0xffff)

def humidity_conv(data):
    return 100 * data / 0xffff


# uint8_t sensirion_common_generate_crc(const uint8_t *data, uint16_t count) {
  # uint16_t current_byte;
  # uint8_t crc = CRC8_INT;
  # uint8_t crc_bit;
  # for (current_byte = 0; current_byte < count; ++current_byte) {
    # crc ^= (data[current_byte]);
    # for (crc_bit = 8; crc_bit > 0; --crc_bit) {
      # if (crc & 0x80)
        # crc = (crc << 1) ^ CRC8_POLYNOMIAL;
      # else
        # crc = (crc << 1);
    # }
  # }
  # return crc;
# }

CRC8_POLYNOMIAL = 0x31
CRC8_INT = 0xff
def sensirion_common_generate_crc(data: bytes, count: int):
    crc = CRC8_INT
    for i in range(0, count):
        crc ^= data[i]
        for _ in range(0, 8):
            if (crc & 0x80) != 0:
                crc = (crc << 1) ^ CRC8_POLYNOMIAL
            else:
                crc = crc << 1
    return crc & 0xff


def crc_check(data):
    return sensirion_common_generate_crc(
           data, 3) == 0

print("Hello, world!!")
if SCD41_ADDR not in bus.scan():
    print("SCD41 not found")
    print("devs: ", bus.scan())
    while True:
        time.sleep(1)
stop_periodic_measurement()
time.sleep(0.5)
start_periodic_measurement()
time.sleep(0.5)
while True:
    time.sleep(1)
    while not is_data_ready():
        time.sleep(0.1)
    res = read_measurement()
    print(f"{res['co2']} ppm, {res['temp']} °C, {res['humidity']} RH")
