#pragma once

enum LogType {
    Log_None,
    Log_UART
};

void log_init(enum LogType type);
void log_print(const char* format, ...);
