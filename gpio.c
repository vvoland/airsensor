#include "lib.h"
#include "gpio.h"

#include <stddef.h>
#include <stdint.h>
#include <avr/io.h>

static uint8_t* get_direction_address(enum GpioPort port) {
    switch (port) {
        case PortB: return (uint8_t*)&DDRB;
        case PortC: return (uint8_t*)&DDRC;
        case PortD: return (uint8_t*)&DDRD;
        default: break;
    }
    return NULL;
}

static uint8_t* get_data_address(enum GpioPort port) {
    switch (port) {
        case PortB: return (uint8_t*)&PORTB;
        case PortC: return (uint8_t*)&PORTC;
        case PortD: return (uint8_t*)&PORTD;
        default: break;
    }
    return NULL;
}

void gpio_set_direction(struct Gpio gpio, enum GpioDirection direction) {
    uint8_t* addr = get_direction_address(gpio.Port);
    switch (direction) {
        case GpioInput:
            (*addr) |= (1 << gpio.Port);
            break;
        case GpioOutput:
            (*addr) &= ~(1 << gpio.Port);
            break;
    }
}

enum GpioDirection gpio_get_direction(struct Gpio gpio) {
    uint8_t* addr = get_direction_address(gpio.Port);
    switch (*addr) {
        case 0: return GpioOutput;
        case 1: return GpioInput;
        default: break;
    }

    return -1;
}

void gpio_write(struct Gpio gpio, bool value) {
    uint8_t* addr = get_data_address(gpio.Port);
    if (value) {
        (*addr) |= (1 << gpio.Port);
    } else {
        (*addr) &= ~(1 << gpio.Port);
    }
}

bool gpio_read(struct Gpio gpio) {
    uint8_t* addr = get_data_address(gpio.Port);
    return ((*addr) & (1 << gpio.Port)) != 0 ? true : false;
}
