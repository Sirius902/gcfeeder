const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
    @cInclude("ViGEm/Client.h");
});

pub const Driver = struct {
    client: c.PVIGEM_CLIENT,

    pub fn init() !Driver {
        const client = c.vigem_alloc() orelse return error.OutOfMemory;
        const code = c.vigem_connect(client);

        if (!c.VIGEM_SUCCESS(code)) {
            return error.VigemFailed;
        }

        return Driver{ .client = client };
    }

    pub fn deinit(self: Driver) void {
        c.vigem_disconnect(self.client);
        c.vigem_free(self.client);
    }
};
