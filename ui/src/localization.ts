import { ReadingKind } from "./reading";

export interface Localization {
    of(kind: ReadingKind): string;
}

export class DummyLocalization implements Localization {
    of(kind: ReadingKind): string {
        switch (kind) {
            case ReadingKind.Temperature: return "Temperature";
            case ReadingKind.Humidity: return "Humidity";
        }
    }
}
