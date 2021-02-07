import { ReadingKind } from "./reading";
import { SensorCardView } from "./sensor-card";
import { DummyLocalization } from "./localization";
import { SensorUpdater } from "./sensor-updater";

var salon: SensorCardView;
var salonUpdater: SensorUpdater;

export async function init() {
    let sensorsContainer = document.getElementById("sensors-container")!;
    salon = new SensorCardView(sensorsContainer,
        [ReadingKind.Temperature, ReadingKind.Humidity],
        new DummyLocalization());
    salon.setName("Salon");
    salonUpdater = new SensorUpdater(salon, 1);

    let updateAllAndScheduleNext = async (interval) => {
        await salonUpdater.refresh();
        setTimeout(() => updateAllAndScheduleNext(interval), interval);
    };

    await updateAllAndScheduleNext(30 * 1000);
}

if (document.readyState === "complete") {
    console.log("Init now!");
    init();
} else {
    console.log("Init onload!");
    document.onreadystatechange = () => {
        if (document.readyState === "complete") {
            console.log("Init now!");
            init();
        }
    };
}
