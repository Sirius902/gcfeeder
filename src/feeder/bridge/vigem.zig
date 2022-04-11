const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
    @cInclude("ViGEm/Client.h");
});

const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;
const LinearFifo = std.fifo.LinearFifo;
const Rumble = @import("../adapter.zig").Rumble;

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

pub const Listener = struct {
    allocator: Allocator,
    mutex: Mutex,
    queue: Fifo,
    device: *const Device,

    const Fifo = LinearFifo(Rumble, .{ .Static = 16 });

    pub fn init(allocator: Allocator, device: *const Device) Allocator.Error!*Listener {
        var self = try allocator.create(Listener);
        self.* = Listener{
            .allocator = allocator,
            .mutex = Mutex{},
            .queue = Fifo.init(),
            .device = device,
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

        return self.queue.readItem();
    }

    fn put(self: *Listener, packet: Rumble) void {
        self.mutex.lock();
        defer self.mutex.unlock();

        self.queue.ensureUnusedCapacity(1) catch {
            self.queue.discard(1);
        };

        self.queue.writeItemAssumeCapacity(packet);
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
        self.put(if (large_motor > 0 or small_motor > 0) .On else .Off);
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
        self.put(if (large_motor > 0 or small_motor > 0) .On else .Off);
    }
};
