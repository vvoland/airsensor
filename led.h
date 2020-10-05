#pragma once

struct Led {
    unsigned int port;
    unsigned int pin;
};

void led_on(struct Led led);
void led_off(struct Led led);

