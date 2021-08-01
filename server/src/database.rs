use chrono::NaiveDateTime;
use crate::sensor::{Sensor, SensorReading, TimestampedSensorReading};

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
        reading: &SensorReading)
        -> Result<(), DatabaseError>;
    fn get_readings(&self, handle: &Self::SensorHandle)
        -> Result<Vec<TimestampedSensorReading>, DatabaseError>;
    fn get_readings_after(&self, handle: &Self::SensorHandle, timestamp: NaiveDateTime)
        -> Result<Vec<TimestampedSensorReading>, DatabaseError>;
    fn get_latest_reading(&self, handle: &Self::SensorHandle, kind: String)
        -> Result<TimestampedSensorReading, DatabaseError>;
}
