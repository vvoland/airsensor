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

    struct Led read_indicator = {
        .Gpio = {
            .Port = PortB,
            .Pin = PB0
        }
    };

    // Count every 1us
#if F_CPU == 1000000
    // No timer prescaler
    TCCR0B |= (1 << CS00);
#elif F_CPU == 8000000
    // 8 prescaler
    TCCR0B |= (1 << CS01);
#else
#error "Unsupported CPU speed"
#endif
    uart_init(4800);
    log_init(Log_UART);
    log_print("Init\r\n");

    dht11_init();

    while (1) {

        log_print("Time: %d\r\n", TCNT0);
        led_on(read_indicator);
        log_print("Time2: %d\r\n", TCNT0);
        log_print("Reading... ");

        unsigned int temperature = 0;
        unsigned int humidity = 0;
        if (dht11_read(&temperature, &humidity)) {
            log_print("Temperature: %d | Humidity: %d \r\n", temperature, humidity);
        }

        log_print("Sleeping... \r\n");
        _delay_ms(1000);
        led_off(read_indicator);
        _delay_ms(10000);
    }

    return 0;
}
