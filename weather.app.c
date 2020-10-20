#include <avr/io.h>
#include <util/delay.h>

#include <assert.h>
#include <time.h>

#include "uart.h"
#include "led.h"
#include "gpio.h"


#define DHT11_WAIT(microseconds) (_delay_us(microseconds))
#define DHT11_ERROR(err) do { uart_transmit(err); uart_transmit("\r\n"); } while (0)
#define DHT11_TIMEOUT(title, gpio, target, dur_us) \
    do \
    { \
        uint8_t duration = dur_us; \
        uint8_t start = TCNT0; \
        uint8_t start_plus_duration = start + duration; \
        if (start_plus_duration < start) { \
            while (start > TCNT0 + duration && gpio_read(gpio) != target);\
            \
        } else {\
            uint8_t rem = duration - (0xFF - start); \
            while (start > TCNT0 && gpio_read(gpio) != target);\
            while (TCNT0 < rem && gpio_read(gpio) != target);\
        } \
        if (gpio_read(gpio) != target) { \
            uart_printf("DHT11 timeout "title); \
            uart_printf("\r\n"); \
            goto err;\
        } \
        DHT11_WAIT(duration);\
    } while (0)


struct Dht11 {
    struct Gpio Gpio;
};


void dht11_init(struct Dht11* sensor) {
    gpio_set_direction(sensor->Gpio, GpioInput);
    gpio_write(sensor->Gpio, GpioHigh);

    // Wait one second to pass the unstable status
    _delay_ms(1000);
}


bool dht11_read(struct Dht11* sensor, unsigned int* temperature, unsigned int* humidity) {
    struct Gpio gpio = sensor->Gpio;

    gpio_set_direction(gpio, GpioOutput);
    gpio_write(gpio, GpioLow);
    DHT11_WAIT(18000 + 2000); // 18 ms

    gpio_write(gpio, GpioHigh);
    DHT11_WAIT(35); // 20-40 us
    gpio_set_direction(gpio, GpioInput);
    DHT11_WAIT(5);

    // Wait for low signal that should last 80us
    DHT11_TIMEOUT("start low", gpio, GpioLow, 80);
    DHT11_WAIT(40);

    // Wait for high signal that should last 80us
    DHT11_TIMEOUT("start high", gpio, GpioHigh, 80);
    DHT11_WAIT(40);

    struct Response {
        uint8_t HumidityIntegral;
        uint8_t HumidityDecimal;
        uint8_t TemperatureIntegral;
        uint8_t TemperatureDecimal;
        uint8_t Checksum;
    };

    struct Response response;
    uint8_t* response_ptr = (uint8_t*)&response;

    for (uint8_t i = 0; i < 5; i++) {
        for (uint8_t bit = 0; bit < 8; bit++) {
            DHT11_TIMEOUT("data low", gpio, GpioLow, 50);
            DHT11_WAIT(35);
            DHT11_TIMEOUT("data high", gpio, GpioHigh, 50);

            DHT11_WAIT(30);

            if (gpio_read(gpio) == GpioLow) {
                response_ptr[i] &= ~(1 << bit);
            } else {
                response_ptr[i] |= (1 << bit);
                DHT11_WAIT(10);
            }
        }
    }

    int8_t total_checksum = 0;
    total_checksum += response.HumidityIntegral;
    total_checksum += response.HumidityDecimal;
    total_checksum += response.TemperatureIntegral;
    total_checksum += response.TemperatureDecimal;

    uint8_t expected_checksum = total_checksum & 0xFF;

    if (response.Checksum != expected_checksum) {
        DHT11_ERROR("DHT bad checksum");
        uart_printf("got: %x, expected: %x \r\n", response.Checksum, expected_checksum);
        //goto err;
    }

    (*temperature) = response.TemperatureIntegral;
    (*humidity) = response.HumidityIntegral;

    dht11_init(sensor);
    return true;
err:
    dht11_init(sensor);
    return false;
}




int main(void) {

    struct Led read_indicator = {
        .Gpio = {
            .Port = PortB,
            .Pin = PB0
        }
    };

    struct Dht11 dht11 = {
        .Gpio = {
            .Port = PortB,
            .Pin = PB1
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
    uart_printf("Init\r\n");
    dht11_init(&dht11);


    while (1) {

        uart_printf("Time: %d\r\n", TCNT0);
        led_on(read_indicator);
        uart_printf("Time2: %d\r\n", TCNT0);
        uart_transmit("Reading... ");

        unsigned int temperature = 0;
        unsigned int humidity = 0;
        if (dht11_read(&dht11, &temperature, &humidity)) {
            //uart_printf("Temperature: %d | Humidity: %d \r\n", temperature, humidity);
        }

        uart_transmit("Sleeping... \r\n");
        _delay_ms(1000);
        led_off(read_indicator);
        _delay_ms(10000);
    }

    return 0;
}
