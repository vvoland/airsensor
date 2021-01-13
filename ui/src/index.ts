import { fetch_latest, fetch_status, fetch_readings, TimestampedSensorReading } from "./sensors-api";
import { ReadingKind, CelsiusTemperature, PercentageHumidity } from "./reading";
import { SensorCardRenderer } from "./sensor-card";
import { DummyLocalization } from "./localization";
import { Chart } from "chart.js";

var test_chart: Chart = undefined;

enum ChartInterval {
    LastHour,
    LastHalfDay,
    LastDay,
    LastWeek
}

function create_chart(canvas: HTMLCanvasElement,
    readings: TimestampedSensorReading[],
    kind: ReadingKind,
    interval: ChartInterval
) {
    let margin = {top: 0, right: 0, bottom: 0, left: 0};

    let last_reading = readings[readings.length - 1];

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

    let filtered_readings = readings
        .filter(reading => reading.kind == kind)
        .filter(is_relevant)
        .filter(is_recent);

    let reading_to_xy = (reading: TimestampedSensorReading) => {
        return {
            x: reading.timestamp,
            y: reading.value
        };
    };


    let values = filtered_readings.map(reading => reading.value);
    let min = values.reduce((a,b) => Math.min(a, b));
    let max = values.reduce((a,b) => Math.max(a, b));

    let fillmode = false;
    let temperatureColor = "#d6a2ad";
    let humidityColor = "#c3b59f";
    let ctx = canvas.getContext("2d");

    var dataset: Chart.ChartDataSets;
    switch (kind) {
        case ReadingKind.Temperature:
            dataset = {
                label: "Temperature",
                backgroundColor: `${temperatureColor}88`,
                borderColor: temperatureColor,
                fill: fillmode,
                data: filtered_readings
                    .map(reading_to_xy),
                pointRadius: 1
            };
            break;
        case ReadingKind.Humidity:
            dataset = {
                label: "Humidity",
                backgroundColor: `${humidityColor}88`,
                borderColor: humidityColor,
                fill: fillmode,
                data: filtered_readings
                    .map(reading_to_xy),
                pointRadius: 1
            };
            break;
    }


    return new Chart(ctx, {
        type: "line",
        data: { datasets: [ dataset ] },
        options: {
            layout: {
                padding: margin
            },
            showLines: true,
            responsive: true,
            legend: {
                display: false
            },
            scales: {
                xAxes: [
                    {
                        type: "time",
                        time: {
                            unit: "minute",
                            displayFormats: {
                                minute: "h:mm"
                            }
                        },
                        display: true,
                        distribution: "series",
                        scaleLabel: {
                            display: false
                        }
                    }
                ],
                yAxes: [
                    {
                        display: true,
                        ticks: {
                            suggestedMin: min - 5,
                            suggestedMax: max + 5
                        },
                        scaleLabel: {
                            display: false,
                            labelString: "value",
                        }
                    }
                ]
            }
        }
    });
}

async function refresh() {
    console.log("Refresh!");
    let temperature = await fetch_latest(1, ReadingKind.Temperature);
    let humidity = await fetch_latest(1, ReadingKind.Humidity);
    let online = await fetch_status(1);
    let readings = await fetch_readings(1);

    document.getElementById("sensors-container")!.innerHTML = new SensorCardRenderer("Salon", online)
        .with(new CelsiusTemperature(temperature))
        .with(new PercentageHumidity(humidity))
        .render(new DummyLocalization());

    test_chart = create_chart(
        (<HTMLCanvasElement>document.getElementById("sensor-chart-Salon-T")),
        readings,
        ReadingKind.Temperature,
        ChartInterval.LastHour
    );
    test_chart = create_chart(
        (<HTMLCanvasElement>document.getElementById("sensor-chart-Salon-H")),
        readings,
        ReadingKind.Humidity,
        ChartInterval.LastHour
    );
}


export function init() {
    refresh();
    setInterval(refresh, 5000);
}

init();
