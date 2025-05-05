from machine import I2C, Pin

bus = I2C(0, scl=Pin(19, pull=Pin.PULL_UP), sda=Pin(21, pull=Pin.PULL_UP), freq=400000, timeout=1000000)

print(bus.scan())
