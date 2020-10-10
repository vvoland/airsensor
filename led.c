#include <avr/io.h>

#include "led.h"


void led_on(struct Led led) {
    gpio_set_direction(led.Gpio, GpioOutput);
    gpio_write(led.Gpio, true);
}

void led_off(struct Led led) {
    gpio_set_direction(led.Gpio, GpioOutput);
    gpio_write(led.Gpio, false);
}
