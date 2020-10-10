#include <avr/io.h>
#include <util/delay.h>

#include "uart.h"
#include "led.h"

#define DELAYTIME 1000

int main(void) {

    uart_init(4800);

    struct Led led = {
        .Gpio = {
            .Port = PORTB,
            .Pin = PB0
        }
    };
    led_on(led);

    while (1) {
        uart_transmit("ON\r\n");
        led_on(led);
        _delay_ms(DELAYTIME);

        uart_transmit("OFF\r\n");
        led_off(led);
        _delay_ms(DELAYTIME);
    }

    return 0;
}
