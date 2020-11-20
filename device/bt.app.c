#include "dht11.h"
#include "uart.h"
#include "led.h"
#include "gpio.h"
#include "log.h"

#include <avr/io.h>
#include <util/delay.h>

#include <assert.h>
#include <time.h>

int main(void) {

    struct Led led = {
        .Gpio = {
            .Port = PortB,
            .Pin = PB0
        }
    };

    led_off(led);

    uart_init(9600);

    led_on(led);
    uart_transmit("AT\r\n");
    _delay_ms(1000);
    led_off(led);

    led_on(led);
    uart_transmit("AT+NAMEWoland\r\n");
    _delay_ms(1000);
    led_off(led);

    led_on(led);
    uart_transmit("AT+PIN432523\r\n");
    _delay_ms(1000);
    led_off(led);

    led_on(led);
    uart_transmit("AT+TYPE2\r\n");
    _delay_ms(1000);
    led_off(led);


    while (1) {
        _delay_ms(10000);
        led_on(led);
        uart_transmit("ABCDEF\r\n");
        _delay_ms(1000);
        led_off(led);
    }

    return 0;
}
