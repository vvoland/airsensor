import { ReadingKind } from "./reading";

function fetch_path(path: string): Promise<any> {
    let base = window.location.host;
    return fetch(`http://${base}/api/sensors/${path}`)
        .then(response => response.json());
}

export async function fetch_latest(sensor_id: Number, kind: ReadingKind): Promise<number> {
    return await fetch_path(`${sensor_id}/latest/${kind}`)
        .catch(error => {
            console.log(`Failed to get latest ${kind} reading for sensor ${sensor_id}`);
            return {"value": 0};
        })
        .then(data => data.value);
}

export class TimestampedSensorReading {
    kind: ReadingKind;
    value: number;
    timestamp: Date;
}

export async function fetch_readings(sensor_id: Number): Promise<Array<TimestampedSensorReading>> {
    return await fetch_path(`${sensor_id}/readings`)
        .catch(error => {
            console.log(`Failed to get readings for sensor ${sensor_id}`);
            return [];
        })
        .then((readings: any[]) => {
            return readings.map(reading => {
                reading.timestamp = new Date(reading.timestamp);
                return reading;
            })
        });
}

export async function fetch_readings_after(sensor_id: Number, after: Date): Promise<Array<TimestampedSensorReading>> {
    let afterStr = after.toISOString();
    return await fetch_path(`${sensor_id}/readings/after/${afterStr}`)
        .catch(error => {
            console.log(`Failed to get readings for sensor ${sensor_id} after ${afterStr}`);
            return [];
        })
        .then((readings: any[]) => {
            return readings.map(reading => {
                reading.timestamp = new Date(reading.timestamp);
                return reading;
            })
        });
}

export async function fetch_status(sensor_id: Number): Promise<boolean> {
    return await fetch_path(`${sensor_id}`)
        .catch(error => {
            console.log(`Failed to get status of sensor ${sensor_id}`);
            return {"status": "Offline"};
        })
        .then(data => data.status)
        .then(status => {
            return status == "Online" ? true : false;
        });
}

