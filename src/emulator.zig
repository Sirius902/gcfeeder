const std = @import("std");
const grindel = @import("grindel");
const Rumble = @import("adapter.zig").Rumble;
const Address = grindel.Address;
const Process = grindel.Process;

pub const Handle = struct {
    process: Process,
    emulator: Emulator,

    pub const Error = Process.Error;

    const Emulator = enum {
        BizHawk,
        ModLoader,
    };

    const MupenAddresses = struct {
        game_id: Address,
        oot_rumble: Address,
    };

    const game_ids = [_][]const u8{ "CZLE", "CZLJ" };

    const bizhawk = MupenAddresses{
        .game_id = Address.comptimeParse(
            \\ "mupen64plus.dll"+106EB5B
        ) catch unreachable,
        .oot_rumble = Address.comptimeParse(
            \\ "mupen64plus.dll"+6C373+11CF10
        ) catch unreachable,
    };

    const modloader = MupenAddresses{
        .game_id = Address.comptimeParse(
            \\ "mupen64plus.dll"+3E44F9B
        ) catch unreachable,
        // Add 3 because ModLoader stores in native endian.
        .oot_rumble = Address.comptimeParse(
            \\ ["mupen64plus.dll"+573E4]+11CF10+3
        ) catch unreachable,
    };

    pub fn open() Error!Handle {
        const self = blk: {
            if (Process.attach("EmuHawk.exe")) |process| {
                break :blk Handle{ .process = process, .emulator = .BizHawk };
            } else |_| {
                if (Process.attachWindow("ModLoader64")) |process| {
                    break :blk Handle{ .process = process, .emulator = .ModLoader };
                } else |_| {
                    return Error.ProcessNotFound;
                }
            }
        };

        return if (try self.checkOoT()) self else return Error.ProcessNotFound;
    }

    pub fn close(self: Handle) void {
        self.process.detach();
    }

    pub fn rumbleState(self: Handle) Error!Rumble {
        const address = try self.addresses().oot_rumble.resolve(&self.process);
        return if ((try self.process.read(u8, address)) != 0) Rumble.On else Rumble.Off;
    }

    pub fn emulatorTitle(self: Handle) []const u8 {
        return switch (self.emulator) {
            .BizHawk => "BizHawk",
            .ModLoader => "ModLoader",
        };
    }

    fn checkOoT(self: Handle) Error!bool {
        var current_id: [4]u8 = undefined;
        const address = try self.addresses().game_id.resolve(&self.process);
        try self.process.readIntoSlice(&current_id, address);

        var valid_id = false;
        for (game_ids) |id| {
            if (std.mem.eql(u8, &current_id, id)) {
                valid_id = true;
            }
        }

        return valid_id;
    }

    fn addresses(self: Handle) *const MupenAddresses {
        return switch (self.emulator) {
            .BizHawk => &bizhawk,
            .ModLoader => &modloader,
        };
    }
};
