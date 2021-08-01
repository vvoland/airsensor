use std::time::Instant;

use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::r2d2;
use diesel::prelude::*;

use log::info;

use crate::{database::{Database, DatabaseError}, schema, sensor::SensorReading, sensor::{Sensor, SensorFamily, TimestampedSensorReading}};

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
type DbConnection = r2d2::PooledConnection<r2d2::ConnectionManager<SqliteConnection>>;

diesel_migrations::embed_migrations!("./migrations/");

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

        let utc = DateTime::<Utc>::from_utc(dto.timestamp, Utc);
        TimestampedSensorReading { timestamp: utc, reading }
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
            diesel::result::Error::DatabaseError(_, _) => {
                let lowercase_err = err.to_string().to_lowercase();
                if lowercase_err.contains("database is locked") {
                    DatabaseError::Busy
                } else if lowercase_err.contains("is not unique") {
                    DatabaseError::Conflict
                } else {
                    DatabaseError::Other(format!("{:?}", err))
                }
            },
            err => DatabaseError::Other(err.to_string())
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
        reading: &SensorReading)
    -> Result<(), DatabaseError> {

        let (kind, value) = match reading {
            SensorReading::Temperature(temperature) => ("T", *temperature as i32),
            SensorReading::Humidity(humidity) => ("H", *humidity as i32),
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

        let before_db = Instant::now();

        let result = self.connection_or_busy()
            .and_then(|conn| {
                schema::Readings::table
                    .filter(schema::Readings::sensor.eq(handle))
                    .filter(schema::Readings::timestamp.gt(timestamp))
                    .load::<schema::ReadingDTO>(&conn)
                    .map_err(Self::sql_error_to_db_error)
            });

        let diff = Instant::now().duration_since(before_db);

        let before_map = Instant::now();
        let mapped = result
            .map(Self::map_readings);
        let diff2 = Instant::now().duration_since(before_map);

        info!("Getting readings took {}ms, mapping took {}ms", diff.as_millis(), diff2.as_millis());

        return mapped;
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
