#include "log.h"
#include "uart.h"

#include <stdio.h>
#include <stdarg.h>

static enum LogType Type = Log_None;

void log_init(enum LogType type) {
    Type = type;
}

void log_print(const char* format, ...) {
    va_list args;
    va_start(args, format);

    char buf[128];
    vsnprintf(buf, sizeof(buf), format, args);

    if (Type == Log_UART) {
        uart_transmit_string(buf);
    }

    va_end(args);
}

