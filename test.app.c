#include <avr/io.h>
#include <util/delay.h>

#include "uart.h"
#include "led.h"
#include "gpio.h"
#include "fast_gpio.h"

#define DELAYTIME 1000

#define MEASURE do {\
    uint8_t start; \
    uint8_t end; \
    start = TCNT0;

#define END_MEASURE(msg, ...) \
    end = TCNT0;\
    unsigned int duration;\
    if (end < start)\
    {\
        duration = 0xFF - start;\
        duration += end;\
    }\
    else\
    {\
        duration = end - start;\
    }\
    uart_printf("%d cycles | " msg "\r\n", duration, ##__VA_ARGS__);\
} while (0)


int main(void) {

    uart_init(4800);

    // No timer prescaler
    TCCR0B |= (1 << CS00);

    struct Gpio gpio = {
        .Port = PORTB,
        .Pin = PB0
    };

    while (1) {
        MEASURE;
        gpio_set_direction(gpio, GpioOutput);
        END_MEASURE("gpio_set_direction(Output)");

        MEASURE;
        gpio_write(gpio, GpioHigh);
        END_MEASURE("gpio_write(High)");

        MEASURE;
        gpio_write(gpio, GpioLow);
        END_MEASURE("gpio_write(Low)");

        MEASURE;
        gpio_set_direction_optimized(gpio, GpioInput);
        END_MEASURE("gpio_set_direction_optimized(Input)");

        MEASURE;
        volatile enum GpioValue val = gpio_read(gpio);
        END_MEASURE("gpio_read -> %d", val);

        uart_printf("==========\r\n");

        MEASURE;
        GPIO_B_OUT(PB0);
        END_MEASURE("GPIO_B_OUT(PB0)");

        MEASURE;
        GPIO_B_WRITE(PB0, GPIO_HIGH);
        END_MEASURE("GPIO_B_WRITE(PB0, GPIO_HIGH)");

        MEASURE;
        GPIO_B_WRITE(PB0, GPIO_LOW);
        END_MEASURE("GPIO_B_WRITE(PB0, GPIO_LOW)");

        MEASURE;
        GPIO_B_IN(PB0);
        END_MEASURE("GPIO_B_IN(PB0)");

        MEASURE;
        volatile int val_raw = ((GPIO_B_READ(PB0))) != 0 ? 1 : 0;
        END_MEASURE("GPIO_B_READ(PB0) -> %d", val_raw);

        uart_printf("==========\r\n");

        MEASURE;
        DDRB |= (1 << PB0);
        END_MEASURE("DDRB |= (1 << PB0)");

        MEASURE;
        PORTB |= (1 << PB0);
        END_MEASURE("PORTB |= (1 << PB0)");

        MEASURE;
        PORTB &= ~(1 << PB0);
        END_MEASURE("PORTB &= ~(1 << PB0)");

        MEASURE;
        DDRB &= ~(1 << PB0);
        END_MEASURE("DDRB &= ~(1 << PB0);");

        MEASURE;
        volatile int val_raw = ((PINB & (1 << PB0))) != 0 ? 1 : 0;
        END_MEASURE("PINB & (1 << PB0) -> %d", val_raw);

        uart_printf("==========\r\n");

        _delay_ms(DELAYTIME);
    }

    return 0;
}
