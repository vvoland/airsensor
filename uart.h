#pragma once

void uart_init(unsigned int baud_rate);
void uart_transmit_char(unsigned char character);
void uart_transmit_string(const char* string);
void uart_transmit(const char* string);
int uart_printf(const char* fmt, ...);

