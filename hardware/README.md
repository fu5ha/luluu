# Hardware

The main part of this folder is the [KiCad](https://www.kicad.org/) project for the main PCB. Open it with KiCad v7+.
There are production files for PCB production and assembly from JLCPCB in the `production/` folder. There is also a
small 3d printed "front panel" part with exported STL files in the `FrontPanel/` folder.

## Online Schematic and Board View

You can view an explorable online version of the schematic and board layout here:
https://kicanvas.org/?github=https%3A%2F%2Fgithub.com%2Ffu5ha%2Fluluu%2Ftree%2Fmain%2Fhardware

## Order and assembly

The board is specifically designed to be economic and easy to order with JLCPCB's
"economic" PCB assembly service. Boards received from JLC will be mostly-assembled
but still require a few pieces of hand assembly: installing the battery connector,
power switch, display connection cable, (optionally) debug connectors, and
(optionally) STEMMAqt connector.

TODO: document BOM, assembly process, etc more

## Front Panel

The front panel part was designed in OnShape. The exported files (printed at JLCPCB in my case but could be printed on
any 3d printer really) can be found in the `FrontPanel/` folder. The original OnShape document can be viewed and
remixed here:

https://cad.onshape.com/documents/f8b0c88e73a74b1a499f2bc3/w/d7cf90c156d1df4edfe95550/e/d6b63c8b8542f003553d2207?renderMode=0&uiState=652e94bf76c80a49126fcdde
