#ifndef CONSOLE_H
#define CONSOLE_H

typedef struct {
  void (*init)();
  void (*putc)(char c);
  void (*getc)(char *c);
} console_ops_t;

#endif
