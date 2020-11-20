#pragma once

enum LogType {
    LogNone,
    LogUART
};

void log_init(enum LogType type);
void log_print(const char* format, ...);
