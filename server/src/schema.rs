use serde::Serialize;
use chrono::NaiveDateTime;

table! {
    #[allow(non_snake_case)]
    Sensors(id) {
        id -> Integer,
        address -> Text,
        name -> Nullable<Text>,
    }
}

table! {
    #[allow(non_snake_case)]
    Readings(id) {
        id -> Integer,
        sensor -> Integer,
        timestamp -> Timestamp,
        kind -> Char,
        value -> Integer,
    }
}

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

