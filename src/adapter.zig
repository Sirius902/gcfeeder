const usb = @import("usb.zig");

pub const Adapter = struct {
    const gc_vid = 0x057E;
    const gc_pid = 0x0337;

    const Endpoints = struct {
        in: u8,
        out: u8,
    };

    handle: usb.DeviceHandle,
    endpoints: Endpoints,

    pub fn init(ctx: *usb.Context) usb.Error!Adapter {
        var handle = try ctx.openDeviceWithVidPid(gc_vid, gc_pid);

        try handle.claimInterface(0);

        return Adapter{
            .handle = handle,
            .endpoints = try findEndpoints(handle),
        };
    }

    pub fn deinit(self: Adapter) void {
        self.handle.deinit();
    }

    fn findEndpoints(handle: usb.DeviceHandle) usb.Error!Endpoints {
        const device = handle.device();
        defer device.deinit();

        const config = try device.configDescriptor(0);
        defer config.deinit();

        var in: u8 = 0;
        var out: u8 = 0;

        var interfaces = config.interfaces();
        while (interfaces.next()) |iface| {
            var descriptors = iface.descriptors();
            while (descriptors.next()) |descriptor| {
                var endpoints = descriptor.endpointDescriptors();
                while (endpoints.next()) |endpoint| {
                    switch (endpoint.direction()) {
                        usb.Direction.In => {
                            in = endpoint.address();
                        },
                        usb.Direction.Out => {
                            out = endpoint.address();
                        },
                    }
                }
            }
        }

        return Endpoints{ .in = in, .out = out };
    }
};
