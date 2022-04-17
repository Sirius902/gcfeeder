const std = @import("std");
const vjoy = @import("vjoy.zig");
const vigem = @import("vigem.zig");
const Allocator = std.mem.Allocator;
const Adapter = @import("../adapter.zig").Adapter;
const Input = @import("../adapter.zig").Input;
const Rumble = @import("../adapter.zig").Rumble;
const stick_range = @import("../adapter.zig").Calibration.stick_range;
const trigger_range = @import("../adapter.zig").Calibration.trigger_range;

const windows_max = 32767;

pub const Error = vjoy.Device.Error || vigem.Device.Error || Allocator.Error;

pub const VJoyBridge = struct {
    allocator: Allocator,
    device: vjoy.Device,
    receiver: *vjoy.FFBReceiver,
    last_timestamp: ?i64 = null,

    pub fn init(allocator: Allocator) Error!*VJoyBridge {
        const device = try vjoy.Device.init(1);
        const receiver = try vjoy.FFBReceiver.init(allocator);
        errdefer receiver.deinit();

        var self = try allocator.create(VJoyBridge);
        self.* = VJoyBridge{
            .allocator = allocator,
            .device = device,
            .receiver = receiver,
        };
        return self;
    }

    pub fn initBridge(allocator: Allocator) Error!Bridge {
        const self = try VJoyBridge.init(allocator);
        return self.bridge();
    }

    pub fn deinit(self: *VJoyBridge) void {
        self.device.deinit();
        self.receiver.deinit();
        self.allocator.destroy(self);
    }

    pub fn feed(self: *VJoyBridge, input: Input) Error!void {
        try self.device.update(toVJoy(input));
    }

    pub fn pollRumble(self: *VJoyBridge) ?Rumble {
        var rumble: ?Rumble = null;

        if (self.receiver.get()) |packet| {
            if (packet.device_id == 1) {
                rumble = switch (packet.effect.operation) {
                    .Stop => .Off,
                    else => .On,
                };

                if (self.last_timestamp) |last| {
                    if (packet.timestamp_ms - last < 2) {
                        rumble = .Off;
                    }
                }

                self.last_timestamp = packet.timestamp_ms;
            }
        }

        return rumble;
    }

    pub fn driverName() []const u8 {
        return "vJoy";
    }

    pub fn bridge(self: *VJoyBridge) Bridge {
        return Bridge.init(self, deinit, feed, pollRumble, driverName);
    }

    fn toVJoy(input: Input) vjoy.JoystickPosition {
        var pos = std.mem.zeroes(vjoy.JoystickPosition);

        var buttons = std.PackedIntArray(u1, 12).init([_]u1{
            @boolToInt(input.button_a),
            @boolToInt(input.button_b),
            @boolToInt(input.button_x),
            @boolToInt(input.button_y),
            @boolToInt(input.button_z),
            @boolToInt(input.button_r),
            @boolToInt(input.button_l),
            @boolToInt(input.button_start),
            @boolToInt(input.button_up),
            @boolToInt(input.button_down),
            @boolToInt(input.button_left),
            @boolToInt(input.button_right),
        });

        pos.lButtons = buttons.sliceCast(u12).get(0);

        pos.wAxisX = @floatToInt(c_long, std.math.ceil(stick_range.normalize(input.stick_x) * windows_max));
        pos.wAxisY = @floatToInt(c_long, std.math.ceil((1.0 - stick_range.normalize(input.stick_y)) * windows_max));

        pos.wAxisXRot = @floatToInt(c_long, std.math.ceil(stick_range.normalize(input.substick_x) * windows_max));
        pos.wAxisYRot = @floatToInt(c_long, std.math.ceil((1.0 - stick_range.normalize(input.substick_y)) * windows_max));

        pos.wAxisZ = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_left) * windows_max));
        pos.wAxisZRot = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_right) * windows_max));

        return pos;
    }
};

pub const ViGEmBridge = struct {
    allocator: Allocator,
    device: *vigem.Device,
    listener: *vigem.Listener,
    config: Config,

    pub const Config = struct {
        pad: vigem.Pad,
        trigger_mode: TriggerMode = .stick_click,
    };

    pub const TriggerMode = enum {
        analog,
        digital,
        combination,
        stick_click,

        pub fn jsonStringify(
            value: TriggerMode,
            options: std.json.StringifyOptions,
            out_stream: anytype,
        ) @TypeOf(out_stream).Error!void {
            _ = options;
            try out_stream.writeByte('"');
            try out_stream.writeAll(std.meta.fieldNames(TriggerMode)[@enumToInt(value)]);
            try out_stream.writeByte('"');
        }
    };

    pub fn init(allocator: Allocator, config: Config) Error!*ViGEmBridge {
        var device = try allocator.create(vigem.Device);
        errdefer allocator.destroy(device);

        device.* = try vigem.Device.init(config.pad);

        const listener = try vigem.Listener.init(allocator, device);
        errdefer listener.deinit();

        var self = try allocator.create(ViGEmBridge);
        self.* = ViGEmBridge{
            .allocator = allocator,
            .device = device,
            .listener = listener,
            .config = config,
        };
        return self;
    }

    pub fn initBridge(allocator: Allocator, config: Config) Error!Bridge {
        const self = try ViGEmBridge.init(allocator, config);
        return self.bridge();
    }

    pub fn deinit(self: *ViGEmBridge) void {
        self.listener.deinit();
        self.device.deinit();
        self.allocator.destroy(self.device);
        self.allocator.destroy(self);
    }

    pub fn feed(self: *ViGEmBridge, input: Input) Error!void {
        try self.device.update(switch (self.config.pad) {
            .x360 => &self.toX360(input),
            .ds4 => &self.toDS4(input),
        });
    }

    pub fn pollRumble(self: *ViGEmBridge) ?Rumble {
        return self.listener.get();
    }

    pub fn driverName() []const u8 {
        return "ViGEm";
    }

    pub fn bridge(self: *ViGEmBridge) Bridge {
        return Bridge.init(self, deinit, feed, pollRumble, driverName);
    }

    fn toX360(self: ViGEmBridge, input: Input) vigem.XUSBReport {
        var pos = std.mem.zeroes(vigem.XUSBReport);

        const trigger_res = self.applyTriggerMode(input);
        var buttons = std.PackedIntArray(u1, 16).init([_]u1{
            @boolToInt(input.button_up),
            @boolToInt(input.button_down),
            @boolToInt(input.button_left),
            @boolToInt(input.button_right),
            @boolToInt(input.button_start),
            0, // back
            trigger_res.ls, // left thumb
            trigger_res.rs, // right thumb
            0, // left shoulder
            @boolToInt(input.button_z),
            0,
            0,
            @boolToInt(input.button_a),
            @boolToInt(input.button_b),
            @boolToInt(input.button_x),
            @boolToInt(input.button_y),
        });

        pos.wButtons = buttons.sliceCast(u16).get(0);

        pos.sThumbLX = @floatToInt(c_short, std.math.ceil((2.0 * stick_range.normalize(input.stick_x) - 1.0) * windows_max));
        pos.sThumbLY = @floatToInt(c_short, std.math.ceil((2.0 * stick_range.normalize(input.stick_y) - 1.0) * windows_max));

        pos.sThumbRX = @floatToInt(c_short, std.math.ceil((2.0 * stick_range.normalize(input.substick_x) - 1.0) * windows_max));
        pos.sThumbRY = @floatToInt(c_short, std.math.ceil((2.0 * stick_range.normalize(input.substick_y) - 1.0) * windows_max));

        pos.bLeftTrigger = trigger_res.l;
        pos.bRightTrigger = trigger_res.r;

        return pos;
    }

    fn toDS4(self: ViGEmBridge, input: Input) vigem.DS4Report {
        var pos = std.mem.zeroes(vigem.DS4Report);

        const trigger_res = self.applyTriggerMode(input);
        const hat_bits = ds4HatBits(input);
        var buttons = std.PackedIntArray(u1, 16).init([_]u1{
            @truncate(u1, hat_bits >> 0),
            @truncate(u1, hat_bits >> 1),
            @truncate(u1, hat_bits >> 2),
            @truncate(u1, hat_bits >> 3),
            @boolToInt(input.button_x),
            @boolToInt(input.button_a),
            @boolToInt(input.button_b),
            @boolToInt(input.button_y),
            0,
            @boolToInt(input.button_z),
            @boolToInt(input.button_l),
            @boolToInt(input.button_r),
            0,
            @boolToInt(input.button_start),
            trigger_res.ls,
            trigger_res.rs,
        });

        pos.wButtons = buttons.sliceCast(u16).get(0);

        pos.bThumbLX = input.stick_x;
        pos.bThumbLY = ~input.stick_y;

        pos.bThumbRX = input.substick_x;
        pos.bThumbRY = ~input.substick_y;

        pos.bTriggerL = trigger_res.l;
        pos.bTriggerR = trigger_res.r;

        return pos;
    }

    fn ds4HatBits(input: Input) u4 {
        const up: u4 = @boolToInt(input.button_up);
        const left: u4 = @boolToInt(input.button_left);
        const right: u4 = @boolToInt(input.button_right);
        const down: u4 = @boolToInt(input.button_down);
        const encoded = (up << 3) | (left << 2) | (right << 1) | down;

        return switch (encoded) {
            0b0000, 0b1111, 0b1001, 0b0110 => 0b1000,
            0b1000, 0b1110 => 0,
            0b1010 => 1,
            0b0010, 0b1011 => 2,
            0b0011 => 3,
            0b0001, 0b0111 => 4,
            0b0101 => 5,
            0b0100, 0b1101 => 6,
            0b1100 => 7,
        };
    }

    fn applyTriggerMode(self: ViGEmBridge, input: Input) struct { l: u8, r: u8, ls: u1, rs: u1 } {
        var l: u8 = undefined;
        var r: u8 = undefined;
        var ls: u1 = 0;
        var rs: u1 = 0;

        switch (self.config.trigger_mode) {
            .analog => {
                l = input.trigger_left;
                r = input.trigger_right;
            },
            .digital => {
                l = if (input.button_l) 255 else 0;
                r = if (input.button_r) 255 else 0;
            },
            .combination => {
                l = if (input.button_l) 255 else input.trigger_left;
                r = if (input.button_r) 255 else input.trigger_right;
            },
            .stick_click => {
                l = input.trigger_left;
                r = input.trigger_right;
                ls = @boolToInt(input.button_l);
                rs = @boolToInt(input.button_r);
            },
        }

        return .{ .l = l, .r = r, .ls = ls, .rs = rs };
    }
};

pub const Bridge = struct {
    ptr: *anyopaque,
    vtable: *const VTable,

    const VTable = struct {
        deinit: fn (ptr: *anyopaque) void,
        feed: fn (ptr: *anyopaque, input: Input) Error!void,
        pollRumble: fn (ptr: *anyopaque) ?Rumble,
        driverName: fn () []const u8,
    };

    pub fn init(
        pointer: anytype,
        comptime deinitFn: fn (ptr: @TypeOf(pointer)) void,
        comptime feedFn: fn (ptr: @TypeOf(pointer), input: Input) Error!void,
        comptime pollRumbleFn: fn (ptr: @TypeOf(pointer)) ?Rumble,
        comptime driverNameFn: fn () []const u8,
    ) Bridge {
        const Ptr = @TypeOf(pointer);
        const ptr_info = @typeInfo(Ptr);

        std.debug.assert(ptr_info == .Pointer); // Must be a pointer
        std.debug.assert(ptr_info.Pointer.size == .One); // Must be a single-item pointer

        const alignment = ptr_info.Pointer.alignment;

        const gen = struct {
            fn deinitImpl(ptr: *anyopaque) void {
                const self = @ptrCast(Ptr, @alignCast(alignment, ptr));
                @call(.{ .modifier = .always_inline }, deinitFn, .{self});
            }
            fn feedImpl(ptr: *anyopaque, input: Input) Error!void {
                const self = @ptrCast(Ptr, @alignCast(alignment, ptr));
                return @call(.{ .modifier = .always_inline }, feedFn, .{ self, input });
            }
            fn pollRumbleImpl(ptr: *anyopaque) ?Rumble {
                const self = @ptrCast(Ptr, @alignCast(alignment, ptr));
                return @call(.{ .modifier = .always_inline }, pollRumbleFn, .{self});
            }

            const vtable = VTable{
                .deinit = deinitImpl,
                .feed = feedImpl,
                .pollRumble = pollRumbleImpl,
                .driverName = driverNameFn,
            };
        };

        return Bridge{
            .ptr = pointer,
            .vtable = &gen.vtable,
        };
    }

    pub fn deinit(self: Bridge) void {
        self.vtable.deinit(self.ptr);
    }

    pub fn feed(self: Bridge, input: Input) Error!void {
        try self.vtable.feed(self.ptr, input);
    }

    pub fn pollRumble(self: Bridge) ?Rumble {
        return self.vtable.pollRumble(self.ptr);
    }

    pub fn driverName(self: Bridge) []const u8 {
        return self.vtable.driverName();
    }
};
