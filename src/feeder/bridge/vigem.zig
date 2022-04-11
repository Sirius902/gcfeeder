const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
    @cInclude("ViGEm/Client.h");
});

pub const XUSBReport = c.XUSB_REPORT;

// b0 - square
// b1 - cross
// b2 - circle
// b3 - triangle
// b4 - LB
// b5 - RB
// b6 - LT
// b7 - RT
// b8 - share
// b9 - option
// b10 - LS
// b11 - RS
// b12 - PS button
// b13 - trackpad
pub const DS4Report = c.DS4_REPORT;

pub const Pad = enum {
    x360,
    ds4,
};

pub const Device = struct {
    client: c.PVIGEM_CLIENT,
    pad: c.PVIGEM_TARGET,
    pad_type: Pad,

    // TODO: Return zig error based on code.
    pub const Error = error{
        ViGEmFail,
        OutOfMemory,
    };

    pub fn init(pad_type: Pad) !Device {
        const client = c.vigem_alloc() orelse return error.OutOfMemory;
        const code = c.vigem_connect(client);

        if (!c.VIGEM_SUCCESS(code)) {
            return error.ViGEmFail;
        }

        const alloc_fn = switch (pad_type) {
            .x360 => c.vigem_target_x360_alloc,
            .ds4 => c.vigem_target_ds4_alloc,
        };
        const pad = alloc_fn() orelse return error.OutOfMemory;

        const pir = c.vigem_target_add(client, pad);

        if (!c.VIGEM_SUCCESS(pir)) {
            return error.ViGEmFail;
        }

        return Device{ .client = client, .pad = pad, .pad_type = pad_type };
    }

    pub fn deinit(self: Device) void {
        _ = c.vigem_target_remove(self.client, self.pad);
        c.vigem_target_free(self.pad);

        c.vigem_disconnect(self.client);
        c.vigem_free(self.client);
    }

    pub fn update(self: Device, report: *align(2) const anyopaque) Error!void {
        // TODO: Union enum?
        const res = switch (self.pad_type) {
            .x360 => c.vigem_target_x360_update(
                self.client,
                self.pad,
                @ptrCast(*const XUSBReport, report).*,
            ),
            .ds4 => c.vigem_target_ds4_update(
                self.client,
                self.pad,
                @ptrCast(*const DS4Report, report).*,
            ),
        };

        if (!c.VIGEM_SUCCESS(res)) {
            return error.ViGEmFail;
        }
    }
};
