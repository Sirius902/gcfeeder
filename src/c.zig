pub usingnamespace @cImport({
    // Use a bunch of defines to gut `windows.h` and prevent a conflict with raylib.
    @cDefine("NOGDICAPMASKS", "");
    @cDefine("NOVIRTUALKEYCODES", "");
    @cDefine("NOWINMESSAGES", "");
    @cDefine("NOWINSTYLES", "");
    @cDefine("NOSYSMETRICS", "");
    @cDefine("NOMENUS", "");
    @cDefine("NOICONS", "");
    @cDefine("NOKEYSTATES", "");
    @cDefine("NOSYSCOMMANDS", "");
    @cDefine("NORASTEROPS", "");
    @cDefine("NOSHOWWINDOW", "");
    @cDefine("OEMRESOURCE", "");
    @cDefine("NOATOM", "");
    @cDefine("NOCLIPBOARD", "");
    @cDefine("NOCOLOR", "");
    @cDefine("NOCTLMGR", "");
    @cDefine("NODRAWTEXT", "");
    @cDefine("NOGDI", "");
    @cDefine("NOKERNEL", "");
    @cDefine("NOUSER", "");
    @cDefine("NOMB", "");
    @cDefine("NOMEMMGR", "");
    @cDefine("NOMETAFILE", "");
    @cDefine("NOMINMAX", "");
    @cDefine("NOMSG", "");
    @cDefine("NOOPENFILE", "");
    @cDefine("NOSCROLL", "");
    @cDefine("NOSERVICE", "");
    @cDefine("NOSOUND", "");
    @cDefine("NOTEXTMETRIC", "");
    @cDefine("NOWH", "");
    @cDefine("NOWINOFFSETS", "");
    @cDefine("NOCOMM", "");
    @cDefine("NOKANJI", "");
    @cDefine("NOHELP", "");
    @cDefine("NOPROFILER", "");
    @cDefine("NODEFERWINDOWPOS", "");
    @cDefine("NOMCX", "");
    @cInclude("raylib.h");
    @cInclude("libusb-1.0/libusb.h");
    @cInclude("vjoyinterface.h");
});
