#include "puts.h"
#include "console_config.h"

int puts(const char *str) {
  int count = 0;

  while (*str) {
    console_ops->putc(*str++);
    ++count;
  }

  return count;
}
