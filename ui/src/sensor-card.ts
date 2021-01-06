import { ReadingKind, SensorReading } from "./reading";
import { Localization } from "./localization";

export class SensorCardRenderer {
    private readings: Map<ReadingKind, SensorReading> = new Map();
    private name: string;
    private online: boolean;

    constructor(name: string, online: boolean) {
        this.name = name;
        this.online = online;
    }

    with(reading: SensorReading) {
        this.readings.set(reading.kind, reading);
        return this;
    }

    render(localization: Localization): string {
        var result = `
            <div class="card">
                <div class="card-title">
                     ${this.name}
        `;

        let status_class = "sensor-status-" + (this.online ? "online" : "offline");
        result += `
                <span class="${status_class}"></span>
                </div>
        `;

        this.readings.forEach((reading, kind) => {
            let value = reading.render();
            let kind_str = localization.of(kind);
            result += `
                    <div class="card-content">
                        <p class="sensor-reading">
                            <span class="sensor-reading-kind">${kind_str}</span>
                            <span class="sensor-reading-value">${value}</span>
                        </p>
                    </div>`;
        });
        result += `
                    <canvas id="sensor-chart-${this.name}"></canvas>
            </div>`;

        return result;
    }
}
