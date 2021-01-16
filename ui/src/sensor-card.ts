import { ReadingKind, SensorReading } from "./reading";
import { Localization } from "./localization";
import { create_chart } from "./sensor-chart";

export class SensorCardView {
    private card: HTMLDivElement;
    private title: HTMLSpanElement;
    private status: HTMLSpanElement;
    private subtitle: HTMLDivElement;

    private readingKinds: Array<ReadingKind>;
    private readings: Map<ReadingKind, HTMLSpanElement> = new Map();
    private charts: Map<ReadingKind, Chart> = new Map();

    constructor(root: HTMLElement, readingKinds: Array<ReadingKind>, localization: Localization) {
        this.readingKinds = readingKinds;
        this.card = this.div("card");
            let title = this.div("card-title");
                this.title = this.span("sensor-name");
                this.status = this.span("sensor-status-offline");
                title.appendChild(this.title);
                title.appendChild(this.status);
            this.card.appendChild(title);

            this.subtitle = this.div("card-subtitle");
                this.subtitle.innerText = "Air Sensor";
            this.card.appendChild(this.subtitle);


            let content = this.div("card-content");
                let readings = document.createElement("ul");
                readings.className = "sensor-content-readings";
                this.readingKinds.forEach(kind => {
                    let reading = document.createElement("li");
                    reading.className = "sensor-reading";
                        let kind_span = this.span("sensor-reading-kind");
                        kind_span.innerText = localization.of(kind) + " ";
                        let value_span = this.span("sensor-reading-value");
                        this.readings.set(kind, value_span)
                        reading.appendChild(kind_span);
                        reading.appendChild(value_span);
                    readings.appendChild(reading);
                    let canvas = document.createElement("canvas");
                    let chart = create_chart(canvas, kind);
                    this.charts.set(kind, chart);
                    readings.appendChild(canvas);
                });
                content.appendChild(readings);

            this.card.appendChild(content);

        root.appendChild(this.card);
    }

    public setOnline(online: boolean) {
        let suffix = online ? "online" : "offline";
        let new_class = `sensor-status-${suffix}`;
        if (this.status.className != new_class)
            this.status.className = new_class;
    }

    public setName(name: string) {
        if (this.title.innerText != name)
            this.title.innerText = name;
    }

    public setCurrentReading(kind: ReadingKind, reading: SensorReading) {
        if (reading.kind != kind) {
            throw new Error("Mismatching reading kind");
        }

        let valueSpan = this.readings.get(kind);
        let newValue = reading.render();
        if (valueSpan.innerText  != newValue) {
            valueSpan.innerText = newValue;
        }
    }

    public setChartData(kind: ReadingKind, readings: any) {
        let chart: Chart = this.charts.get(kind);
        chart.data.datasets[0].data = readings;
        this.calculateMinMax(chart);
        chart.update();

    }

    public appendChartData(kind: ReadingKind, readings: any) {
        let chart: Chart = this.charts.get(kind);
        chart.data.datasets[0].data.push(...readings);
        this.calculateMinMax(chart);
        chart.update();
    }

    private calculateMinMax(chart: Chart) {
        let data: any = chart.data.datasets[0].data;

        var min: number;
        var max: number;
        if (data.length == 0) {
            min = 0;
            max = 10;
        } else {
            min = data[0].y;
            max = data[0].y;
        }

        data.forEach(i => {
            max = Math.max(i.y, max);
            min = Math.min(i.y, min);
        });

        chart.options.scales.yAxes[0].ticks.suggestedMin = min - 5;
        chart.options.scales.yAxes[0].ticks.suggestedMax = max + 5;
    }


    private div(className: string): HTMLDivElement {
        let div = document.createElement("div");
        div.className = className;
        return div;
    }

    private span(className: string): HTMLSpanElement {
        let span = document.createElement("span");
        span.className = className;
        return span;
    }
}
