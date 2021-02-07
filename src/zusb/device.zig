const c = @import("c.zig");
const ConfigDescriptor = @import("config_descriptor.zig").ConfigDescriptor;
const fromLibusb = @import("constructor.zig").fromLibusb;

usingnamespace @import("error.zig");

pub const Device = struct {
    ctx: *c.libusb_context,
    device: *c.libusb_device,

    pub fn deinit(self: Device) void {
        _ = c.libusb_unref_device(self.device);
    }

    pub fn deviceDescriptor(self: Device) Error!DeviceDescriptor {
        var descriptor: c.libusb_device_descriptor = undefined;

        try failable(c.libusb_get_device_descriptor(
            self.device,
            &descriptor,
        ));

        return DeviceDescriptor{ .descriptor = descriptor };
    }

    pub fn configDescriptor(self: Device, config_index: u8) Error!ConfigDescriptor {
        var descriptor: ?*c.libusb_config_descriptor = null;

        try failable(c.libusb_get_config_descriptor(
            self.device,
            config_index,
            &descriptor,
        ));

        return ConfigDescriptor{ .descriptor = descriptor.? };
    }

    pub fn busNumber(self: Device) u8 {
        return c.libusb_get_bus_number(self.device.device);
    }

    pub fn address(self: Device) u8 {
        return c.libusb_get_device_address(self.device.device);
    }

    pub fn open(self: Device) Error!DeviceHandle {
        var handle: ?*c.libusb_device_handle = null;
        try failable(c.libusb_open(self.device, &handle));

        return fromLibusb(DeviceHandle, .{ self.ctx, handle.? });
    }
};
