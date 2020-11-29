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

#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body("Server is up and running!")
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("<html><head><title>Not found</title><body><h1>404</h1></html>")
}

#[get("/list")]
async fn sensors_list(pool: web::Data<DbPool>)  -> HttpResponse {
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

#[get("/{id}/readings")]
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

struct BleMaster<P: Peripheral> {
    to_inspect: Mutex<Vec<P>>,
    sensors: Mutex<Vec<AlphaSensor<P>>>

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

}

fn build_http(db_pool: DbPool) -> actix_web::dev::Server {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(move || {
                let sensors_scope: Scope = web::scope("/api/sensors")
                    .service(sensors_list)
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
                    .data(db_pool.clone())
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

    let db_manager = r2d2::ConnectionManager::<SqliteConnection>::new("database.sqlite3");
    let db_pool = r2d2::Pool::builder()
        .build(db_manager)
        .expect("Could not create database pool");

    println!("Database connected");

    {
        let migration_connection = db_pool.get().expect("Failed to get database connection for migrations");
        if let Err(err) = embedded_migrations::run_with_output(&migration_connection, &mut std::io::stdout()) {
            println!("Migration failed: {}", err);
            return Err(String::from("Migration failed"));
        }
    }

    println!("Database initialized");

    let srv = build_http(db_pool.clone());
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
            sensors.retain(|sensor| {
                match sensor.poll() {
                    Ok(reading) => {
                        println!("Temperature: {}C, Humidity: {}%", reading.temperature, reading.humidity);

                        let table = schema::Sensors::table;
                        let properties = sensor.peripheral.properties();
                        let address_str = properties.address.to_string();

                        let find_sensor = |addr| {
                            table
                                .filter(schema::Sensors::dsl::address.eq(addr))
                                .first::<schema::SensorDTO>(&db_pool.get().expect("Could not obtain database connection"))
                        };

                        let first_sensor_query = find_sensor(&address_str);

                        let sensor_id = match first_sensor_query {
                            Ok(sensor_dto) => sensor_dto.id,
                            Err(_) => {
                                diesel::insert_into(table)
                                    .values(schema::AddSensorDTO {
                                        name: properties.local_name,
                                        address: properties.address.to_string()
                                    })
                                    .execute(&db_pool.get().expect("Could not obtain database connection"))
                                    .expect("Failed to insert sensor");
                                find_sensor(&address_str)
                                    .unwrap()
                                    .id
                            }
                        };


                        let now = Utc::now().naive_utc();

                        diesel::insert_into(schema::Readings::table)
                            .values(schema::AddReadingDTO {
                                sensor: sensor_id,
                                timestamp: now,
                                kind: "T",
                                value: reading.temperature as i32
                            })
                            .execute(&db_pool.get().expect("Could not obtain database connection"))
                            .expect("Failed to insert temperature reading");

                        diesel::insert_into(schema::Readings::table)
                            .values(schema::AddReadingDTO {
                                sensor: sensor_id,
                                timestamp: now,
                                kind: "H",
                                value: reading.humidity as i32
                            })
                            .execute(&db_pool.get().expect("Could not obtain database connection"))
                            .expect("Failed to insert humidity reading");

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
