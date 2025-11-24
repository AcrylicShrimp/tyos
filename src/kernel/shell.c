#include "shell.h"
#include "get_line.h"
#include "puts.h"
#include "string.h"

void print_help();
void print_version();
void print_unknown_command(char *buf);

void enter_shell() {
  char buf[4096];

  for (;;) {
    puts("tyos > ");
    get_line(buf, sizeof(buf));

    if (strcmp(buf, "help") == 0) {
      print_help();
      puts("\n");
      continue;
    }

    if (strcmp(buf, "version") == 0) {
      print_version();
      puts("\n");
      continue;
    }

    print_unknown_command(buf);
    puts("\n");
  }
}

void print_help() {
  puts("Available commands:\n");
  puts("  help    - Print this help message\n");
  puts("  version - Print the version of the system\n");
  puts("\n");
}

void print_version() { puts("tyos 0.0.1\n"); }

void print_unknown_command(char *buf) {
  puts("Unknown command: ");
  puts(buf);
  puts("\n");
  puts("Type 'help' for available commands.\n");
}
