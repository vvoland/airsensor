#include "dht11.h"
#include "uart.h"
#include "fast_gpio.h"
#include "log.h"

#include <avr/io.h>
#include <util/delay.h>

#define DHT11_PIN       PB1

#define GPIO_WRITE(v) GPIO_B_WRITE(DHT11_PIN, v)
#define GPIO_READ()   GPIO_B_READ(DHT11_PIN)

#define GPIO_OUT()    GPIO_B_OUT(DHT11_PIN)
#define GPIO_IN()     GPIO_B_IN(DHT11_PIN)

#define GPIO_OUT_LOW()    GPIO_B_LOW(DHT11_PIN)
#define GPIO_OUT_HIGH()   GPIO_B_HIGH(DHT11_PIN)

#define DHT11_WAIT(microseconds) (_delay_us(microseconds))
#define DHT11_ERROR(err, ...) do { log_print(err, __VA_ARGS__); log_print("\r\n"); } while (0)
#define DHT11_TIMEOUT(title, target, dur_us) \
    do \
    { \
        uint8_t duration = dur_us; \
        uint8_t start = TCNT0; \
        uint8_t start_plus_duration = start + duration; \
        bool ok = false; \
        if (start_plus_duration > start) { \
            while (start > TCNT0 + duration && !ok) ok |= GPIO_READ() == target;\
        } else {\
            uint8_t rem = duration - (0xFF - start); \
            while (!ok && start > TCNT0) ok |= GPIO_READ() == target;\
            while (!ok && TCNT0 < rem)   ok |= GPIO_READ() == target;\
        } \
        if (!ok && GPIO_READ() != target) { \
            DHT11_ERROR("DHT11 timeout %s", title); \
            goto err; \
        } \
    } while (0)


void dht11_init() {
    GPIO_IN();
    GPIO_OUT_HIGH();

    // Wait one second to pass the unstable status
    _delay_ms(1000);
}


bool dht11_read(int8_t* temperature, uint8_t* humidity) {
    uint8_t i = 0, bit = 0;

    GPIO_OUT();
    GPIO_OUT_LOW();
    DHT11_WAIT(18000 + 1000); // 18 ms

    GPIO_OUT_HIGH();
    DHT11_WAIT(35); // 20-40 us
    GPIO_IN();

    // Wait for low signal that should last 80us
    DHT11_TIMEOUT("start low", GPIO_LOW, 80);
    DHT11_WAIT(70);
    

    // Wait for high signal that should last 80us
    DHT11_TIMEOUT("start high", GPIO_HIGH, 80);
    DHT11_WAIT(70);

    struct Response {
        uint8_t HumidityIntegral;
        uint8_t HumidityDecimal;
        int8_t TemperatureIntegral;
        uint8_t TemperatureDecimal;
        uint8_t Checksum;
    };

    struct Response response;
    uint8_t* response_ptr = (uint8_t*)&response;

    for (i = 0; i < 5; i++) {
        for (bit = 0; bit < 8; bit++) {

            while (GPIO_READ() != GPIO_LOW);
            while (GPIO_READ() != GPIO_HIGH);

            DHT11_WAIT(30);

            if (GPIO_READ() == GPIO_LOW) {
                response_ptr[i] &= ~(1 << (7 - bit));
            } else {
                response_ptr[i] |= (1 << (7 - bit));
                DHT11_WAIT(30);
            }
        }
    }

    uint8_t total_checksum = 0;
    total_checksum += response.HumidityIntegral;
    total_checksum += response.HumidityDecimal;
    total_checksum += response.TemperatureIntegral;
    total_checksum += response.TemperatureDecimal;

    uint8_t expected_checksum = total_checksum & 0xFF;

    if (response.Checksum != expected_checksum) {
        DHT11_ERROR("DHT bad checksum\r\n"
            "got: %x, expected: %x \r\n", response.Checksum, expected_checksum);
        return false;
    }

    (*temperature) = response.TemperatureIntegral;
    (*humidity) = response.HumidityIntegral;

    dht11_init();
    return true;
err:
    DHT11_ERROR("bit %d; i %d\r\n", bit, i);
    dht11_init();
    return false;
}
