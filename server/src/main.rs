extern crate btleplug;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
extern crate env_logger;

use btleplug::api::{Central, CentralEvent, Peripheral, BDAddr};
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
    manager.down(&adapter).expect("Failed to put adapter down");
    manager.up(&adapter).expect("Failed to put adapter up");
    adapter.connect().unwrap()
}

use std::time::Instant;
use actix_web::{web, App, HttpResponse, HttpServer, Scope, rt::System, middleware::Logger};
use std::vec::Vec;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::sync::atomic::{Ordering, AtomicBool};
use std::thread;
use std::time::Duration;
use chrono::Utc;

mod sensor;
use sensor::*;

mod alpha_sensor;
use alpha_sensor::*;

mod database;
use database::{Database, DatabaseError};

mod sqlite_database;
use sqlite_database::SqliteDatabase;

pub mod schema;
mod api;


struct BleMaster<P: Peripheral, D: Database, S: SensorsState> {
    to_inspect: Mutex<Vec<P>>,
    sensors: Mutex<Vec<AlphaSensor<P>>>,
    state: StatePtr<S>,
    db: D
}

struct AppState {
    sensors: Mutex<Vec<Sensor>>
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            sensors: Mutex::new(Vec::<Sensor>::new())
        }
    }
}

pub trait SensorsState {
    fn get_status(&self, sensor: &Sensor) -> SensorStatus;
    fn add(&mut self, sensor: Sensor);
    fn remove(&mut self, sensor: &Sensor) -> Result<(), ()>;
}

impl SensorsState for AppState {
    fn get_status(&self, sensor: &Sensor) -> SensorStatus {
        let data = self.sensors.lock().expect("Poisoned mutex");
        match data.contains(sensor) {
            true => SensorStatus::Online,
            false => SensorStatus::Offline
        }
    }

    fn add(&mut self, sensor: Sensor) {
        let mut data = self.sensors.lock().unwrap();
        if !data.contains(&sensor) {
            data.push(sensor);
        }
    }

    fn remove(&mut self, sensor: &Sensor) -> Result<(), ()> {
        let mut data = self.sensors.lock().unwrap();
        if let Some(idx) = data.iter().position(|i| i == sensor) {
            data.remove(idx);
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<P: Peripheral, D: Database, S: SensorsState> BleMaster<P, D, S> {

    pub fn new(db: D, state: StatePtr<S>) -> Self {
        BleMaster::<P, D, S> {
            db,
            state,
            to_inspect: Mutex::new(Vec::<P>::new()),
            sensors: Mutex::new(Vec::<AlphaSensor<P>>::new())
        }
    }

    pub fn on_disconnect(&mut self, address: BDAddr) {
        let is_not_lost = |peripheral: &P| {
            peripheral.address() != address
        };

        println!("Lost {}...", address);
        {
            let mut data = self.sensors.lock().expect("Poisoned mutex");
            let mut state = self.state.write().expect("Poisoned RwLock");
            data.iter()
                .filter(|sensor| !is_not_lost(&sensor.peripheral))
                .map(Self::sensor_from_alpha)
                .for_each(|sensor| {
                    match state.remove(&sensor) {
                        Ok(()) => println!("{:?} gone offline!", sensor.name),
                        Err(()) => println!("Could not remove {:?}!", sensor.name)
                    };
                });
            data.retain(|sensor| is_not_lost(&sensor.peripheral));
        }
        {
            let mut data = self.to_inspect.lock().expect("Poisoned mutex");
            data.retain(is_not_lost);
        }
    }

    pub fn on_discovered(&mut self, peripheral: P) {
        let mut to_inspect = self.to_inspect.lock().expect("Poisoned mutex");
        to_inspect.push(peripheral);
    }

    pub fn pop_and_inspect(&mut self) {
        let mut to_inspect = self.to_inspect.lock().expect("Poisoned mutex");

        if let Some(peripheral) = to_inspect.pop() {
            self.inspect(peripheral)
        }
    }

    fn sensor_from_alpha(alpha: &AlphaSensor<P>) -> Sensor {
        let properties = alpha.peripheral.properties();
        Sensor {
            family: SensorFamily::Alpha,
            address: properties.address.to_string(),
            name: properties.local_name
        }
    }

    pub fn inspect(&self, peripheral: P) {
        if peripheral.properties().local_name.map_or(true, |name| !name.contains("Weather")) {
            println!("Ignoring {}", peripheral.address());
            return
        }

        println!("Inspecting {}...", peripheral.address());

        if let Some(characteristic) = AlphaSensor::inspect(&peripheral) {
            println!("Found characteristics in {}", peripheral.address());
            let mut sensors = self.sensors.lock().expect("Poisoned mutex");
            if let Some(sensor) = AlphaSensor::try_new(peripheral.clone(), characteristic) {
                let domain_sensor = Self::sensor_from_alpha(&sensor);
                sensors.push(sensor);

                let mut state = self.state.write().unwrap();
                state.add(domain_sensor);
            } else {
                let _ = peripheral.disconnect();
            }
        }
    }

    pub fn try_poll_sensor(&self, sensor: &AlphaSensor<P>) -> bool {
        println!("Polling sensor...");
        match sensor.poll() {
            Ok(reading) => {
                println!("Polling ok");
                let now = Utc::now().naive_utc();
                let properties = sensor.peripheral.properties();
                let name_str = match properties.local_name.clone() {
                    Some(s) => s,
                    None => "???".to_string()
                };
                println!("[{}] Temperature: {}C, Humidity: {}%", name_str, reading.temperature, reading.humidity);

                let sensor_data = Sensor {
                    family: SensorFamily::Alpha,
                    address: properties.address.to_string(),
                    name: properties.local_name
                };

                self.db.create_sensor_if_not_exists(&sensor_data)
                    .expect("Could not ensure that the sensor exists in the database");
                let handle = self.db.get_sensor_handle(&sensor_data).expect("Failed to get handle to just added sensor");

                for reading in [
                    SensorReading::Temperature(reading.temperature as i32),
                    SensorReading::Humidity(reading.humidity)
                ].iter() {
                    loop {
                        match self.db.add_reading(&handle, now, reading) {
                            Ok(_) => break,
                            Err(DatabaseError::Busy) => thread::sleep(Duration::from_secs(1)),
                            Err(err) => panic!("Could not insert reading {:?} due to {:?}", reading, err)
                        }
                    }
                }

                true
            }
            Err(AlphaSensorPollError::SendFailed) => {
                println!("Polling err");
                let mut to_inspect = self.to_inspect.lock().unwrap();
                to_inspect.push(sensor.peripheral.clone());
                println!("Could not communicate with sensor");
                false
            }
            Err(err) => {
                println!("Could not poll sensor data! {:?}", err);
                true
            }
        }
    }
}

type StatePtr<S> = Arc<RwLock<Box<S>>>;

fn build_http<D: Database<SensorHandle=i32> + Send + Clone + 'static, S: SensorsState + Sync + Send + 'static>(db: D, state: StatePtr<S>) -> actix_web::dev::Server {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(move || {
                let sensors_scope: Scope = web::scope("/api/sensors")
                    .service(web::resource("/list")
                        .route(web::get().to(api::sensors_list::<D>))
                    )
                    .service(web::resource("/{id}/readings")
                        .route(web::get().to(api::sensor_readings::<D>))
                    )
                    .service(web::resource("/{id}")
                        .route(web::get().to(api::sensor_status::<D, S>))
                    )
                    .service(web::resource("/{id}/readings/after/{timestamp}")
                        .route(web::get()
                            .to(api::sensor_readings_after_time::<D>)
                            .to(api::sensor_readings_after_time_utc::<D>)
                        )
                    )
                    .service(web::resource("/{id}/latest/{kind}")
                        .route(web::get().to(api::sensor_latest_reading::<D>))
                    )
                    .default_service(web::route().to(|| HttpResponse::NotFound()));

                let frontend_scope: Scope = web::scope("/")
                    .service(actix_files::Files::new("", "./app/")
                        .use_etag(true)
                        .index_file("index.html")
                        .default_handler(web::route().to(api::not_found)));

                App::new()
                    .service(api::status)
                    .service(sensors_scope)
                    .service(frontend_scope)
                    .wrap(Logger::default())
                    .data(db.clone())
                    .data(state.clone())
            })
            .bind("0.0.0.0:80")?
            .shutdown_timeout(60)
            .run();

        let _ = tx.send(srv);
        sys.run()
    });

    rx.recv().unwrap()
}

async fn wait_for_keyboard_interrupt(mut wait_action: Box<dyn FnMut()>) -> () {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&stop)).expect("Failed to register SIGINT hook");

    while !stop.load(Ordering::Relaxed) {
        wait_action();
    }
}

#[actix_web::main]
async fn main() -> Result<(), String> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database = SqliteDatabase::new("./database.sqlite3");

    let app_state = Arc::new(RwLock::new(Box::new(AppState::new())));

    let srv = build_http(database.clone(), app_state.clone());
    let manager = Manager::new().unwrap();
    let central = get_central(&manager);

    println!("Starting BLE scan...");
    central.start_scan().expect("Unable to start scan");
    println!("Scan started");

    thread::sleep(Duration::from_secs(2));

    println!("Getting the event receiver");
    let events = central.event_receiver().unwrap();
    let mut master = BleMaster::new(database, app_state);

    let mut prev_poll = Instant::now();
    let mut prev_inspect = Instant::now();

    let inspect_interval_secs = 1;
    let poll_interval_secs = 5 * 60;

    println!("Running the app...");
    wait_for_keyboard_interrupt(Box::new(move || {
        match events.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => match event {
                CentralEvent::DeviceDiscovered(addr) => {
                    println!("{} discovered", addr);
                    match central.peripheral(addr) {
                        Some(peripheral) => { 
                            prev_inspect = Instant::now();
                            master.on_discovered(peripheral);
                        },
                        None => println!("* Failed to get the peripheral")
                    }
                },
                CentralEvent::DeviceDisconnected(addr) => {
                    master.on_disconnect(addr);
                    println!("Rescan after disconnect");
                    central.start_scan().expect("Failed to start scan");
                },
                _ => {}
            },
            _ => {}
        };

        let now = Instant::now();
        let inspect_dt = now.duration_since(prev_inspect);
        if inspect_dt.as_secs() >= inspect_interval_secs {
            master.pop_and_inspect();
            prev_inspect = Instant::now();
        }
        let poll_dt = now.duration_since(prev_poll);
        if poll_dt.as_secs() >= poll_interval_secs {
            prev_poll = Instant::now();
            let mut sensors = master.sensors.lock().expect("Poisoned mutex");
            sensors.retain(|sensor| master.try_poll_sensor(sensor));
        }
    })).await;

    println!("Stopping the server...");
    srv.clone().stop(true).await;
    Ok(())
}
