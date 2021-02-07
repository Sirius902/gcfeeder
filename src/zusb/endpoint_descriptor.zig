const c = @import("c.zig");
const Direction = @import("fields.zig").Direction;

pub const EndpointDescriptor = struct {
    descriptor: *const c.libusb_endpoint_descriptor,

    pub fn direction(self: EndpointDescriptor) Direction {
        return switch (self.descriptor.*.bEndpointAddress & c.LIBUSB_ENDPOINT_DIR_MASK) {
            c.LIBUSB_ENDPOINT_OUT => Direction.Out,
            c.LIBUSB_ENDPOINT_IN => Direction.In,
            else => Direction.In,
        };
    }

    pub fn transferType(self: EndpointDescriptor) TransferType {
        return switch (self.descriptor.*.bmAttributes & c.LIBUSB_TRANSFER_TYPE_MASK) {
            c.LIBUSB_TRANSFER_TYPE_CONTROL => TransferType.Control,
            c.LIBUSB_TRANSFER_TYPE_ISOCHRONOUS => TransferType.Isochronous,
            c.LIBUSB_TRANSFER_TYPE_BULK => TransferType.Bulk,
            c.LIBUSB_TRANSFER_TYPE_INTERRUPT => TransferType.Interrupt,
            else => TransferType.Interrupt,
        };
    }

    pub fn number(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress & 0x07;
    }

    pub fn address(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress;
    }

    pub fn interval(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bInterval;
    }
};
