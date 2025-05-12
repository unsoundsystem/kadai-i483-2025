#include <endian.h>
#include <stdio.h>
#include <string.h>

#include <inttypes.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_flash.h"
#include "esp_log.h"
#include "esp_system.h"

#include "sdkconfig.h"

static char *TAG = "rpr0521rs measurement";

#define I2C_MASTER_SCL_IO 19
#define I2C_MASTER_SDA_IO 21

#define I2C_ADDR_RPR0521RS 0x38

#define RPR_MODE_CONTROL 0x41
// 0b10001010
#define RPR_MODE_VAL 0x8a 
#define RPR_SYSTEM_CONTROL 0x40
#define RPR_ALS_PS_CONTROL 0x42
#define RPR_ALS_PS_CONTROL_VAL 0x3
#define RPR_ALS_DATA_0_LSBs 0x46
#define RPR_ALS_DATA_0_MSBs 0x47
#define RPR_ALS_DATA_1_LSBs 0x48
#define RPR_ALS_DATA_1_MSBs 0x49

i2c_master_bus_handle_t bus;
i2c_master_dev_handle_t rpr0521rs;


void setup_i2c(void) {
  i2c_master_bus_config_t bus_config = {.clk_source = I2C_CLK_SRC_DEFAULT,
                                        .glitch_ignore_cnt = 7,
                                        .i2c_port = 0,
                                        .scl_io_num = I2C_MASTER_SCL_IO,
                                        .sda_io_num = I2C_MASTER_SDA_IO,
                                        .flags.enable_internal_pullup = true};

  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_config, &bus));

  esp_err_t result = i2c_master_probe(bus, I2C_ADDR_RPR0521RS, 1000);
  if (ESP_OK == result) {
    printf("rpr0521rs detected!\n");
    i2c_master_bus_add_device(
        bus,
        &((const i2c_device_config_t){.device_address = I2C_ADDR_RPR0521RS,
                                      .dev_addr_length = I2C_ADDR_BIT_LEN_7,
                                      .scl_speed_hz = 100000
									  }),
        &rpr0521rs);
  } else {
    printf("failed to detect rpr0521rs!(error: %x)", result);
    esp_restart();
  }
}

void setup_rpr() {
	uint8_t buf = RPR_SYSTEM_CONTROL;
	esp_err_t res = i2c_master_transmit_receive(rpr0521rs, &buf, sizeof(buf), &buf, sizeof(buf), 100);
	ESP_ERROR_CHECK(res);

	if ((buf & 0xfff) != 0b001010) {
		printf("part id not met\n");
	}

	uint8_t write_buf[2] = {0x40, 0x80};
	res = i2c_master_transmit(rpr0521rs, write_buf, sizeof(write_buf), 100);
	ESP_ERROR_CHECK(res);

	vTaskDelay(500 / portTICK_PERIOD_MS);

	uint8_t write_buf1[2] = {RPR_MODE_CONTROL, RPR_MODE_VAL};
	res = i2c_master_transmit(rpr0521rs, write_buf1, sizeof(write_buf1), 100);
	ESP_ERROR_CHECK(res);

	vTaskDelay(500 / portTICK_PERIOD_MS);

	uint8_t write_buf2[2] = {RPR_ALS_PS_CONTROL, RPR_ALS_PS_CONTROL_VAL};
	res = i2c_master_transmit(rpr0521rs, write_buf2, sizeof(write_buf2), 100);
	ESP_ERROR_CHECK(res);

	printf("setup rpr0521rs done!\n");
}

void app_main(void) {
  printf("Hello world!\n");
  setup_i2c();
  setup_rpr();

  esp_err_t res;
  while(1) {
	  vTaskDelay(1000 / portTICK_PERIOD_MS);

	  uint8_t als0_lsb_read_buf[] = {0};
	  uint8_t als0_msb_read_buf[] = {0};
	  uint8_t als0_lsb_write_buf[1] = {RPR_ALS_DATA_0_LSBs};
	  uint8_t als0_msb_write_buf[1] = {RPR_ALS_DATA_0_MSBs};
	  res = i2c_master_transmit_receive(rpr0521rs, als0_lsb_write_buf, sizeof(als0_lsb_write_buf),
			  als0_lsb_read_buf, sizeof(als0_lsb_read_buf), 100);
	  ESP_ERROR_CHECK(res);
	  res = i2c_master_transmit_receive(rpr0521rs, als0_msb_write_buf, sizeof(als0_msb_write_buf),
			  als0_msb_read_buf, sizeof(als0_msb_read_buf), 100);
	  ESP_ERROR_CHECK(res);
	  uint16_t als0_data = ((uint16_t)als0_msb_read_buf[0] << 8) | als0_lsb_read_buf[0];

	  uint8_t als1_lsb_read_buf[] = {0};
	  uint8_t als1_msb_read_buf[] = {0};
	  uint8_t als1_lsb_write_buf[1] = {RPR_ALS_DATA_1_LSBs};
	  uint8_t als1_msb_write_buf[1] = {RPR_ALS_DATA_1_MSBs};
	  res = i2c_master_transmit_receive(rpr0521rs, als1_lsb_write_buf, sizeof(als1_lsb_write_buf),
			  als1_lsb_read_buf, sizeof(als1_lsb_read_buf), 100);
	  ESP_ERROR_CHECK(res);
	  res = i2c_master_transmit_receive(rpr0521rs, als1_msb_write_buf, sizeof(als1_msb_write_buf),
			  als1_msb_read_buf, sizeof(als1_msb_read_buf), 100);
	  ESP_ERROR_CHECK(res);
	  uint16_t als1_data = ((uint16_t)als1_msb_read_buf[0] << 8) | als0_lsb_read_buf[0];

	  printf("%f lx (infrated: %f)\n",
			  (double)als0_data * ((double)100 / (double)400),
			  (double)als0_data * ((double)100 / (double)400)
			);
  }

}
