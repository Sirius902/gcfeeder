const c = @import("c.zig");

pub const DeviceDescriptor = struct {
    descriptor: c.libusb_device_descriptor,

    pub fn classCode(self: DeviceDescriptor) u8 {
        return self.descriptor.bDeviceClass;
    }

    pub fn subClassCode(self: DeviceDescriptor) u8 {
        return self.descriptor.bDeviceSubClass;
    }

    pub fn vendorId(self: DeviceDescriptor) u16 {
        return self.descriptor.idVendor;
    }

    pub fn productId(self: DeviceDescriptor) u16 {
        return self.descriptor.idProduct;
    }
};
