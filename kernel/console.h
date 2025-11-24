typedef struct {
  void (*putc)(char c);
  void (*getc)(char *c);
} console_ops_t;
