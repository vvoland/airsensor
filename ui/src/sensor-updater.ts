import { CelsiusTemperature, PercentageHumidity, ReadingKind } from "./reading";
import { SensorCardView } from "./sensor-card";
import { fetch_latest, fetch_readings_after, fetch_status, TimestampedSensorReading } from "./sensors-api";

export enum ChartInterval {
    LastHour,
    LastHalfDay,
    LastDay,
    LastWeek
}

export class SensorUpdater {
    private view: SensorCardView;
    private id: number;
    private lastUpdate: Date;
    private allReadings: TimestampedSensorReading[];

    constructor(sensorView: SensorCardView, id: number) {
        this.view = sensorView;
        this.id = id;
        this.lastUpdate = new Date(0);
        this.allReadings = [];
    }

    private constructXY(kind: ReadingKind, interval: ChartInterval) {

        let last_reading = this.allReadings[this.allReadings.length - 1];

        let is_within = (hours: number) => (reading: TimestampedSensorReading) => {
            let ms_diff = last_reading.timestamp.getTime()
                            - reading.timestamp.getTime();
            let hours_diff = ms_diff
                            / 1000 // ms -> s
                            / 60   // s -> m
                            / 60;  // m -> h

            return hours_diff <= hours;
        };


        let interval_func = new Map([
            [ ChartInterval.LastHour, is_within(1) ],
            [ ChartInterval.LastHalfDay, is_within(12) ],
            [ ChartInterval.LastDay, is_within(24) ],
            [ ChartInterval.LastWeek, is_within(24*7) ]
        ]);
        let is_recent = interval_func.get(interval);

        let is_relevant = (reading: TimestampedSensorReading) => {
            switch (reading.kind) {
                case ReadingKind.Temperature:
                    return reading.value > 0;
                case ReadingKind.Humidity:
                    return reading.value > 20;
            }

            return true;
        };

        let reading_to_xy = (reading: TimestampedSensorReading) => {
            return {
                x: reading.timestamp,
                y: reading.value
            };
        };

        return this.allReadings
            .filter(reading => reading.kind == kind)
            .filter(is_relevant)
            .filter(is_recent)
            .map(reading_to_xy);
    }

    public async refresh() {
        return Promise.all([
            fetch_latest(this.id, ReadingKind.Temperature), fetch_latest(this.id, ReadingKind.Humidity),
            fetch_status(this.id), fetch_readings_after(this.id, this.lastUpdate)
        ])
            .then(([temperature, humidity, is_online, readings]) => {
                this.view.setOnline(is_online);
                this.view.setCurrentReading(ReadingKind.Temperature, new CelsiusTemperature(temperature));
                this.view.setCurrentReading(ReadingKind.Humidity, new PercentageHumidity(humidity));

                console.log(readings.length);
                if (readings.length > 0) {
                    this.allReadings.push(...readings);
                    this.lastUpdate = readings[readings.length - 1].timestamp;


                    let temperature_xy = this.constructXY(ReadingKind.Temperature, ChartInterval.LastDay);
                    let humidity_xy = this.constructXY(ReadingKind.Humidity, ChartInterval.LastDay);

                    this.view.setChartData(ReadingKind.Temperature, temperature_xy);
                    this.view.setChartData(ReadingKind.Humidity, humidity_xy);
                }

                return true;
            })
            .catch(reason => {
                console.warn("Failed to get an update! " + reason);
                return false;
            });
    }
}
