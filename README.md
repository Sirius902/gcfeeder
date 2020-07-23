## gcfeeder
A vJoy feeder written in Rust for Gamecube controllers using the Gamecube Controller Adapter.
Inspired by [Wii U GCN USB Driver](http://m4sv.com/page/wii-u-gcn-usb-driver).

The process for reading adapter inputs is based on Dolphin's Gamecube Adapter support and therefore is compatible with Dolphin's passthrough.

### Notes
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.

### Usage Requirements
* Both [vJoy and vJoyConf](http://vjoystick.sourceforge.net/site) must be installed.
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie). For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
