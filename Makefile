F_CPU = 1000000UL

CC = avr-gcc
OBJCOPY = avr-objcopy

CFLAGS = -Os -g -Wall -Wextra
CFLAGS += -I/usr/lib/avr/include/
CFLAGS += -DF_CPU=$(F_CPU)
LDFLAGS = 
ARCH = -mmcu=atmega328p

OBJDIR = obj
BINDIR = bin

.DEFAULT_GOAL := weather

$(OBJDIR)/%.o: %.c
	@mkdir -p $(OBJDIR)
	$(CC) $(ARCH) $(CFLAGS) $(CPPFLAGS) -c -o $@ $<

$(BINDIR)/blink.elf: $(OBJDIR)/blink.o $(OBJDIR)/uart.o $(OBJDIR)/led.o
	@mkdir -p $(BINDIR)
	$(CC) $(ARCH) $(LDFLAGS) $^ -o $@

$(BINDIR)/weather.elf: $(OBJDIR)/weather.o $(OBJDIR)/uart.o $(OBJDIR)/led.o
	@mkdir -p $(BINDIR)
	$(CC) $(ARCH) $(LDFLAGS) $^ -o $@

$(BINDIR)/%.hex: $(BINDIR)/%.elf
	$(OBJCOPY) -j .text -j .data -O ihex $< $@

clean:
	rm -r $(BINDIR)
	rm -r $(OBJDIR)

.PHONY: blink
blink: $(BINDIR)/blink.hex

.PHONY: weather
weather: $(BINDIR)/weather.hex
