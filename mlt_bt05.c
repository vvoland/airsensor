#include "mlt_bt05.h"
#include "uart.h"

#include <stdint.h>

#define BEGIN() uart_transmit_string("AT")
#define SEND(s) uart_transmit_string(s)
#define END()   uart_transmit_string("\r\n")

void bt_mlt05_init() {
    uart_init(9600);
}

void bt_mlt05_set_name(const char* name) {
    BEGIN();
    SEND("+NAME"); SEND(name);
    END();
}

void bt_mlt05_set_pin(const char* pin) {
    BEGIN();
    SEND("+PIN"); SEND(pin);
    END();
    BEGIN();
    SEND("+TYPE2"); SEND(pin);
    END();
}

void bt_mlt05_send(const uint8_t* data, unsigned int size) {
    for (unsigned int i = 0; i < size; i++) {
        uart_transmit_char(data[i]);
    }
}

void bt_mlt05_send_string(const char* str) {
    uart_transmit_string(str);
}

