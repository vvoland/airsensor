//import * as d3 from "d3";

enum ReadingKind {
    Temperature = "T",
    Humidity = "H"
}

interface SensorReading {
    kind: ReadingKind;
    value: number;
    render(): string;
}

interface Localization {
    of(kind: ReadingKind): string;
}

class DummyLocalization implements Localization {
    of(kind: ReadingKind): string {
        switch (kind) {
            case ReadingKind.Temperature: return "Temperature";
            case ReadingKind.Humidity: return "Humidity";
        }
    }
}

class CelsiusTemperature implements SensorReading {
    kind: ReadingKind = ReadingKind.Temperature;
    value: number;

    constructor(value: number) {
        this.value = value;
    }

    render(): string {
        return `${this.value}C`;
    }
}

class FahrenheitTemperature implements SensorReading {
    kind: ReadingKind = ReadingKind.Temperature;
    value: number;

    constructor(value: number) {
        this.value = value;
    }

    render(): string {
        let value = this.value * 1.8 + 32.0;
        return `${value}F`;
    }
}

class PercentageHumidity implements SensorReading {
    kind: ReadingKind = ReadingKind.Humidity;
    value: number;

    constructor(value: number) {
        this.value = value;
    }

    render(): string {
        return `${this.value}%`;
    }
}

class SensorCardRenderer {
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
            </div>`;

        return result;
    }
}

async function fetch_latest(sensor_id: Number, kind: ReadingKind): Promise<number> {
    return await fetch(`http://${window.location.host}/api/sensors/${sensor_id}/latest/${kind}`)
        .then(response => response.json())
        .catch(error => {
            console.log(`Failed to get latest ${kind} reading for sensor ${sensor_id}`);
            return {"value": 0};
        })
        .then(data => data.value);
}

async function fetch_status(sensor_id: Number): Promise<boolean> {
    return await fetch(`http://${window.location.host}/api/sensors/${sensor_id}`)
        .then(response => response.json())
        .catch(error => {
            console.log(`Failed to get status of sensor ${sensor_id}`);
            return {"status": "Offline"};
        })
        .then(data => data.status)
        .then(status => {
            return status == "Online" ? true : false;
        });
}

async function refresh() {
    console.log("Refresh!");
    let temperature = await fetch_latest(1, ReadingKind.Temperature);
    let humidity = await fetch_latest(1, ReadingKind.Humidity);
    let online = await fetch_status(1);

    document.getElementById("sensors-container").innerHTML = new SensorCardRenderer("Salon", online)
        .with(new CelsiusTemperature(temperature))
        .with(new PercentageHumidity(humidity))
        .render(new DummyLocalization());
}


function init() {
    refresh();
    setInterval(refresh, 5000);
}
