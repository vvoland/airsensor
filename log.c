#include "log.h"
#include "uart.h"

#include <stdio.h>
#include <stdarg.h>

static enum LogType Type = LogNone;

void log_init(enum LogType type) {
    Type = type;
}

void log_print(const char* format, ...) {
    va_list args;
    va_start(args, format);

    if (Type == LogUART) {
        char buf[128];
        vsnprintf(buf, sizeof(buf), format, args);
        uart_transmit_string(buf);
    }

    va_end(args);
}

