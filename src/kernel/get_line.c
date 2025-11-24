#include "get_line.h"
#include "console_config.h"

#define KEY_BACKSPACE 0x08
#define KEY_DELETE 0x7F

int get_line(char *buf, int max_len) {
  int i = 0;
  char c;

  if (max_len <= 0) {
    return -1;
  }

  for (;;) {
    console_ops->getc(&c);

    if (c == '\r' || c == '\n') {
      buf[i] = '\0';
      console_ops->putc('\r');
      console_ops->putc('\n');
      return i;
    }

    if (c == KEY_BACKSPACE || c == KEY_DELETE) {
      if (i != 0) {
        i -= 1;
        console_ops->putc('\b');
        console_ops->putc(' ');
        console_ops->putc('\b');
      }

      continue;
    }

    if (i < max_len - 1) {
      if (0x20 <= c && c <= 0x7E) {
        buf[i++] = c;
        console_ops->putc(c);
      }
    }
  }
}
