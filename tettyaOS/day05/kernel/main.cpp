/**
 * @file main.cpp
 *
 * カーネル本体のプログラムを書いたファイル．
 */

#include <cstdint>
#include <cstddef>
#include <cstdio>

#include "frame_buffer_config.hpp"
#include "graphics.hpp"
#include "font.hpp"
#include "console.hpp"

void *operator new(size_t size, void *buf)
{
    return buf;
}

void operator delete(void *obj) noexcept
{
}

char pixel_writer_buf[sizeof(RGBResv8BitPerColorPixelWriter)];
PixelWriter *pixel_writer;

char console_buf[sizeof(Console)];
Console *console;
int printk(const char *format, ...)
{
    va_list ap;
    int result;
    char s[1024];

    va_start(ap, format);
    result = vsprintf(s, format, ap);
    va_end(ap);

    console->PutString(s);
    return result;
}

extern "C" void KernelMain(const FrameBufferConfig &frame_buffer_config)
{
    switch (frame_buffer_config.pixel_format)
    {
    case kPixelRGBResv8BitPerColor:
        pixel_writer = new (pixel_writer_buf)
            RGBResv8BitPerColorPixelWriter{frame_buffer_config};
        break;
    case kPixelBGRResv8BitPerColor:
        pixel_writer = new (pixel_writer_buf)
            BGRResv8BitPerColorPixelWriter{frame_buffer_config};
        break;
    }

    for (int x = 0; x < frame_buffer_config.horizontal_resolution; ++x)
    {
        for (int y = 0; y < frame_buffer_config.vertical_resolution; ++y)
        {
            pixel_writer->Write(x, y, {255, 255, 255});
        }
    }
    for (int x = 0; x < 200; ++x)
    {
        for (int y = 0; y < 100; ++y)
        {
            pixel_writer->Write(x, y, {0, 255, 0});
        }
    }
    /*
        char tmpstr[] = "abcdefghijklmnopqrstuvwxyz0123456789!#$%&()=~|-^\\\"\'@[]{}:;+:,./?><ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        for (unsigned char idx = 0; idx < 96; idx++)
        {
            WriteAscii(*pixel_writer, 50 + idx * 8, 50, tmpstr[idx], {250, 100, 250});
        }

        char buf[128];
        sprintf(buf, "1 + 2= %d", 1 + 2);
        WriteString(*pixel_writer, 0, 82, buf, {0, 0, 0});
        Console con(*pixel_writer, {0, 0, 0}, {255, 255, 255});
        for (int i = 0; i < 50; i++)
        {
            sprintf(buf, "New Line! %d\n", i);
            con.PutString(buf);
        }
    */
    console = new (console_buf) Console{*pixel_writer, {0, 0, 0}, {255, 255, 255}};
    // #@@range_end(new_console)

    // #@@range_begin(use_printk)
    for (int i = 0; i < 27; ++i)
    {
        printk("printk: %d\n", i);
    }
    while (1)
        __asm__("hlt");
}
