tior is a serial device tool, inspired by [tio](https://github.com/tio/tio).

It is aimed at testing SBC's, FPGA's and other similar devices.

# Keyboard commands

* Prefix: `<CTRL-T>`
* Quit the program: `<PREFIX> <CTRL-Q>`
* Send file: `<PREFIX> <CTRL-S>`

# Debugging

Debug trace can be acquired by redirecting stderr, as the TTY session only
reserves stdin and stdout:

```
RUST_LOG=debug target/debug/tior open /dev/ttyUSB0 2> log.txt
```
