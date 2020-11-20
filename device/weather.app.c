#include "dht11.h"
#include "uart.h"
#include "led.h"
#include "gpio.h"
#include "log.h"
#include "mlt_bt05.h"

#include <avr/io.h>
#include <util/delay.h>

#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <time.h>

#define RESPONSE_DATA_LENGTH (3)

enum CommandResult {
    Ok = 0,
    SensorFail = 1,
    InvalidCommand = 2
};

enum CommandResult command_read(uint8_t response[RESPONSE_DATA_LENGTH]) {
    int8_t temperature = 0;
    uint8_t humidity = 0;

    if (dht11_read(&temperature, &humidity)) {
        response[0] = temperature;
        response[1] = humidity;
        return Ok;
    }

    return SensorFail;
}

enum CommandResult command_hello(uint8_t response[RESPONSE_DATA_LENGTH]) {
    response[0] = 0xF0;
    response[1] = 0x14;
    response[2] = 0x4D;
    return Ok;
}

enum CommandResult handle_command(uint8_t command, uint8_t response[RESPONSE_DATA_LENGTH]) {
    switch (command) {
        case 0x66: return command_read(response);
        case 0x10: return command_hello(response);
    }

    return InvalidCommand;
}

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
    //uart_init(4800);
    //log_init(LogUART);

    log_print("Init\r\n");
    bt_mlt05_init();

    dht11_init();

    bt_mlt05_set_name("WeatherWoland");
    bt_mlt05_set_pin("432523");

    while (1) {

        log_print("Wait for command...\r\n");

        led_on(read_indicator);
        uint8_t command = bt_mlt05_receive();
        led_off(read_indicator);

        uint8_t response[RESPONSE_DATA_LENGTH + 1] = { 0 };
        response[0] = (uint8_t)handle_command(command, &response[1]);
        bt_mlt05_send(response, sizeof(response));

        log_print("Sleeping... \r\n");
        _delay_ms(5000);
    }

    return 0;
}
