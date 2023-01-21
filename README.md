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
### Windows
* WinUSB (libusb) driver must be installed for the adapter (WUP-028) with [Zadig](https://zadig.akeo.ie).
For a tutorial follow Dolphin's guide [here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
* [ViGEmBus](https://github.com/ViGEm/ViGEmBus/releases) must be installed.

### Linux
* Follow Dolphin's Linux GameCube Adapter setup guide
[here](https://dolphin-emu.org/docs/guides/how-use-official-gc-controller-adapter-wii-u).
    * **Note**: You may want to use the `udev` rules file from the
    [Dolphin repository](https://github.com/dolphin-emu/dolphin/blob/master/Data/51-usb-device.rules)
    as it has been updated compared to the guide.
* You must also add a `udev` rule to allow the `input` group (or a group of your choosing) to access
`uinput`. Doing this will allow the `input` group to create a virtual controller.
    * A sample rules file for this can be found [here](rules/51-input-udev.rules).
    * Make sure to place the rules file in `/etc/udev/rules.d` and reload the `udev` rules with the
    following command.
    ```sh
    sudo udevadm control --reload-rules
    ```
* Change the permissions of `gcfeeder` make it part of the `input` group and allow it to set its group ID.
    * This can be done by running the following commands.
    ```sh
    sudo chown :input gcfeeder
    sudo chmod g+s gcfeeder
    ```
    * You may choose to make `gcfeeder` owned by root and place it in `/usr/local/bin` to prevent malicious
    programs from using it to obtain access to `uinput`. To do that you would instead run this command to
    change its owner and group.
    ```sh
    sudo chown root:input gcfeeder
    ```

## Config

### Location
* Windows
    * `%appdata%\gcfeeder\gcfeeder.toml`
* Linux
    * Stored at one of the following locations:
        * `$XDG_CONFIG_HOME/gcfeeder/gcfeeder.toml`
        * `$HOME/.config/gcfeeder/gcfeeder.toml`

### ViGEm Options (Windows)
Options found under the `vigem_config` key.
* `pad` can be `x360` to emulate an Xbox 360 controller, and `ds4` to emulate a DualShock 4 controller.
* `trigger_mode` can be:
    * `analog` - Only the analog trigger input on the GameCube controller will be mapped to the output trigger.
    * `digital` - Only the digital trigger input on the GameCube controller will be mapped to the output trigger.
    * `combination` - If the digital trigger is pressed output trigger will max. Otherwise, analog trigger input is used.
    * `stick_click` - Digital trigger inputs will be treated as a stick click. Trigger output uses analog trigger input.

## Notes
* libusb does not allow more than one process to interface with a device at a time, so, the feeder may not be active when using Dolphin's passthrough.
