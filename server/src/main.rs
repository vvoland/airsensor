extern crate btleplug;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;

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

use std::time::SystemTime;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Scope, rt::System};
use std::vec::Vec;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{Ordering, AtomicBool};
use std::thread;
use std::time::Duration;
use chrono::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use diesel::r2d2;

mod alpha_sensor;
use alpha_sensor::*;

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

#[derive(Debug, Clone)]
pub enum DatabaseError {
    Busy,
    NotFound,
    Conflict,
    Other(String)
}

pub trait Database {
    fn create_sensor_if_not_exists(&self, sensor: &Sensor) -> Result<bool, DatabaseError>;
    fn add_reading(&self,
        sensor: &Sensor,
        timestamp: NaiveDateTime,
        reading: SensorReading)
    -> Result<(), DatabaseError>;
    fn get_readings(&self, sensor: &Sensor) -> Result<Vec<SensorReading>, DatabaseError>;
}

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: DbPool
}

impl SqliteDatabase {
    pub fn new(path: &'static str) -> Self {
        let db_manager = r2d2::ConnectionManager::<SqliteConnection>::new(path);
        let db_pool = r2d2::Pool::builder()
            .build(db_manager)
            .expect("Could not create database pool");

        println!("Database connected");
        {
            let migration_connection = db_pool.get().expect("Failed to get database connection for migrations");
            embedded_migrations::run_with_output(&migration_connection, &mut std::io::stdout())
                .expect("Migration failed");
        }

        SqliteDatabase {
            pool: db_pool
        }

    }

    fn find_sensor(&self, addr: BDAddr) -> Result<schema::SensorDTO, diesel::result::Error> {
        schema::Sensors::table
            .filter(schema::Sensors::dsl::address.eq(addr.to_string()))
            .first::<schema::SensorDTO>(&self.pool.get().expect("Could not obtain database connection"))
    }

    fn find_sensor_map_err(&self, addr: BDAddr) -> Result<schema::SensorDTO, DatabaseError> {
        self.find_sensor(addr)
            .map_err(|err| match err {
                diesel::result::Error::NotFound => DatabaseError::NotFound,
                _ => DatabaseError::Other(err.to_string())
            })
    }

    fn to_reading(&self, reading: &schema::ReadingDTO) -> SensorReading {
        match reading.kind.as_ref() {
            "T" => SensorReading::Temperature(reading.value),
            "H" => SensorReading::Humidity(reading.value as u8),
            _ => SensorReading::Unknown
        }
    }
}

impl Database for SqliteDatabase {
    fn create_sensor_if_not_exists(&self, sensor: &Sensor) -> Result<bool, DatabaseError> {
        let table = schema::Sensors::table;
        match self.find_sensor(sensor.address) {
            Ok(_) => Ok(false),
            Err(diesel::result::Error::NotFound) => {
                match self.pool.get() {
                    Ok(conn) =>
                        diesel::insert_into(table)
                            .values(schema::AddSensorDTO {
                                name: sensor.name.clone(),
                                address: sensor.address.to_string()
                            })
                            .execute(&conn)
                            .map(|_| true)
                            .map_err(|err| DatabaseError::Other(err.to_string())),
                    Err(_) => Err(DatabaseError::Busy)
                }
            }
            Err(err) => Err(DatabaseError::Other(err.to_string()))
        }
    }

    fn add_reading(&self,
        sensor: &Sensor,
        timestamp: NaiveDateTime,
        reading: SensorReading)
    -> Result<(), DatabaseError> {

        let (kind, value) = match reading {
            SensorReading::Temperature(temperature) => ("T", temperature as i32),
            SensorReading::Humidity(humidity) => ("H", humidity as i32),
            SensorReading::Unknown => panic!("An attempt to insert unknown sensor reading")
        };

        self.find_sensor_map_err(sensor.address)
            .and_then(|dto| {
                diesel::insert_into(schema::Readings::table)
                    .values(schema::AddReadingDTO {
                        sensor: dto.id,
                        timestamp, kind, value
                    })
                    .execute(&self.pool.get().expect("Could not obtain database connection"))
                    .map(|inserts| assert!(inserts == 1))
                .map_err(|err| DatabaseError::Other(err.to_string()))
            })
    }

    fn get_readings(&self, sensor: &Sensor) -> Result<Vec<SensorReading>, DatabaseError> {
        self.find_sensor_map_err(sensor.address)
            .and_then(|dto| {
                match self.pool.get() {
                    Ok(conn) => {
                        schema::Readings::table
                            .filter(schema::Readings::sensor.eq(dto.id))
                            .load::<schema::ReadingDTO>(&conn)
                            .map_err(|err| DatabaseError::Other(err.to_string()))
                    },
                    Err(_) => Err(DatabaseError::Busy)
                }
            })
            .map(|readings| {
                readings
                    .iter()
                    .map(|reading| self.to_reading(&reading))
                    .collect()
            })
    }
}


#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body("Server is up and running!")
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("<html><head><title>Not found</title><body><h1>404</h1></html>")
}

/*
#[get("/list")]
async fn sensors_list<D: Database>(pool: web::Data<D>)  -> HttpResponse {
    pool.get()
        .map_or(HttpResponse::ServiceUnavailable().body("Database connection failed"),
            |conn| {
                schema::Sensors::table
                    .load::<schema::SensorDTO>(&conn)
                    .map_or(HttpResponse::InternalServerError().body("Sensors query failed"), |sensors| {
                        HttpResponse::Ok().json(&sensors)
                    })
            }
        )
}
*/

#[get("/{id}/latest/{type}")]
async fn sensor_current_reading(request: web::Path<(i32, String)>, pool: web::Data<DbPool>)  -> HttpResponse {
    pool.get()
        .map_or(HttpResponse::ServiceUnavailable().body("Database connection failed"),
            |conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(request.0.0))
                    .filter(schema::Readings::kind.eq(request.0.1))
                    .order_by(schema::Readings::id.desc())
                    .first::<schema::ReadingDTO>(&conn)
                    .map_or(HttpResponse::InternalServerError().body("Latest query failed"), |latest| {
                        HttpResponse::Ok().json(latest)
                    })
            }
        )
}

#[get("/{name}/readings")]
async fn sensor_readings(request: web::Path<i32>, pool: web::Data<DbPool>)  -> HttpResponse {
    pool.get()
        .map_or(HttpResponse::ServiceUnavailable().body("Database connection failed"),
            |conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(request.0))
                    .load::<schema::ReadingDTO>(&conn)
                    .map_or(HttpResponse::InternalServerError().body("Readings query failed"), |latest| {
                        HttpResponse::Ok().json(latest)
                    })
            }
        )
}

#[get("/{id}/readings/after/{timestamp}")]
async fn sensor_readings_after_time(request: web::Path<(i32, chrono::NaiveDateTime)>, pool: web::Data<DbPool>)  -> HttpResponse {
    pool.get()
        .map_or(HttpResponse::ServiceUnavailable().body("Database connection failed"),
            |conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(request.0.0))
                    .filter(schema::Readings::timestamp.gt(request.0.1))
                    .load::<schema::ReadingDTO>(&conn)
                    .map_or(HttpResponse::InternalServerError().body("Readings query failed"), |latest| {
                        HttpResponse::Ok().json(latest)
                    })
            }
        )
}

async fn wait_for_keyboard_interrupt(mut wait_action: Box<dyn FnMut()>) -> () {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&stop)).expect("Failed to register SIGINT hook");

    while !stop.load(Ordering::Relaxed) {
        wait_action();
    }
}

pub enum SensorFamily {
    Alpha
}

pub enum SensorReading {
    Temperature(i32),
    Humidity(u8),
    Unknown
}

pub struct Sensor {
    pub family: SensorFamily,
    pub address: BDAddr,
    pub name: Option<String>
}

struct BleMaster<P: Peripheral, D: Database> {
    to_inspect: Mutex<Vec<P>>,
    sensors: Mutex<Vec<AlphaSensor<P>>>,
    db: D
}

impl<P: Peripheral, D: Database> BleMaster<P, D> {

    pub fn new(db: D) -> Self {
        BleMaster::<P, D> {
            db,
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
            println!("Ignoring {}", peripheral.address());
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
        println!("Inspecting {}...", peripheral.address());
        if let Some(characteristic) = AlphaSensor::inspect(&peripheral) {
            println!("Found characteristics in {}", peripheral.address());
            let mut sensors = self.sensors.lock().expect("Poisoned mutex");
            if let Some(sensor) = AlphaSensor::try_new(peripheral.clone(), characteristic) {
                sensors.push(sensor);
            } else {
                let _ = peripheral.disconnect();
            }
        }
    }

    pub fn try_poll_sensor(&self, sensor: &AlphaSensor<P>) -> bool {
        match sensor.poll() {
            Ok(reading) => {
                let now = Utc::now().naive_utc();
                let properties = sensor.peripheral.properties();
                let name_str = match properties.local_name.clone() {
                    Some(s) => s,
                    None => "???".to_string()
                };
                println!("[{}] Temperature: {}C, Humidity: {}%", name_str, reading.temperature, reading.humidity);

                let sensor_data = Sensor {
                    family: SensorFamily::Alpha,
                    address: properties.address,
                    name: properties.local_name
                };

                self.db.create_sensor_if_not_exists(&sensor_data)
                    .expect("Could not ensure that the sensor exists in the database");
                self.db.add_reading(&sensor_data, now, SensorReading::Temperature(reading.temperature as i32))
                    .expect("Could not insert temperature reading");
                self.db.add_reading(&sensor_data, now, SensorReading::Humidity(reading.humidity))
                    .expect("Could not insert temperature reading");

                true
            }
            Err(AlphaSensorPollError::SendFailed) => {
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

fn build_http<D: Database + Send + Clone + 'static>(db: D) -> actix_web::dev::Server {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(move || {
                let sensors_scope: Scope = web::scope("/api/sensors")
                    //.service(sensors_list)
                    .service(sensor_readings)
                    .service(sensor_readings_after_time)
                    .service(sensor_current_reading)
                    .default_service(web::route().to(|| HttpResponse::NotFound()));

                let frontend_scope: Scope = web::scope("/")
                    .service(actix_files::Files::new("", "./app/")
                        .index_file("index.html")
                        .default_handler(web::route().to(not_found)));

                App::new()
                    .service(status)
                    .service(sensors_scope)
                    .service(frontend_scope)
                    .data(db.clone())
            })
            .bind("0.0.0.0:80")?
            .shutdown_timeout(60)
            .run();

        let _ = tx.send(srv);
        sys.run()
    });

    rx.recv().unwrap()
}

diesel_migrations::embed_migrations!("./migrations/");

mod schema {
    table! {
        Sensors(id) {
            id -> Integer,
            address -> Text,
            name -> Nullable<Text>,
        }
    }

    table! {
        Readings(id) {
            id -> Integer,
            sensor -> Integer,
            timestamp -> Timestamp,
            kind -> Char,
            value -> Integer,
        }
    }

    use super::*;

    #[derive(Serialize, Debug, Clone, Queryable)]
    pub struct ReadingDTO {
       pub id: i32,
       pub sensor: i32,
       pub timestamp: NaiveDateTime,
       pub kind: String,
       pub value: i32
    }

    #[derive(Debug, Clone, Insertable)]
    #[table_name="Readings"]
    pub struct AddReadingDTO {
       pub sensor: i32,
       pub timestamp: NaiveDateTime,
       pub kind: &'static str,
       pub value: i32
    }

    #[derive(Serialize, Debug, Clone, Queryable)]
    pub struct SensorDTO {
       pub id: i32,
       pub address: String,
       pub name: Option<String>
    }

    #[derive(Debug, Clone, Insertable)]
    #[table_name="Sensors"]
    pub struct AddSensorDTO {
       pub name: Option<String>,
       pub address: String
    }
}


#[actix_web::main]
async fn main() -> Result<(), String> {

    let database = SqliteDatabase::new("./database.sqlite3");

    let srv = build_http(database.clone());
    let manager = Manager::new().unwrap();
    let central = get_central(&manager);

    println!("Starting BLE scan...");
    central.start_scan().expect("Unable to start scan");
    println!("Scan started");

    thread::sleep(Duration::from_secs(2));

    println!("Getting the event receiver");
    let events = central.event_receiver().unwrap();
    let mut master = BleMaster::new(database);

    let mut prev = SystemTime::now();

    println!("Running the app...");
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
                CentralEvent::DeviceDisconnected(_) => {
                    println!("Rescan after disconnect");
                    central.start_scan().expect("Failed to start scan");
                },
                _ => {}
            },
            _ => {}
        };
        master.pop_and_inspect();
        let dt = SystemTime::now().duration_since(prev).unwrap();
        if dt.as_secs() >= 30 {
            prev = SystemTime::now();
            let mut sensors = master.sensors.lock().expect("Poisoned mutex");
            sensors.retain(|sensor| master.try_poll_sensor(sensor));
        }
    })).await;

    println!("Stopping the server...");
    srv.clone().stop(true).await;
    Ok(())
}
