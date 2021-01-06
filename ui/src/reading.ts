export enum ReadingKind {
    Temperature = "T",
    Humidity = "H"
}

export interface SensorReading {
    kind: ReadingKind;
    value: number;
    render(): string;
}


export class CelsiusTemperature implements SensorReading {
    kind: ReadingKind = ReadingKind.Temperature;
    value: number;

    constructor(value: number) {
        this.value = value;
    }

    render(): string {
        return `${this.value}C`;
    }
}

export class FahrenheitTemperature implements SensorReading {
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

export class PercentageHumidity implements SensorReading {
    kind: ReadingKind = ReadingKind.Humidity;
    value: number;

    constructor(value: number) {
        this.value = value;
    }

    render(): string {
        return `${this.value}%`;
    }
}
