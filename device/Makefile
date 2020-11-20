F_CPU = 8000000UL

CC = avr-gcc
LD = avr-ld
OBJCOPY = avr-objcopy

CFLAGS = -Ofast -Wall -Wextra
CFLAGS += -I/usr/lib/avr/include/
CFLAGS += -DF_CPU=$(F_CPU)
LDFLAGS = 

ifeq ($(CC), clang)
	CFLAGS += -target avr
	LDFLAGS += -target avr
	CFLAGS += -D__DELAY_BACKWARD_COMPATIBLE__
endif

MCU = atmega328p
ARCH = -mmcu=$(MCU)

OBJDIR = obj
BINDIR = bin

SRCS = $(wildcard *.c)
SRCS := $(filter-out $(wildcard *.app.c), $(SRCS))
OBJS = $(addprefix $(OBJDIR)/, $(patsubst %.c, %.o, $(SRCS)))

.DEFAULT_GOAL := weather

$(OBJDIR)/%.o: %.c
	@mkdir -p $(OBJDIR)
	$(CC) $(ARCH) $(CFLAGS) $(CPPFLAGS) -c -o $@ $<

$(BINDIR)/%.elf: $(OBJS) $(OBJDIR)/%.app.o
	@mkdir -p $(BINDIR)
	$(CC) $(ARCH) $(LDFLAGS) $^ -o $@

$(BINDIR)/%.hex: $(BINDIR)/%.elf
	avr-size -C --mcu=$(MCU) $^
	$(OBJCOPY) -j .text -j .data -O ihex $< $@

db: clean
	make blink test weather -n | compiledb

clean:
	rm -r $(BINDIR) || true
	rm -r $(OBJDIR) || true

.PHONY: blink
blink: $(BINDIR)/blink.hex

.PHONY: bt
bt: $(BINDIR)/bt.hex

.PHONY: test
test: $(BINDIR)/test.hex

.PHONY: weather
weather: $(BINDIR)/weather.hex
