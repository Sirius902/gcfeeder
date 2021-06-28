# gcfeeder
A vJoy feeder and input viewer for Gamecube controllers using the Gamecube Controller Adapter.

The process for reading adapter inputs is based on Dolphin's Gamecube Adapter support.
Since this program uses the same driver as Dolphin, it does not conflict with Dolphin's passthrough.

A built-in ESS adapter for use in Dolphin with The Legend of Zelda: Ocarina of Time is also included
with code and inversion methods from Skuzee's [ESS-Adapter](https://github.com/Skuzee/ESS-Adapter)
project.

## Usage Requirements
* Both [vJoy and vJoyConf](https://github.com/jshafer817/vJoy) must be installed.
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie).
For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
* vJoy driver must be enabled and device 1 must have the following configuration:
    * Axes: X, Y, Z, Rx, Ry, Rz
    * Number of Buttons: 12
    * POV Hat Switch: Continuous with 0 POVs
    * Force Feedback effects: May be optionally enabled for rumble support
* To use the ESS Adapter, start the program with the `-e` flag.
* To create a UDP server for controller inputs on port `4096`, start the program with the `-i` flag.

## Notes
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.
* Only supports port one on the adapter.
* Currently reads Force Feedback packets from all effects on vJoy to control rumble for compatibility (Dolphin seems to crash unless all effects are enabled).
