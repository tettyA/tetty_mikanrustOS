#include <cstddef>
#include <new>
#include <cstdint>

extern "C"
{
    // 共有ライブラリの破棄制御用ハンドラ（静的リンクでは使用しませんわ）
    void *__dso_handle = nullptr;

    // 純粋仮想関数が誤って呼び出された場合のフォールバックですわ
    void __cxa_pure_virtual()
    {
        while (1)
            __asm__("hlt");
    }

    // 静的オブジェクトのデストラクタ登録関数ですわ（カーネル終了処理はしないため無視しますわ）
    int __cxa_atexit(void (*destructor)(void *), void *arg, void *dso_handle)
    {
        return 0;
    }

    // 致命的エラー時の停止関数ですわ
    void abort()
    {
        while (1)
            __asm__("hlt");
    }
}

// 簡易的な固定長アロケータですわ（64KB）
static uint8_t memory_pool[64 * 1024];
static size_t memory_index = 0;

void *operator new(size_t size)
{
    size_t alignment = 8;
    memory_index = (memory_index + alignment - 1) & ~(alignment - 1);
    if (memory_index + size > sizeof(memory_pool))
        abort();
    void *p = &memory_pool[memory_index];
    memory_index += size;
    return p;
}

void *operator new(size_t size, std::align_val_t align)
{
    size_t alignment = static_cast<size_t>(align);
    memory_index = (memory_index + alignment - 1) & ~(alignment - 1);
    if (memory_index + size > sizeof(memory_pool))
        abort();
    void *p = &memory_pool[memory_index];
    memory_index += size;
    return p;
}

void operator delete(void *p) noexcept {}
void operator delete(void *p, size_t size) noexcept {}
void operator delete(void *p, std::align_val_t align) noexcept {}