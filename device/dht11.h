#pragma once
#include "lib.h"

void dht11_init();
bool dht11_read(int8_t* temperature, uint8_t* humidity);

