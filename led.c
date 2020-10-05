#include <avr/io.h>

#include "led.h"

void led_on(struct Led led) {
    _SFR_BYTE(led.port) |= (1 << led.pin);
}

void led_off(struct Led led) {
    _SFR_BYTE(led.port) &= ~(1 << led.pin);
}
