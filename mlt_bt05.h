#pragma once
#include <stdint.h>

void bt_mlt05_init();
void bt_mlt05_send(const uint8_t* data, const unsigned int length);
void bt_mlt05_send_string(const char* str);

void bt_mlt05_set_name(const char* name);
void bt_mlt05_set_pin(const char* pin);
void bt_mlt05_send(const uint8_t* data, unsigned int size);
void bt_mlt05_send_string(const char* str);
uint8_t bt_mlt05_receive();
unsigned int bt_mlt05_receive_string(char* buffer, unsigned int buffer_size);

