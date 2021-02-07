const c = @import("c.zig");
const EndpointDescriptor = @import("endpoint_descriptor.zig").EndpointDescriptor;

pub const Interface = struct {
    iter: []const c.libusb_interface_descriptor,

    pub fn number(self: Interface) u8 {
        return self.iter[0].bInterfaceNumber;
    }

    pub fn descriptors(self: Interface) InterfaceDescriptors {
        return InterfaceDescriptors{
            .iter = self.iter,
            .i = 0,
        };
    }
};

pub const InterfaceDescriptor = struct {
    descriptor: *const c.libusb_interface_descriptor,

    pub fn endpointDescriptors(self: InterfaceDescriptor) EndpointDescriptors {
        return EndpointDescriptors{
            .iter = self.descriptor.*.endpoint[0..self.descriptor.*.bNumEndpoints],
            .i = 0,
        };
    }
};

pub const EndpointDescriptors = struct {
    iter: []const c.libusb_endpoint_descriptor,
    i: usize,

    pub fn next(self: *EndpointDescriptors) ?EndpointDescriptor {
        if (self.i < self.iter.len) {
            defer self.i += 1;
            return EndpointDescriptor{ .descriptor = &self.iter[self.i] };
        } else {
            return null;
        }
    }
};

pub const InterfaceDescriptors = struct {
    iter: []const c.libusb_interface_descriptor,
    i: usize,

    pub fn next(self: *InterfaceDescriptors) ?InterfaceDescriptor {
        if (self.i < self.iter.len) {
            defer self.i += 1;
            return InterfaceDescriptor{ .descriptor = &self.iter[self.i] };
        } else {
            return null;
        }
    }
};
