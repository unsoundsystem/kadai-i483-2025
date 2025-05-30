
use rumqttc::{MqttOptions, Client, QoS, Incoming, Event};
use std::time::Duration;
use std::thread;

fn main() {
    let mut mqttoptions = MqttOptions::new("i483-mqtt-recv", "150.65.230.59", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let topics = vec![
        "i483/sensors/s2510030/DPS310/temperature",
        "i483/sensors/s2510030/DPS310/air_pressure",
        "i483/sensors/s2510030/SCD41/co2",
        "i483/sensors/s2510030/SCD41/temperature",
        "i483/sensors/s2510030/SCD41/humidity",
        "i483/sensors/s2510030/BH1750/illumination",
        "i483/sensors/s2510030/RPR0521/illumination",
        "i483/sensors/s2510030/RPR0521/infrared_illumination"
    ];

    let (mut client, mut connection) = Client::new(mqttoptions, 10);

    for t in topics {
        client.subscribe(t, QoS::AtLeastOnce).unwrap()
    }

    for (i, notification) in connection.iter().enumerate() {
        if let Ok(Event::Incoming(Incoming::Publish(packet))) = notification {
            let message = str::from_utf8(packet.payload.as_ref()).unwrap();
            let topic = packet.topic;
            println!("{message} from {topic}");
        }
    }
}
