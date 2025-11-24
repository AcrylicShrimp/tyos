#include "mcf5208_uart0_console.h"

#define UART0_BASE 0xFC060000

#define UART0_USR ((volatile unsigned char *)(UART0_BASE + 0x04))
#define UART0_UTB ((volatile unsigned char *)(UART0_BASE + 0x0C))
#define UART0_UCR ((volatile unsigned char *)(UART0_BASE + 0x08))

#define CMD_RESET_TX 0x30
#define CMD_TX_ENABLE 0x04
#define CMD_RX_ENABLE 0x01

#define USR_TXRDY (1 << 2)
#define USR_RXRDY (1 << 0)

void console_init() {
  *UART0_UCR = CMD_RESET_TX;
  *UART0_UCR = CMD_TX_ENABLE | CMD_RX_ENABLE;
}

void console_putc(char c) {
  while (!(*UART0_USR & USR_TXRDY))
    ;

  *UART0_UTB = c;

  if (c == '\n') {
    while (!(*UART0_USR & USR_TXRDY))
      ;

    *UART0_UTB = '\r';
  }
}

void console_getc(char *c) {
  while (!(*UART0_USR & USR_RXRDY))
    ;

  *c = *UART0_UTB;
}

console_ops_t mcf5208_uart0_console_ops = {
    .init = console_init,
    .putc = console_putc,
    .getc = console_getc,
};
