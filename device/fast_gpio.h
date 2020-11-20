#pragma once

#define GPIO_LOW  0
#define GPIO_HIGH 1

#define GPIO_B_HIGH(pin)   do { PORTB |=   (1 << pin); } while(0)
#define GPIO_B_LOW(pin)    do { PORTB &=  ~(1 << pin); } while(0)
//#define GPIO_B_READ(pin)     (((PINB) >> pin) & 0x1)
#define GPIO_B_READ(pin)     (((PINB & (1 << pin)) != 0) ? GPIO_HIGH : GPIO_LOW)

#define GPIO_B_IN(pin)    do { DDRB &= ~(1 << pin); } while(0)
#define GPIO_B_OUT(pin)     do { DDRB |=  (1 << pin); } while(0)

