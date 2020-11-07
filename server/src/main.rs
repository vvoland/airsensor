extern crate btleplug;

use btleplug::api::{Central, CentralEvent, Peripheral, BDAddr, UUID, Characteristic, ValueNotification};
#[cfg(target_os = "linux")]
use btleplug::bluez::{adapter::ConnectedAdapter, manager::Manager};
#[cfg(target_os = "macos")]
use btleplug::corebluetooth::{adapter::Adapter, manager::Manager};
#[cfg(target_os = "windows")]
use btleplug::winrtble::{adapter::Adapter, manager::Manager};

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().unwrap();
    adapters.into_iter().nth(0).expect("No BLE adapters");
}

#[cfg(target_os = "linux")]
fn get_central(manager: &Manager) -> ConnectedAdapter {
    let adapters = manager.adapters().unwrap();
    let adapter = adapters.into_iter().nth(0).expect("No BLE adapters");
    adapter.connect().unwrap()
}

use std::time::SystemTime;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Scope, rt::System};
use std::vec::Vec;
use std::sync::{mpsc, Arc, Mutex};
use std::cell::RefCell;
use std::sync::atomic::{Ordering, AtomicBool};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};


#[get("/")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body("Server is up and running!")
}

#[derive(Serialize, Deserialize)]
struct SensorsList {
    pub sensors: Vec::<String>
}

#[get("/list")]
async fn sensors_list()  -> HttpResponse {
    HttpResponse::Ok()
        .json(SensorsList {
            sensors: Vec::<String>::new()
        })
}

async fn wait_for_keyboard_interrupt(mut wait_action: Box<dyn FnMut()>) -> () {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&stop)).expect("Failed to register SIGINT hook");

    while !stop.load(Ordering::Relaxed) {
        wait_action();
    }
}

struct BleMaster<P: Peripheral> {
    to_inspect: Mutex<Vec<P>>,
    sensors: Mutex<Vec<AlphaSensor<P>>>

}

struct AlphaSensor<P: Peripheral> {
    peripheral: P,
    characteristic: Characteristic,
    data_receiver: mpsc::Receiver<Vec<u8>>
}

#[derive(Debug, Eq, PartialEq)]
enum AlphaSensorPollError {
    UnexpectedResponse,
    SensorError,
    Timeout,
    SendFailed
}

struct AlphaSensorReading {
    pub temperature: i8,
    pub humidity: u8
}

impl<P: Peripheral> AlphaSensor<P> {
    fn try_new(peripheral: P, characteristic: Characteristic) -> Option<Self> {

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
                peripheral, characteristic,
                data_receiver: rx
            })
        } else {
            None
        }
    }

    fn check_hello(peripheral: &P, characteristic: &Characteristic, rx: &mpsc::Receiver<Vec<u8>>) -> bool {
        if let Err(_) = peripheral.command(characteristic, &[0x10u8]) {
            return false;
        }
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(data) => {
                if data.eq(&vec![0x00u8, 0xF0u8, 0x14u8, 0x4Du8]) {
                    true
                } else {
                    println!("Hello data did not match!");
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
            .map_or(Err(AlphaSensorPollError::Timeout), 
                |data| {
                    if data.len() == 4 {
                        if data[0] == 0x00u8 {
                            let temperature = i8::from_le_bytes([data[1]]);
                            let humidity = data[2];
                            Ok(AlphaSensorReading { temperature, humidity })
                        } else {
                            Err(AlphaSensorPollError::SensorError)
                        }
                    } else {
                        Err(AlphaSensorPollError::UnexpectedResponse)
                    }
                }
            )
    }
}

impl<P: Peripheral> Drop for AlphaSensor<P> {
    fn drop(&mut self) {
        println!("Disconnecting dropped sensor {}...", self.peripheral.address());
        let _ = self.peripheral.disconnect();
    }
}

impl<P: Peripheral> BleMaster<P> {

    pub fn new() -> Self {
        BleMaster::<P> {
            to_inspect: Mutex::new(Vec::<P>::new()),
            sensors: Mutex::new(Vec::<AlphaSensor<P>>::new())
        }
    }

    pub fn on_lost(&mut self, address: BDAddr) {
        {
            let mut data = self.sensors.lock().expect("Poisoned mutex");
            data.retain(|sensor| sensor.peripheral.address() != address);
        }
        {
            let mut data = self.to_inspect.lock().expect("Poisoned mutex");
            data.retain(|peripheral| peripheral.address() != address);
        }
    }

    pub fn on_discovered(&mut self, peripheral: P) {
        if peripheral.properties().local_name.map_or(true, |name| !name.contains("Weather")) {
            return
        }

        let mut to_inspect = self.to_inspect.lock().expect("Poisoned mutex");
        to_inspect.push(peripheral);
    }

    pub fn pop_and_inspect(&mut self) {
        let mut to_inspect = self.to_inspect.lock().expect("Poisoned mutex");

        if let Some(peripheral) = to_inspect.pop() {
            self.inspect(peripheral);
        }
    }

    pub fn inspect(&self, peripheral: P) {

        println!("Connecting to {} {}...", peripheral.address(), peripheral.properties().local_name.unwrap());

        let characteristic = peripheral.connect()
            .and_then(|_| {
                println!("Discovering characteristics of {}...", peripheral.address());
                peripheral.discover_characteristics()
            })
            .map_or_else(
                |_| {
                    println!("Disconnecting {}...", peripheral.address());
                    if let Err(err) = peripheral.disconnect() {
                        println!("Could not disconnect from device {}, {}", peripheral.address(), err);
                    }
                    None
                },
                |characteristics| {
                    characteristics.iter()
                        .find(|c| c.uuid == UUID::B16(0xFFE1))
                        .map(|c| c.clone())
                }
            );

        if let Some(characteristic) = characteristic {
            println!("Found characteristics in {}", peripheral.address());
            let mut sensors = self.sensors.lock().expect("Poisoned mutex");
            if let Some(sensor) = AlphaSensor::try_new(peripheral.clone(), characteristic) {
                sensors.push(sensor);
            } else {
                let _ = peripheral.disconnect();
            }
        }

    }

}

fn build_http() -> actix_web::dev::Server {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(|| {
                let sensors_scope: Scope = web::scope("/api/sensors")
                    .service(sensors_list);

                App::new()
                    .service(status)
                    .service(sensors_scope)
            })
            .bind("0.0.0.0:8000")?
            .shutdown_timeout(60)
            .run();

        let _ = tx.send(srv);
        sys.run()
    });

    rx.recv().unwrap()
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let srv = build_http();
    

    let manager = Manager::new().unwrap();
    let central = get_central(&manager);

    central.start_scan().expect("Unable to start scan");

    thread::sleep(Duration::from_secs(2));

    let events = central.event_receiver().unwrap();
    let mut master = BleMaster::new();

    let mut prev = SystemTime::now();

    wait_for_keyboard_interrupt(Box::new(move || {
        match events.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => match event {
                CentralEvent::DeviceDiscovered(addr) => {
                    println!("{} discovered", addr);
                    match central.peripheral(addr) {
                        Some(peripheral) => { 
                            master.on_discovered(peripheral);
                        },
                        None => println!("* Failed to get the peripheral")
                    }
                },
                CentralEvent::DeviceLost(addr) => {
                    master.on_lost(addr);
                },
                _ => {}
            },
            _ => {}
        };
        master.pop_and_inspect();
        let dt = SystemTime::now().duration_since(prev).unwrap();
        if dt.as_secs() >= 10 {
            prev = SystemTime::now();
            let mut sensors = master.sensors.lock().expect("Poisoned mutex");
            sensors.retain(|sensor| {
                match sensor.poll() {
                    Ok(reading) => {
                        println!("Temperature: {}C, Humidity: {}%", reading.temperature, reading.humidity);
                        true
                    }
                    Err(AlphaSensorPollError::SendFailed) => {
                        let mut to_inspect = master.to_inspect.lock().unwrap();
                        to_inspect.push(sensor.peripheral.clone());
                        println!("Could not communicate with sensor");
                        false
                    }
                    Err(err) => {
                        println!("Could not poll sensor data! {:?}", err);
                        true
                    }
                }
            });
        }
    })).await;

    println!("Stopping the server...");
    srv.clone().stop(true).await;
    Ok(())
}
