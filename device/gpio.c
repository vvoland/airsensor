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

static uint8_t* get_output_data_address(enum GpioPort port) {
    switch (port) {
        case PortB: return (uint8_t*)&PORTB;
        case PortC: return (uint8_t*)&PORTC;
        case PortD: return (uint8_t*)&PORTD;
        default: break;
    }
    return NULL;
}

static uint8_t* get_input_data_address(enum GpioPort port) {
    switch (port) {
        case PortB: return (uint8_t*)&PINB;
        case PortC: return (uint8_t*)&PINC;
        case PortD: return (uint8_t*)&PIND;
        default: break;
    }
    return NULL;
}

void gpio_set_direction(struct Gpio gpio, enum GpioDirection direction) {
    uint8_t* addr = get_direction_address(gpio.Port);
    switch (direction) {
        case GpioInput:
            (*addr) &= ~(1 << gpio.Pin);
            break;
        case GpioOutput:
            (*addr) |= (1 << gpio.Pin);
            break;
    }
    __asm("nop");
}

void gpio_set_direction_optimized(struct Gpio gpio, enum GpioDirection direction) {
    /*
    41 cycles
    static uint8_t* const addresses[] = {
        (uint8_t*)&DDRB, (uint8_t*)&DDRC, (uint8_t*)&DDRD
    };
    const uint8_t val = 1 << gpio.Pin;

    if (direction == GpioInput)
        *(addresses[(int)gpio.Port]) &= ~val;
    else
        *(addresses[(int)gpio.Port]) |= val;
    */
    /* 32 cycles */
    const uint8_t val = 1 << gpio.Pin;
    
    if (direction == GpioInput) {
        if (gpio.Port == PortB)
            PORTB &= ~val;
        else if (gpio.Port == PortC)
            PORTC &= ~val;
        else if (gpio.Port == PortD)
            PORTD &= ~val;
    } else {
        if (gpio.Port == PortB)
            PORTB |= val;
        else if (gpio.Port == PortC)
            PORTC |= val;
        else if (gpio.Port == PortD)
            PORTD |= val;
    }

    __asm("nop");
}

enum GpioDirection gpio_get_direction(struct Gpio gpio) {
    uint8_t* addr = get_direction_address(gpio.Port);
    if (((*addr) & (1 << gpio.Pin)) != 0)
        return GpioOutput;
    return GpioInput;
}

void gpio_write(struct Gpio gpio, enum GpioValue value) {
    uint8_t* addr = get_output_data_address(gpio.Port);
    switch (value) {
        case GpioHigh:
            (*addr) |= (1 << gpio.Pin);
            break;
        case GpioLow:
            (*addr) &= ~(1 << gpio.Pin);
            break;
        default:
            break;
    }
    __asm("nop");
}

enum GpioValue gpio_read(struct Gpio gpio) {
    uint8_t* addr = get_input_data_address(gpio.Port);
    return ((*addr) & (1 << gpio.Pin)) != 0 ? GpioHigh : GpioLow;
}
