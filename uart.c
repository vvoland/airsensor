#include <avr/io.h>
#include <stdarg.h>
#include <math.h>
#include <stdio.h>

#include "uart.h"

void uart_init(unsigned int baud_rate) {
    unsigned int ubrr = F_CPU / 16 / baud_rate - 1;

    UBRR0H = (unsigned char)((ubrr >> 8) & 0xFF);
    UBRR0L = (unsigned char)((ubrr >> 0) & 0xFF);

    // Enable receiver, enable transmitter
    UCSR0B = (1 << RXEN0) | (1 << TXEN0);

    // Frame format: 1 stop bit, 8data
    UCSR0C = (0 << USBS0) | (3 << UCSZ00);
}

inline void uart_transmit_char(unsigned char character) {
    // Wait for empty transmit buffer
    while ((UCSR0A & (1 << UDRE0)) == 0);

    UDR0 = character;
}

inline void uart_transmit_string(const char* string) {
    while (*string)
        uart_transmit_char(*string++);
}

inline void uart_transmit(const char* string) {
    uart_transmit_string(string);
}

