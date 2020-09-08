## gcfeeder
A vJoy feeder for Gamecube controllers using the Gamecube Controller Adapter.
Inspired by [Wii U GCN USB Driver](http://m4sv.com/page/wii-u-gcn-usb-driver).

The process for reading adapter inputs is based on Dolphin's Gamecube Adapter support.
Since this program uses the same driver as Dolphin, it does not conflict with Dolphin's passthrough.

### Usage Requirements
* Both [vJoy and vJoyConf](http://vjoystick.sourceforge.net/site) must be installed.
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie). For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
* vJoy driver must be enabled and device 1 must have the following configuration:
    * Axes: X, Y, Z, Rx, Ry, Rz
    * Number of Buttons: 12
    * POV Hat Switch: Continuous with 0 POVs
    * Force Feedback effects: May be optionally enabled for rumble support

### Notes
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.
* Only supports port one on the adapter.
* Currently reads Force Feedback packets from all effects on vJoy to control rumble for compatibility (Dolphin seems to crash unless all effects are enabled).
