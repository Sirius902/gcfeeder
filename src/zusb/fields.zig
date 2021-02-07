pub const Direction = enum {
    In,
    Out,
};

pub const TransferType = enum {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
};
