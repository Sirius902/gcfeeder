const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
    @cDefine("VIGEM_USE_CRASH_HANDLER", {});
    @cInclude("ViGEm/Client.h");
});

const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;
const LinearFifo = std.fifo.LinearFifo;
const Rumble = @import("../adapter.zig").Rumble;
const max = std.math.max;
const maxInt = std.math.maxInt;

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

    pub fn jsonStringify(
        value: Pad,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        _ = options;
        try out_stream.writeByte('"');
        try out_stream.writeAll(std.meta.tagName(value));
        try out_stream.writeByte('"');
    }
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

pub const Listener = struct {
    allocator: Allocator,
    mutex: Mutex,
    device: *const Device,
    rumble_state: struct {
        index: usize = 0,
        poll_count: u3 = 0,
    },

    const Pattern = u6;
    const rumble_patterns = &[_]Pattern{
        0b000000,
        0b100000,
        0b100100,
        0b101010,
        0b110110,
        0b111110,
        0b111111,
    };

    pub fn init(allocator: Allocator, device: *const Device) Allocator.Error!*Listener {
        var self = try allocator.create(Listener);
        self.* = Listener{
            .allocator = allocator,
            .mutex = Mutex{},
            .device = device,
            .rumble_state = .{},
        };

        _ = switch (device.pad_type) {
            .x360 => c.vigem_target_x360_register_notification(device.client, device.pad, x360Callback, self),
            .ds4 => c.vigem_target_ds4_register_notification(device.client, device.pad, ds4Callback, self),
        };

        return self;
    }

    pub fn deinit(self: *Listener) void {
        switch (self.device.pad_type) {
            .x360 => c.vigem_target_x360_unregister_notification(self.device.pad),
            .ds4 => c.vigem_target_ds4_unregister_notification(self.device.pad),
        }

        self.allocator.destroy(self);
    }

    pub fn get(self: *Listener) ?Rumble {
        self.mutex.lock();
        defer self.mutex.unlock();

        const state = &self.rumble_state;
        defer state.poll_count = (state.poll_count + 1) % @bitSizeOf(Pattern);
        if ((rumble_patterns[state.index] & (@as(u8, 1) << state.poll_count)) != 0) {
            return .On;
        } else {
            return .Off;
        }
    }

    fn handlePacket(self: *Listener, large_motor: u8, small_motor: u8) void {
        const strength = @as(usize, max(large_motor, small_motor));
        var rumble_index = if (strength > 0)
            1 + ((strength - 1) * (rumble_patterns.len - 1)) / maxInt(u8)
        else
            0;

        // Defend against vigem passing values larger than 255 for motors.
        if (rumble_index >= rumble_patterns.len) {
            rumble_index = 0;
        }

        self.mutex.lock();
        defer self.mutex.unlock();
        self.rumble_state.index = rumble_index;
        self.rumble_state.poll_count = 0;
    }

    export fn x360Callback(
        client: c.PVIGEM_CLIENT,
        target: c.PVIGEM_TARGET,
        large_motor: c.UCHAR,
        small_motor: c.UCHAR,
        led_number: c.UCHAR,
        user_data: ?*anyopaque,
    ) void {
        _ = client;
        _ = target;
        _ = led_number;
        const self = @intToPtr(*Listener, @ptrToInt(user_data.?));
        self.handlePacket(large_motor, small_motor);
    }

    export fn ds4Callback(
        client: c.PVIGEM_CLIENT,
        target: c.PVIGEM_TARGET,
        large_motor: c.UCHAR,
        small_motor: c.UCHAR,
        lightbar_color: c.DS4_LIGHTBAR_COLOR,
        user_data: ?*anyopaque,
    ) void {
        _ = client;
        _ = target;
        _ = lightbar_color;
        const self = @intToPtr(*Listener, @ptrToInt(user_data.?));
        self.handlePacket(large_motor, small_motor);
    }
};
