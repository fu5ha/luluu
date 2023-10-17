# LuLuu

A cute and smart arm-warmer sleeve with a 1.3" full color TFT display built into the back of the
hand ^_^

![PCB drawing][img/board_drawing.png]
![PCB render][img/board_render.png]

## Hardware

Runs on a RaspberryPi RP2040 microcontroller. The small 30x75mm PCB has built-in battery management for a single-cell
LiPo, microSD card storage, and direct connection via SPI to an Adafruit 4520 1.3" TFT (ST7789VW controller). It's
also designed to be expanded via SparkFun Qwiic/Adafruit STEMMA QT compatible (3.3v only) I2C sensor modules. Finallly,
it implements the RaspberryPi 3-wire debug connector spec for ARM SWD serial-wire debugging and RS232-style
UART communication.

The hardware design files all live in the [`hardware/`](hardware/) folder. See the [`README`](hardware/README.md) in
that folder for more.

## Software

TODO. Using Rust embedded crates:

- `embedded-hal` (`rp2040-hal`)
- `embedded-sdmmc`
- `embedded-graphics`
- `tinybmp`/`tinygif`
- `display-interface`
- `mipidsi`

