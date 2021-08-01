use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, ser::SerializeStruct};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SensorFamily {
    Alpha
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum SensorReading {
    Temperature(i32),
    Humidity(u8),
    Unknown
}

#[derive(Serialize, Deserialize)]
pub enum SensorStatus {
    Online,
    Offline
}

pub struct TimestampedSensorReading {
    pub timestamp: DateTime<Utc>,
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
