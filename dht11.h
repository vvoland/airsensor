#pragma once
#include "lib.h"

void dht11_init();
bool dht11_read(unsigned int* temperature, unsigned int* humidity);

