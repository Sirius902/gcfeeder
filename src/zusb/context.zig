const c = @import("c.zig");
const DeviceHandle = @import("device_handle.zig").DeviceHandle;
const DeviceList = @import("device_list.zig").DeviceList;
const fromLibusb = @import("constructor.zig").fromLibusb;

usingnamespace @import("error.zig");

pub const Context = struct {
    raw: *c.libusb_context,

    pub fn init() Error!Context {
        var ctx: ?*c.libusb_context = null;
        try failable(c.libusb_init(&ctx));

        return Context{ .raw = ctx.? };
    }

    pub fn deinit(self: Context) void {
        _ = c.libusb_exit(self.raw);
    }

    pub fn devices(self: *Context) Error!DeviceList {
        return DeviceList.init(self);
    }

    pub fn openDeviceWithVidPid(
        self: *Context,
        vendor_id: u16,
        product_id: u16,
    ) Error!?DeviceHandle {
        if (c.libusb_open_device_with_vid_pid(self.raw, vendor_id, product_id)) |handle| {
            return fromLibusb(DeviceHandle, .{ self, handle });
        } else {
            return null;
        }
    }
};
