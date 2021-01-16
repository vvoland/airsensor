import { Chart } from "chart.js";
import { ReadingKind } from "./reading";

export function create_chart(canvas: HTMLCanvasElement, kind: ReadingKind) {
    let margin = {top: 0, right: 0, bottom: 0, left: 0};

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
                data: [],
                pointRadius: 1
            };
            break;
        case ReadingKind.Humidity:
            dataset = {
                label: "Humidity",
                backgroundColor: `${humidityColor}88`,
                borderColor: humidityColor,
                fill: fillmode,
                data: [],
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
                            suggestedMin: 0,
                            suggestedMax: 100
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