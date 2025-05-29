mod bh1750;
mod dps310;
mod rpr0521rs;
mod scd41;
mod wifi;
use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals, delay::FreeRtos, i2c::{I2cConfig, I2cDriver}, units::*},
  sys::link_patches,
  nvs::EspDefaultNvsPartition,
  eventloop::EspSystemEventLoop,
  mqtt::client::{Details, EspMqttClient, MqttClientConfiguration, QoS},
};
use wifi::wifi;
use crate::dps310::Dps310Coefficients;
use std::sync::{Mutex, Arc, mpsc::channel};

// 15000 - (sensor interaction overhead) 
const DATA_COLLECTION_INTERVAL_MS: u32 = 15000;
const MQTT_TOPIC_PREFIX: &'static str = "i483/sensors/s2510030";

#[derive(Debug)]
enum SensorData {
    Scd41 { temperature: f64, humidity: f64, co2: u16 },
    Bh1750 { illumination: f64 },
    Rpr0521 { illumination: f64, infrared_illumination: f64},
    Dps310 { pressure: f32, temperature: f32}
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio19;

    log::info!("Starting I2C");

    // speed down
    let config = I2cConfig::new().baudrate(KiloHertz(100).into());
    let (sender, receiver) = channel::<SensorData>();
    let mut i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    // WiFi setup
    let wifi = wifi(
        "JAISTALL",
        "",
        peripherals.modem,
        sysloop,
        nvs,
    ).expect("failed to connect WiFi");

    // server url: 150.65.230.59
    let mqtt_config = MqttClientConfiguration::default();
    let broker_url = "mqtt://150.65.230.59";
    let mut mqtt_client = EspMqttClient::new(broker_url, &mqtt_config).unwrap();


    log::info!("Setting up sensors");
    bh1750::setup(&mut i2c);
    FreeRtos::delay_ms(500);
    dps310::setup(&mut i2c);
    FreeRtos::delay_ms(500);
    rpr0521rs::setup(&mut i2c);
    FreeRtos::delay_ms(500);
    scd41::setup(&mut i2c);
    FreeRtos::delay_ms(500);
    let dps_coef = dps310::read_coefficients(&mut i2c).unwrap();
    log::info!("Sensor setup done");
    log::info!("Waiting for sensors to generate first measurement...");
    FreeRtos::delay_ms(5000);

    //print_csv(&mut i2c, &dps_coef);
    let i2c_handle = Arc::new(Mutex::new(i2c));

    let builder = std::thread::Builder::new().stack_size(4096);

    let i2c = Arc::clone(&i2c_handle);
    let tx = sender.clone();
    let h4 = std::thread::Builder::new().stack_size(4096).spawn(move || {

        loop {
            FreeRtos::delay_ms(DATA_COLLECTION_INTERVAL_MS);
            let mut i2c = i2c.lock().unwrap();
            FreeRtos::delay_ms(1);

            // NOTE: Even restarting there wasn't help
            //let mut i = 0usize;
            //while !scd41::is_data_ready(&mut i2c).unwrap() {
                //i += 1;
                //FreeRtos::delay_ms(1);
                //if i % 1000 == 0 {
                    //log::warn!("Waiting for SCD41");
                //}
                //if i % 5000 == 0 && i > 0 {
                    //log::warn!("Trying to restart SCD41");
                    //scd41::setup(&mut i2c);
                    //FreeRtos::delay_ms(5000);
                    //continue;
                //}
            //}

            if scd41::is_data_ready(&mut i2c).unwrap() {
                let (co2, temp, hum) = scd41::perform_measurement(&mut i2c).unwrap();

                tx.send(SensorData::Scd41 {
                    co2,
                    temperature: temp,
                    humidity: hum
                }).expect("failed to enqueue");
            } else {
                log::error!("failed to measure SCD41");
            }
        }
    });


    let i2c = Arc::clone(&i2c_handle);
    let tx = sender.clone();
    let h = builder.spawn(move || {
        let i2c = Arc::clone(&i2c);
        loop {
            FreeRtos::delay_ms(DATA_COLLECTION_INTERVAL_MS);
            let mut i2c = i2c.lock().unwrap();

            FreeRtos::delay_ms(1);
            // DPS310
            let rawtmp = dps310::read_temprature(&mut i2c).unwrap();
            //println!("[DPS310] tmp: {:.2} °C, rawtmp: {}", dps310::comp_temp_val(rawtmp, &dps_coef), rawtmp);
            //println!("[DPS310] tmp: {:.2} °C", dps310::comp_temp_val(rawtmp, &dps_coef));
            FreeRtos::delay_ms(100);
            let rawprs = dps310::read_pressure(&mut i2c).unwrap();
            //println!("[DPS310] prs: {} hPa", dps310::comp_prs_val(rawprs, rawtmp, &dps_coef) / 100f32);
            tx.send(SensorData::Dps310 {
                pressure: dps310::comp_prs_val(rawprs, rawtmp, &dps_coef) / 100f32,
                temperature: dps310::comp_temp_val(rawtmp, &dps_coef)
            }).expect("failed to enqueue");
        }
    });

    let i2c = Arc::clone(&i2c_handle);
    let tx = sender.clone();
    let h2 = std::thread::Builder::new().stack_size(4096).spawn(move || {
        loop {
            FreeRtos::delay_ms(DATA_COLLECTION_INTERVAL_MS);
            let mut i2c = i2c.lock().unwrap();
            FreeRtos::delay_ms(1);
            let rawlx = bh1750::perform_measurement(&mut i2c).unwrap();
            //println!("[BH1750] lux: {:.2}", bh1750::calc_lux(rawlx));
            tx.send(SensorData::Bh1750 {
                illumination: bh1750::calc_lux(rawlx)
            }).expect("failed to enqueue");
        }
    });

    let i2c = Arc::clone(&i2c_handle);
    let tx = sender.clone();
    let h3 = std::thread::Builder::new().stack_size(4096).spawn(move || {
        loop {
            FreeRtos::delay_ms(DATA_COLLECTION_INTERVAL_MS);
            let mut i2c = i2c.lock().unwrap();
            FreeRtos::delay_ms(1);
            let (lx, inflx) = rpr0521rs::perform_measurement(&mut i2c).unwrap();
            tx.send(SensorData::Rpr0521 {
                illumination: lx,
                infrared_illumination: inflx,
            }).expect("failed to enqueue");
        }
    });

    //let mqtt_client = Arc::clone(&mqtt_client);
    let consumer = std::thread::Builder::new().stack_size(8192).spawn(move || {
        while !wifi.is_connected().unwrap() {
            let config = wifi.get_configuration().unwrap();
            println!("Waiting for station {:?}", config);
        }
        loop {
            FreeRtos::delay_ms(1);
            while let Ok(data) = receiver.recv() {
                //let mut mqtt_client = mqtt_client.lock().unwrap();
                log::info!("{:?}", data);
                match data {
                    SensorData::Dps310 { pressure, temperature } => {
                        publish_data(&mut mqtt_client.0, "DPS310", "temperature", &format!("{:.2}", temperature));
                        publish_data(&mut mqtt_client.0, "DPS310", "air_pressure", &format!("{:.2}", pressure));
                    }
                    SensorData::Scd41 { co2, temperature, humidity } => {
                        publish_data(&mut mqtt_client.0, "SCD41", "temperature", &format!("{:.2}", temperature));
                        publish_data(&mut mqtt_client.0, "SCD41", "humidity", &format!("{:.2}", humidity));
                        publish_data(&mut mqtt_client.0, "SCD41", "co2", &format!("{co2}"));
                    }
                    SensorData::Rpr0521 { illumination, infrared_illumination } => {
                        publish_data(&mut mqtt_client.0, "RPR0521", "illumination", &format!("{illumination}"));
                        publish_data(&mut mqtt_client.0, "RPR0521", "infrared_illumination", &format!("{infrared_illumination}"));
                    }
                    SensorData::Bh1750 { illumination } => {
                        publish_data(&mut mqtt_client.0, "BH1750", "illumination", &format!("{illumination}"));
                    }
                }
            }
        }
    });
    Ok(())
}

fn publish_data(mqtt_client: &mut EspMqttClient, sensor: &str, data_type: &str, data: &str) {
    mqtt_client.publish(
        &format!("{MQTT_TOPIC_PREFIX}/{sensor}/{data_type}"),
        QoS::AtLeastOnce,
        false,
        data.as_bytes()
    );
}

fn print_csv(i2c: &mut I2cDriver, dps_coef: &Dps310Coefficients) {

    // DPS310_temp, DPS310_pres, BH1750_lx, RPR0521RS_lx, RPR0521RS_inf, SCD41_co2, SCD41_temp, SCD41_humidity
    println!("DPS310_temp,DPS310_pres,BH1750_lx,RPR0521RS_lx,RPR0521RS_inf,SCD41_co2,SCD41_temp,SCD41_humidity");
    loop {
        FreeRtos::delay_ms(DATA_COLLECTION_INTERVAL_MS);

        // DPS310
        let rawtmp = dps310::read_temprature(i2c).unwrap();
        let dps_temp = dps310::comp_temp_val(rawtmp, dps_coef);
        FreeRtos::delay_ms(100);
        let rawprs = dps310::read_pressure(i2c).unwrap();
        let dps_pres = dps310::comp_prs_val(rawprs, rawtmp, dps_coef) / 100f32;

        // BH1750
        let rawlx = bh1750::perform_measurement(i2c).unwrap();
        let bh_lx = bh1750::calc_lux(rawlx);

        // rpr0521rs
        let (rpr_lx, rpr_inf) = rpr0521rs::perform_measurement(i2c).unwrap();

        // SCD41
        let mut scd_temp: f64;
        let mut scd_hum: f64;
        let mut scd_co2: u16;
        while !scd41::is_data_ready(i2c).unwrap() {
            FreeRtos::delay_ms(1);
        }
        let (co2, temp, hum) = scd41::read_measurement(i2c).unwrap();
        scd_temp = scd41::temp_comp(temp);
        scd_co2 = co2;
        scd_hum = scd41::humidity_comp(hum);

        // DPS310_temp, DPS310_pres, BH1750_lx, RPR0521RS_lx, RPR0521RS_inf, SCD41_co2, SCD41_temp, SCD41_humidity
        println!("{dps_temp:.2},{dps_pres:.2},{bh_lx:.2},{rpr_lx:.2},{rpr_inf:.2},{scd_co2},{scd_temp:.2},{scd_hum:.2}");
    }
}
