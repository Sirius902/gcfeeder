pub usingnamespace @import("config_descriptor.zig");
pub usingnamespace @import("context.zig");
pub usingnamespace @import("device_descriptor.zig");
pub usingnamespace @import("device_handle.zig");
pub usingnamespace @import("device_list.zig");
pub usingnamespace @import("device.zig");
pub usingnamespace @import("endpoint_descriptor.zig");
pub usingnamespace @import("error.zig");
pub usingnamespace @import("fields.zig");
pub usingnamespace @import("interface_descriptor.zig");
pub usingnamespace @import("transfer.zig");

pub const dt_hid = c.LIBUSB_DT_HID;
