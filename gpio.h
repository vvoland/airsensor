#pragma once
#include "lib.h"

enum GpioPort {
    PortB,
    PortC,
    PortD
};

enum GpioDirection {
    GpioOutput = 0,
    GpioInput = 1
};

struct Gpio {
    enum GpioPort Port;
    uint8_t Pin;
};

void gpio_set_direction(struct Gpio gpio, enum GpioDirection direction);
enum GpioDirection gpio_get_direction(struct Gpio gpio);
void gpio_write(struct Gpio gpio, bool value);
bool gpio_read(struct Gpio gpio);

