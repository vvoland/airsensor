#pragma once
#include "lib.h"

enum GpioPort {
    PortB,
    PortC,
    PortD
};

enum GpioDirection {
    GpioInput = 0,
    GpioOutput = 1
};

enum GpioValue {
    GpioLow = 0,
    GpioHigh = 1
};

struct Gpio {
    enum GpioPort Port;
    uint8_t Pin;
};

void gpio_set_direction(struct Gpio gpio, enum GpioDirection direction);
void gpio_set_direction_optimized(struct Gpio gpio, enum GpioDirection direction);
enum GpioDirection gpio_get_direction(struct Gpio gpio);
void gpio_write(struct Gpio gpio, enum GpioValue value);
enum GpioValue gpio_read(struct Gpio gpio);

