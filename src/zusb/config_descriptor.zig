const c = @import("c.zig");
const std = @import("std");
const Interface = @import("interface_descriptor.zig").Interface;

pub const ConfigDescriptor = struct {
    descriptor: *c.libusb_config_descriptor,

    pub fn deinit(self: ConfigDescriptor) void {
        _ = c.libusb_free_config_descriptor(self.descriptor);
    }

    pub fn interfaces(self: ConfigDescriptor) Interfaces {
        return Interfaces{
            .interfaces = self.descriptor.*.interface[0..self.descriptor.*.bNumInterfaces],
            .i = 0,
        };
    }
};

pub const Interfaces = struct {
    interfaces: []const c.libusb_interface,
    i: usize,

    pub fn next(self: *Interfaces) ?Interface {
        if (self.i < self.interfaces.len) {
            defer self.i += 1;

            const len = std.math.cast(
                usize,
                self.interfaces[self.i].num_altsetting,
            ) catch unreachable;

            return Interface{
                .iter = self.interfaces[self.i].altsetting[0..len],
            };
        } else {
            return null;
        }
    }
};
