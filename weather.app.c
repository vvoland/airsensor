#include <avr/io.h>
#include <util/delay.h>

#include <assert.h>

#include "uart.h"
#include "led.h"
#include "gpio.h"

int main(void) {

    struct Led read_indicator = {
        .Gpio = {
            .Port = PortB,
            .Pin = PB0
        }
    };

    uart_init(4800);

    while (1) {

        led_on(read_indicator);

        uart_transmit("Temperature: 26C\r\n");

        led_off(read_indicator);

        _delay_ms(6000);
    }

    return 0;
}
