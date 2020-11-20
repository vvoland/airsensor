#pragma once
#include "gpio.h"

struct Led {
    struct Gpio Gpio;
};

void led_on(struct Led led);
void led_off(struct Led led);

