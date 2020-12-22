use btleplug::api::{Characteristic, Peripheral, UUID};
use std::sync::mpsc;
use std::time::Duration;

pub struct AlphaSensor<P: Peripheral> {
    pub peripheral: P,
    characteristic: Characteristic,
    data_receiver: mpsc::Receiver<Vec<u8>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum AlphaSensorPollError {
    UnexpectedResponse,
    SensorError,
    Timeout,
    SendFailed,
}

pub struct AlphaSensorReading {
    pub temperature: i8,
    pub humidity: u8,
}

impl<P: Peripheral> AlphaSensor<P> {
    pub fn try_new(peripheral: P, characteristic: Characteristic) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let notification_characteristic = characteristic.clone();
        peripheral.on_notification(Box::new(move |notification| {
            if notification.uuid == notification_characteristic.uuid {
                tx.send(notification.value).expect("Send failure");
            } else {
                println!("Unexpected notification uuid: {}", notification.uuid);
            }
        }));

        if Self::check_hello(&peripheral, &characteristic, &rx) {
            Some(AlphaSensor {
                peripheral,
                characteristic,
                data_receiver: rx,
            })
        } else {
            None
        }
    }

    pub fn inspect(peripheral: &P) -> Option<Characteristic> {
        println!(
            "Connecting to {} {}...",
            peripheral.address(),
            peripheral.properties().local_name.unwrap()
        );

        peripheral
            .connect()
            .and_then(|_| {
                println!("Discovering characteristics of {}...", peripheral.address());
                peripheral.discover_characteristics()
            })
            .map_or_else(
                |_| {
                    println!("Disconnecting {}...", peripheral.address());
                    if let Err(err) = peripheral.disconnect() {
                        println!(
                            "Could not disconnect from device {}, {}",
                            peripheral.address(),
                            err
                        );
                    }
                    None
                },
                |characteristics| {
                    characteristics
                        .iter()
                        .find(|c| c.uuid == UUID::B16(0xFFE1))
                        .map(|c| c.clone())
                },
            )
    }

    fn check_hello(
        peripheral: &P,
        characteristic: &Characteristic,
        rx: &mpsc::Receiver<Vec<u8>>,
    ) -> bool {
        if let Err(_) = peripheral.command(characteristic, &[0x10u8]) {
            return false;
        }
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(data) => {
                if data.eq(&vec![0x00u8, 0xF0u8, 0x14u8, 0x4Du8]) {
                    true
                } else {
                    println!("Hello data did not match! {:?}", data);
                    false
                }
            }
            Err(_) => {
                println!("Hello timeout");
                false
            }
        }
    }

    pub fn poll(&self) -> Result<AlphaSensorReading, AlphaSensorPollError> {
        if let Err(_) = self.peripheral.command(&self.characteristic, &[0x66u8]) {
            return Err(AlphaSensorPollError::SendFailed);
        }
        self.data_receiver
            .recv_timeout(Duration::from_secs(5))
            .map_or(Err(AlphaSensorPollError::Timeout), |data| {
                if data.len() == 4 {
                    if data[0] == 0x00u8 {
                        let temperature = i8::from_le_bytes([data[1]]);
                        let humidity = data[2];
                        Ok(AlphaSensorReading {
                            temperature,
                            humidity,
                        })
                    } else {
                        Err(AlphaSensorPollError::SensorError)
                    }
                } else {
                    Err(AlphaSensorPollError::UnexpectedResponse)
                }
            })
    }
}

impl<P: Peripheral> Drop for AlphaSensor<P> {
    fn drop(&mut self) {
        println!(
            "Disconnecting dropped sensor {}...",
            self.peripheral.address()
        );
        let _ = self.peripheral.disconnect();
    }
}
