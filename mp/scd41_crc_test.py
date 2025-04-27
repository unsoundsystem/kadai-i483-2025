
CRC8_POLYNOMIAL = 0x31
CRC8_INT = 0xff
def sensirion_common_generate_crc(data: bytes, count: int):
    print(f"crc: data: {data}, count: {count}")
    crc = CRC8_INT
    for i in range(0, count):
        crc ^= data[i]
        for _ in range(0, 8):
            if (crc & 0x80) != 0:
                crc = (crc << 1) ^ CRC8_POLYNOMIAL
            else:
                crc = crc << 1
    return crc & 0xff

print(f"{sensirion_common_generate_crc(bytes([0xbe, 0xef, 0x92]), 3):x}")
