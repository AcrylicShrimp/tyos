#include "console_config.h"
#include "shell.h"

void kmain() {
  console_ops->init();
  enter_shell();
}
