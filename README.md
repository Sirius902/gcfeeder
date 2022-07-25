# gcfeeder
A ViGEm feeder for GameCube controllers using the GameCube Controller Adapter.

The process for reading adapter inputs is based on Dolphin's GameCube Adapter support.
Since this program uses the same driver as Dolphin, it does not conflict with Dolphin's passthrough.

## Input Viewer
[gcviewer](https://github.com/Sirius902/gcviewer) is an input viewer that can be used with gcfeeder.
It used to be a part of this repository but has moved to its own after commit
[c4c65b2](https://github.com/Sirius902/gcfeeder/commit/c4c65b291bec4ac31879d24497caa13c22acbe81).

## ESS Adapter
A built-in ESS adapter for use in Dolphin with mappings for the following games is also included.
* The Legend of Zelda: Ocarina of Time (OoT) on Virtual Console
* The Legend of Zelda: Majora's Mask (MM) on Virtual Console
* OoT and MM on GameCube

## Usage Requirements
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie).
For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
### **ViGEm**
* [ViGEmBus](https://github.com/ViGEm/ViGEmBus/releases) must be installed.

## Program Arguments
For info on the program arguments, run with `--help`.

## Config
The configuration is stored in `gcfeeder.json`.

### ViGEm Options
Options found under the `vigem_config` key.
* `pad` can be `x360` to emulate an Xbox 360 controller, and `ds4` to emulate a DualShock 4 controller.
* `trigger_mode` can be:
    * `analog` - Only the analog trigger input on the GameCube controller will be mapped to the output trigger.
    * `digital` - Only the digital trigger input on the GameCube controller will be mapped to the output trigger.
    * `combination` - If the digital trigger is pressed output trigger will max. Otherwise, analog trigger input is used.
    * `stick_click` - Digital trigger inputs will be treated as a stick click. Trigger output uses analog trigger input.

## Notes
* Only supports port one on the adapter.
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.
