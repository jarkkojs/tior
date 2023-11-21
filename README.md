tior is a serial device tool, inspired by [tio](https://github.com/tio/tio).
It is aimed at testing SBC's, FPGA boards and other embedded devices. It was
originally developed to help with Linux kernel testing.

# Debugging

Debug trace can be acquired by redirecting stderr, as the TTY session only
reserves stdin and stdout:

```
RUST_LOG=debug target/debug/tior open /dev/ttyUSB0 2> log.txt
```
