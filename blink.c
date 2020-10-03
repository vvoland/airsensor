#include <avr/io.h>
#include <util/delay.h>


#define DELAYTIME 1000

#define setBit(sfr, bit)     (_SFR_BYTE(sfr) |= (1 << bit))
#define clearBit(sfr, bit)   (_SFR_BYTE(sfr) &= ~(1 << bit))
#define toggleBit(sfr, bit)  (_SFR_BYTE(sfr) ^= (1 << bit))

int main(void) {

  setBit(DDRB, PB0);

  while (1) {
    setBit(PORTB, PB0);
    _delay_ms(DELAYTIME);

    clearBit(PORTB, PB0);
    _delay_ms(DELAYTIME);
  }

  return 0;
}
