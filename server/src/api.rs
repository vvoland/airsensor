use actix_web::{get, web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::{SensorStatus, SensorsState, StatePtr, database::{Database, DatabaseError}};

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

#[get("/status")]
pub async fn status() -> impl Responder {
    HttpResponse::Ok().body("Server is up and running!")
}

pub async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("<html><head><title>Not found</title><body><h1>404</h1></html>")
}

//#[get("/list")]
pub async fn sensors_list<D: Database>(db: web::Data<D>)  -> HttpResponse {
    map_db_call_to_http_response(db.get_sensors())
}

//#[get("/{id}/latest/{type}")]
pub async fn sensor_latest_reading<D: Database>(
    request: web::Path<(D::SensorHandle, String)>,
    db: web::Data<D>) 
-> HttpResponse {
    let handle = request.0.0;
    let kind = request.0.1;

    map_db_call_to_http_response(db.get_latest_reading(&handle, kind))
}

//#[get("/{id}")]
pub async fn sensor_status<D: Database, S: SensorsState>(request: web::Path<D::SensorHandle>, db: web::Data<D>, state: web::Data<StatePtr<S>>) -> HttpResponse {
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
pub async fn sensor_readings<D: Database>(request: web::Path<D::SensorHandle>, db: web::Data<D>) -> HttpResponse {
    let handle = request.0;
    map_db_call_to_http_response(db.get_readings(&handle))
}

//#[get("/{id}/readings/after/{timestamp}")]
pub async fn sensor_readings_after_time<D: Database>(
    request: web::Path<(D::SensorHandle, chrono::NaiveDateTime)>,
    db: web::Data<D>)
    -> HttpResponse {

    let handle = request.0.0;
    let date = request.0.1;
    map_db_call_to_http_response(db.get_readings_after(&handle, date))
}

pub async fn sensor_readings_after_time_utc<D: Database>(
    request: web::Path<(D::SensorHandle, chrono::DateTime<Utc>)>,
    db: web::Data<D>)
    -> HttpResponse {

    let handle = request.0.0;
    let date = request.0.1;
    let time = date
        // Subtract 1ms to ensure greather-than
        .checked_add_signed(chrono::Duration::milliseconds(1))
        .unwrap()
        .naive_utc();

    map_db_call_to_http_response(db.get_readings_after(&handle, time))
}
