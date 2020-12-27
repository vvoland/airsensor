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
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::sync::atomic::{Ordering, AtomicBool};
use std::thread;
use std::time::Duration;
use chrono::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use diesel::prelude::*;
use diesel::r2d2;

mod alpha_sensor;
use alpha_sensor::*;

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
type DbConnection = r2d2::PooledConnection<r2d2::ConnectionManager<SqliteConnection>>;

#[derive(Debug, Clone)]
pub enum DatabaseError {
    Busy,
    NotFound,
    Conflict,
    Other(String)
}

pub trait Database {
    type SensorHandle;
    fn get_sensor_handle(&self, sensor: &Sensor) -> Result<Self::SensorHandle, DatabaseError>;
    fn get_sensor_by_addr(&self, addr: String) -> Result<Self::SensorHandle, DatabaseError>;
    fn get_sensor_by_handle(&self, handle: &Self::SensorHandle) -> Result<Sensor, DatabaseError>;
    fn create_sensor_if_not_exists(&self, sensor: &Sensor) -> Result<bool, DatabaseError>;
    fn get_sensors(&self) -> Result<Vec<Sensor>, DatabaseError>;
    fn add_reading(&self,
        sensor: &Self::SensorHandle,
        timestamp: NaiveDateTime,
        reading: SensorReading)
        -> Result<(), DatabaseError>;
    fn get_readings(&self, handle: &Self::SensorHandle)
        -> Result<Vec<TimestampedSensorReading>, DatabaseError>;
    fn get_readings_after(&self, handle: &Self::SensorHandle, timestamp: NaiveDateTime)
        -> Result<Vec<TimestampedSensorReading>, DatabaseError>;
    fn get_latest_reading(&self, handle: &Self::SensorHandle, kind: String)
        -> Result<TimestampedSensorReading, DatabaseError>;
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

    fn find_sensor(&self, addr: &String) -> Result<schema::SensorDTO, diesel::result::Error> {
        schema::Sensors::table
            .filter(schema::Sensors::dsl::address.eq(addr))
            .first::<schema::SensorDTO>(&self.pool.get().expect("Could not obtain database connection"))
    }

    fn find_sensor_map_err(&self, addr: &String) -> Result<schema::SensorDTO, DatabaseError> {
        self.find_sensor(addr)
            .map_err(|err| match err {
                diesel::result::Error::NotFound => DatabaseError::NotFound,
                _ => DatabaseError::Other(err.to_string())
            })
    }

    fn connection_or_busy(&self) -> Result<DbConnection, DatabaseError> {
        self.pool.get()
            .map_err(|_| DatabaseError::Busy)
    }

    fn to_reading(dto: &schema::ReadingDTO) -> TimestampedSensorReading {
        let reading = match dto.kind.as_ref() {
            "T" => SensorReading::Temperature(dto.value),
            "H" => SensorReading::Humidity(dto.value as u8),
            _ => SensorReading::Unknown
        };

        TimestampedSensorReading { timestamp: dto.timestamp, reading }
    }

    fn to_sensor(sensor: &schema::SensorDTO) -> Sensor {
        Sensor {
            family: SensorFamily::Alpha, // No other sensors supported for now!
            address: sensor.address.clone(),
            name: sensor.name.clone()
        }
    }

    fn map_readings(readings: Vec<schema::ReadingDTO>) -> Vec<TimestampedSensorReading> {
        readings
            .iter()
            .map(|reading| Self::to_reading(&reading))
            .collect()
    }

    fn map_sensors(readings: Vec<schema::SensorDTO>) -> Vec<Sensor> {
        readings
            .iter()
            .map(|sensor| Self::to_sensor(&sensor))
            .collect()
    }

    fn sql_error_to_db_error(err: diesel::result::Error) -> DatabaseError {
        match err {
            diesel::result::Error::AlreadyInTransaction => DatabaseError::Busy,
            _ => DatabaseError::Other(err.to_string())
        }
    }
}

impl Database for SqliteDatabase {
    type SensorHandle = i32;
    fn create_sensor_if_not_exists(&self, sensor: &Sensor) -> Result<bool, DatabaseError> {
        let table = schema::Sensors::table;
        match self.find_sensor(&sensor.address) {
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
                            .map_err(Self::sql_error_to_db_error),
                    Err(_) => Err(DatabaseError::Busy)
                }
            }
            Err(err) => Err(DatabaseError::Other(err.to_string()))
        }
    }

    fn add_reading(&self,
        handle: &Self::SensorHandle,
        timestamp: NaiveDateTime,
        reading: SensorReading)
    -> Result<(), DatabaseError> {

        let (kind, value) = match reading {
            SensorReading::Temperature(temperature) => ("T", temperature as i32),
            SensorReading::Humidity(humidity) => ("H", humidity as i32),
            SensorReading::Unknown => panic!("An attempt to insert unknown sensor reading")
        };

        self.connection_or_busy()
            .and_then(|conn| {
                diesel::insert_into(schema::Readings::table)
                    .values(schema::AddReadingDTO {
                        sensor: *handle,
                        timestamp, kind, value
                    })
                    .execute(&conn)
                    .map_err(Self::sql_error_to_db_error)
                    .map(|inserts| assert!(inserts == 1))
            })
    }

    fn get_readings(&self, handle: &Self::SensorHandle) -> Result<Vec<TimestampedSensorReading>, DatabaseError> {
        self.connection_or_busy()
            .and_then(|conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(handle))
                    .load::<schema::ReadingDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            })
            .map(Self::map_readings)
    }

    fn get_readings_after(&self, handle: &Self::SensorHandle, timestamp: NaiveDateTime)
        -> Result<Vec<TimestampedSensorReading>, DatabaseError> {

        self.connection_or_busy()
            .and_then(|conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(handle))
                    .filter(schema::Readings::timestamp.gt(timestamp))
                    .load::<schema::ReadingDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            })
            .map(Self::map_readings)
    }

    fn get_sensor_by_addr(&self, addr: String) -> Result<Self::SensorHandle, DatabaseError> {
        self.find_sensor_map_err(&addr)
            .map(|dto| dto.id)
    }

    fn get_sensor_by_handle(&self, handle: &Self::SensorHandle) -> Result<Sensor, DatabaseError> {
        schema::Sensors::table
            .filter(schema::Sensors::dsl::id.eq(handle))
            .first::<schema::SensorDTO>(&self.pool.get().expect("Could not obtain database connection"))
            .map(|sensor| Self::to_sensor(&sensor))
            .map_err(Self::sql_error_to_db_error)
    }


    fn get_sensor_handle(&self, sensor: &Sensor) -> Result<Self::SensorHandle, DatabaseError> {
        self.connection_or_busy()
            .and_then(|conn| {
                schema::Sensors::table
                    .filter(schema::Sensors::dsl::address.eq(sensor.address.to_string()))
                    .filter(schema::Sensors::dsl::name.eq(&sensor.name))
                    .first::<schema::SensorDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            })
            .map(|dto| dto.id)
    }

    fn get_sensors(&self) -> Result<Vec<Sensor>, DatabaseError> {
        self.connection_or_busy()
            .and_then(|conn| {
                schema::Sensors::table
                    .load::<schema::SensorDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            })
            .map(Self::map_sensors)
    }

    fn get_latest_reading(&self, handle: &Self::SensorHandle, kind: String)
        -> Result<TimestampedSensorReading, DatabaseError> {

        self.connection_or_busy()
            .and_then(|conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(handle))
                    .filter(schema::Readings::kind.eq(kind))
                    .order_by(schema::Readings::id.desc())
                    .first::<schema::ReadingDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            })
            .map(|reading| Self::to_reading(&reading))
    }
}


#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body("Server is up and running!")
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("<html><head><title>Not found</title><body><h1>404</h1></html>")
}

//#[get("/list")]
async fn sensors_list<D: Database>(db: web::Data<D>)  -> HttpResponse {
    map_db_call_to_http_response(db.get_sensors())
}

//#[get("/{id}/latest/{type}")]
async fn sensor_latest_reading<D: Database>(
    request: web::Path<(D::SensorHandle, String)>,
    db: web::Data<D>) 
-> HttpResponse {
    let handle = request.0.0;
    let kind = request.0.1;

    map_db_call_to_http_response(db.get_latest_reading(&handle, kind))
}

fn map_database_error_to_http(err: DatabaseError) -> HttpResponse {
    match err {
        DatabaseError::Busy => HttpResponse::ServiceUnavailable().body("Database connection failed"),
        DatabaseError::NotFound => HttpResponse::NotFound().body("{}"),
        DatabaseError::Other(msg) => HttpResponse::InternalServerError().body(msg),
        _ => HttpResponse::InternalServerError().body("Unknown error")
    }
}

fn map_db_call_to_http_response<R: serde::Serialize>(db_result: Result<R, DatabaseError>) -> HttpResponse {
    match db_result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => map_database_error_to_http(err)
    }
}

//#[get("/{id}")]
async fn sensor_status<D: Database, S: SensorsState>(request: web::Path<D::SensorHandle>, db: web::Data<D>, state: web::Data<StatePtr<S>>) -> HttpResponse {
    let handle = request.0;

    #[derive(Serialize, Deserialize)]
    pub struct StatusResponse {
        status: SensorStatus,
    }

    match db.get_sensor_by_handle(&handle) {
        Ok(sensor) => {
            let state = state.read().unwrap();
            HttpResponse::Ok().json(StatusResponse { status: state.get_status(&sensor) })
        },
        Err(err) => map_database_error_to_http(err)
    }
}

//#[get("/{id}/readings")]
async fn sensor_readings<D: Database>(request: web::Path<D::SensorHandle>, db: web::Data<D>) -> HttpResponse {
    let handle = request.0;
    map_db_call_to_http_response(db.get_readings(&handle))
}

//#[get("/{id}/readings/after/{timestamp}")]
async fn sensor_readings_after_time<D: Database>(
    request: web::Path<(D::SensorHandle, chrono::NaiveDateTime)>,
    db: web::Data<D>)
    -> HttpResponse {

    let handle = request.0.0;
    let date = request.0.1;
    map_db_call_to_http_response(db.get_readings_after(&handle, date))
}

async fn wait_for_keyboard_interrupt(mut wait_action: Box<dyn FnMut()>) -> () {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&stop)).expect("Failed to register SIGINT hook");

    while !stop.load(Ordering::Relaxed) {
        wait_action();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SensorFamily {
    Alpha
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum SensorReading {
    Temperature(i32),
    Humidity(u8),
    Unknown
}

pub struct TimestampedSensorReading {
    pub timestamp: NaiveDateTime,
    pub reading: SensorReading
}

impl serde::Serialize for TimestampedSensorReading {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("timestamped_reading", 3)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        match self.reading {
            SensorReading::Temperature(temperature) => {
                state.serialize_field("kind", "T")?;
                state.serialize_field("value", &temperature)?;
            },
            SensorReading::Humidity(humidity) => {
                state.serialize_field("kind", "H")?;
                state.serialize_field("value", &humidity)?;
            },
            SensorReading::Unknown => {
                state.serialize_field("kind", "?")?;
                state.serialize_field("value", "null")?;
            }
        }
        state.end()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sensor {
    pub family: SensorFamily,
    pub address: String,
    pub name: Option<String>
}

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

#[derive(Serialize, Deserialize)]
pub enum SensorStatus {
    Online,
    Offline
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
                    address: properties.address.to_string(),
                    name: properties.local_name
                };

                self.db.create_sensor_if_not_exists(&sensor_data)
                    .expect("Could not ensure that the sensor exists in the database");
                let handle = self.db.get_sensor_handle(&sensor_data).expect("Failed to get handle to just added sensor");
                self.db.add_reading(&handle, now, SensorReading::Temperature(reading.temperature as i32))
                    .expect("Could not insert temperature reading");
                self.db.add_reading(&handle, now, SensorReading::Humidity(reading.humidity))
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

type StatePtr<S> = Arc<RwLock<Box<S>>>;

fn build_http<D: Database<SensorHandle=i32> + Send + Clone + 'static, S: SensorsState + Sync + Send + 'static>(db: D, state: StatePtr<S>) -> actix_web::dev::Server {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(move || {
                let sensors_scope: Scope = web::scope("/api/sensors")
                    .service(web::resource("/list")
                        .route(web::get().to(sensors_list::<D>))
                    )
                    .service(web::resource("/{id}/readings")
                        .route(web::get().to(sensor_readings::<D>))
                    )
                    .service(web::resource("/{id}")
                        .route(web::get().to(sensor_status::<D, S>))
                    )
                    .service(web::resource("/{id}/readings/after/{timestamp}")
                        .route(web::get().to(sensor_readings_after_time::<D>))
                    )
                    .service(web::resource("/{id}/latest/{kind}")
                        .route(web::get().to(sensor_latest_reading::<D>))
                    )
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

    let mut prev_poll = SystemTime::now();
    let mut prev_inspect = SystemTime::now();

    println!("Running the app...");
    wait_for_keyboard_interrupt(Box::new(move || {
        match events.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => match event {
                CentralEvent::DeviceDiscovered(addr) => {
                    println!("{} discovered", addr);
                    match central.peripheral(addr) {
                        Some(peripheral) => { 
                            prev_inspect = SystemTime::now();
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

        let now = SystemTime::now();
        let inspect_dt = now.duration_since(prev_inspect).unwrap();
        if inspect_dt.as_secs() >= 1 {
            master.pop_and_inspect();
            prev_inspect = SystemTime::now();
        }
        let poll_dt = now.duration_since(prev_poll).unwrap();
        if poll_dt.as_secs() >= 30 {
            prev_poll = SystemTime::now();
            let mut sensors = master.sensors.lock().expect("Poisoned mutex");
            sensors.retain(|sensor| master.try_poll_sensor(sensor));
        }
    })).await;

    println!("Stopping the server...");
    srv.clone().stop(true).await;
    Ok(())
}
