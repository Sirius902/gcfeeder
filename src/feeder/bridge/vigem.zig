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

pub const Device = struct {
    client: c.PVIGEM_CLIENT,
    pad: c.PVIGEM_TARGET,

    pub const Error = error{
        ViGEmFail,
        OutOfMemory,
    };

    pub fn init() !Device {
        const client = c.vigem_alloc() orelse return error.OutOfMemory;
        const code = c.vigem_connect(client);

        // TODO: Return zig error based on code.
        if (!c.VIGEM_SUCCESS(code)) {
            return error.ViGEmFail;
        }

        const pad = c.vigem_target_ds4_alloc() orelse return error.OutOfMemory;
        const pir = c.vigem_target_add(client, pad);

        if (!c.VIGEM_SUCCESS(pir)) {
            return error.ViGEmFail;
        }

        return Device{ .client = client, .pad = pad };
    }

    pub fn deinit(self: Device) void {
        _ = c.vigem_target_remove(self.client, self.pad);
        c.vigem_target_free(self.pad);

        c.vigem_disconnect(self.client);
        c.vigem_free(self.client);
    }

    pub fn update(self: Device, report: DS4Report) Error!void {
        const res = c.vigem_target_ds4_update(self.client, self.pad, report);

        if (!c.VIGEM_SUCCESS(res)) {
            return error.ViGEmFail;
        }
    }
};
