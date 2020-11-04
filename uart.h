#pragma once
#include <stdint.h>

void uart_init(unsigned int baud_rate);
void uart_transmit_char(unsigned char character);
void uart_transmit_string(const char* string);
void uart_transmit(const char* string);
uint8_t uart_receive_byte();
unsigned int uart_receive_string(char* buffer, unsigned int size);

