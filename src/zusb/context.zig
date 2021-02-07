const c = @import("c.zig");
const DeviceHandle = @import("device_handle.zig").DeviceHandle;
const fromLibusb = @import("constructor.zig").fromLibusb;

usingnamespace @import("error.zig");

pub const Context = struct {
    ctx: *c.libusb_context,

    pub fn init() Error!Context {
        var ctx: ?*c.libusb_context = null;
        try failable(c.libusb_init(&ctx));

        return Context{ .ctx = ctx.? };
    }

    pub fn deinit(self: Context) void {
        _ = c.libusb_exit(self.ctx);
    }

    pub fn openDeviceWithVidPid(
        self: Context,
        vendor_id: u16,
        product_id: u16,
    ) Error!?DeviceHandle {
        if (c.libusb_open_device_with_vid_pid(self.ctx, vendor_id, product_id)) |handle| {
            return fromLibusb(DeviceHandle, .{ self.ctx, handle });
        } else {
            return null;
        }
    }
};
