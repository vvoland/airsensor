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

    constructor(name: string) {
        this.name = name;
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
                </div>`;

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

function refresh() {
    console.log("Refresh!");
    document.getElementById("sensors-container").innerHTML = new SensorCardRenderer("Salon")
        .with(new CelsiusTemperature(25))
        .with(new PercentageHumidity(57))
        .render(new DummyLocalization());
    document.getElementById("sensors-container").innerHTML += new SensorCardRenderer("Kitchen")
        .with(new CelsiusTemperature(28))
        .with(new PercentageHumidity(77))
        .render(new DummyLocalization());
    fetch("https://woland.xyz");
}


function init() {
    refresh();
    setInterval(refresh, 5000);
}
