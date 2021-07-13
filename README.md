# gcfeeder
A vJoy feeder and input viewer for GameCube controllers using the GameCube Controller Adapter.

The process for reading adapter inputs is based on Dolphin's GameCube Adapter support.
Since this program uses the same driver as Dolphin, it does not conflict with Dolphin's passthrough.

## ESS Adapter
A built-in ESS adapter for use in Dolphin with mappings for the following games is also included.
* The Legend of Zelda: Ocarina of Time (OoT) on Virtual Console
* The Legend of Zelda: Majora's Mask (MM) on Virtual Console
* OoT and MM on GameCube

## Usage Requirements
* Both [vJoy and vJoyConf](https://github.com/jshafer817/vJoy) must be installed.
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie).
For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
* vJoy driver must be enabled and device 1 must have the following configuration:
    * Axes: X, Y, Z, Rx, Ry, Rz
    * Number of Buttons: 12
    * POV Hat Switch: Continuous with 0 POVs
    * Force Feedback effects: May be optionally enabled for rumble support
* To use the ESS Adapter with the default OoT VC mapping, start the program with the `-e` flag. To specify another mapping use
the `-m <MAP>` option. The following mappings are available.
    * To specify OoT VC use `-m oot-vc`.
    * To specify MM VC use `-m mm-vc`.
    * To specify OoT and MM GC use `-m z64-gc`.
* To create a UDP server for controller inputs on port `4096`, start the program with the `-s` flag. The port can be customized
with the `-p <PORT>` option.

## Themes

To customize the theme of the input viewer, put a `color.frag` according to the specification in the same directory as the executable.
The default theme can be found at `src/viewer/shader/color.frag`. Other themes can be found in `theme`.

## Notes
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.
To use a controller with Dolphin using the feeder, configure the vJoy Device as a Standard Controller instead.
* Only supports port one on the adapter.
* Currently reads Force Feedback packets from all effects on vJoy to control rumble for compatibility (Dolphin seems to crash unless all effects are enabled).
