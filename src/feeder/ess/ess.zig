const Input = @import("../adapter.zig").Input;

const c = @cImport({
    @cInclude("ESS.h");
});

pub fn map(input: Input) Input {
    var coords = [_]u8{ input.stick_x, input.stick_y };
    c.invert_vc_gc(&coords);

    var mapped = input;
    mapped.stick_x = coords[0];
    mapped.stick_y = coords[1];
    return mapped;
}
