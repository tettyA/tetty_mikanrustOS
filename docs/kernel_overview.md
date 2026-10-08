# mikanrustos カーネル 処理解説

MikanOS（osbook）を Rust で実装している OS カーネルの、現時点（osbook_day10e まで）の処理内容と流れの解説です。
図は Mermaid で書いています（GitHub / VS Code のプレビューで表示できます）。

---

## 1. 全体像

```mermaid
flowchart TD
    UEFI["UEFI ファームウェア"] --> Loader["ブートローダ<br/>MikanLoaderPkg/Main.c<br/>(Loader.efi)"]
    Loader -->|"kernel.elf を読み込み<br/>FrameBufferConfig と MemoryMap を渡す"| KM["KernelMain<br/>(kernel/src/main.rs)"]
    KM --> KMN["KernelMainNewStack<br/>(専用スタックに切替後の本体)"]
    KMN --> Rust["Rust モジュール群<br/>描画 / レイヤー / コンソール / メモリ管理 / 割り込み"]
    KMN -->|"extern C"| Bridge["kernel/cpp/bridge/rust_bridge.cpp"]
    Bridge --> CPP["C++ USB ドライバ・PCI<br/>(kernel/cpp 以下, 本家 MikanOS のコード)"]
    CPP -->|"xHCI 割り込み (ベクタ 0x40)"| Rust
```

### ディレクトリ構成

| パス | 役割 |
|---|---|
| `MikanLoaderPkg/` | UEFI ブートローダ（C, edk2 でビルド） |
| `kernel/src/` | Rust 製カーネル本体 |
| `kernel/cpp/` | 本家 MikanOS 由来の C++ コード（USB ドライバ, PCI）。`make` で `libmikan_usb.a` になる |
| `kernel/cpp/usb/` | 本家の USB ドライバ（無改造） |
| `kernel/cpp/bridge/` | Rust から呼ぶための `extern "C"` 薄ラッパー |
| `kernel/build.rs` | `libmikan_usb.a` を Rust にリンクする設定 |
| `build_and_runOS.sh` | ローダ → C++ ライブラリ → Rust カーネル → QEMU 起動までを一括実行 |

### ビルドの流れ

```mermaid
flowchart LR
    A["edk2 build<br/>→ Loader.efi"] --> B["make (kernel/cpp)<br/>→ libmikan_usb.a"]
    B --> C["cargo build --release -p kernel<br/>(build.rs が .a をリンク)"]
    C --> D["kernel.elf"]
    D --> E["run_qemu.sh<br/>Loader.efi + kernel.elf"]
```

---

## 2. 起動から定常動作までの流れ

`KernelMain` は C ABI のエントリポイントで、ブートローダから `FrameBufferConfig` と `MemoryMap` を受け取ります。

```mermaid
flowchart TD
    S0["KernelMain<br/>カーネル専用スタック(1MiB)に rsp を切り替え"] --> S1["KernelMainNewStack"]
    S1 --> S2["segment::init_segment()<br/>GDT を作り CS/SS 等を設定"]
    S2 --> S3["paging::setup_identify_page_table()<br/>恒等マッピングを作り CR3 に設定"]
    S3 --> S4["MemoryMap を走査して<br/>MEMORY_MANAGER に使用中フレームを登録"]
    S4 --> S5["allocator::init_heap()<br/>フレームを確保しヒープ(Box/Vec/Rc)を有効化"]
    S5 --> S6["timer::initialize_lapic_timer()"]
    S6 --> S7["interrupts::init_idt()<br/>ベクタ 0x40 に xHCI 割り込みハンドラ登録"]
    S7 --> S8["画面の準備<br/>FrameBuffer / Window(背景・マウス・メイン・コンソール)"]
    S8 --> S9["LayerManager 作成<br/>レイヤー追加 + 重なり順設定"]
    S9 --> S10["LAYER_MANAGER に登録 → 全描画<br/>console::init() で printk 有効化"]
    S10 --> S11["MOUSE_CURSOR.init() とカーソル描画"]
    S11 --> S12["pci::ScanAllBus()<br/>HIDMouseDriver::set_default_observer()<br/>usb::xhci::Initialize()"]
    S12 --> S13["sti (割り込み許可)"]
    S13 --> S14["メモリマップを printk 表示"]
    S14 --> LOOP["メインループ (次章)"]
```

各ステップの内容:

1. **スタック切替** … ブートローダのスタックから、カーネル内の静的領域 `KERNEL_STACK` に切り替えます。
2. **GDT** … カーネル用コード/データセグメントを持つ GDT をロードし、セグメントレジスタを設定します。
3. **ページテーブル** … 64GiB を 2MiB ページで恒等マッピング（仮想アドレス＝物理アドレス）します。
4. **メモリマネージャ** … UEFI のメモリマップを読み、利用可能領域以外（隙間・予約領域）を「使用中」に印を付けます。
5. **ヒープ** … メモリマネージャから `64*512` フレーム（128MiB）を確保し、`linked_list_allocator` に渡します。以降 `Box`, `Vec`, `Rc` が使えます。
6. **IDT** … xHCI 割り込み（ベクタ `0x40`）のハンドラを登録します。
7. **画面構築** … 下の「描画の仕組み」参照。
8. **USB 初期化** … PCI 全探索 → マウスのコールバック登録 → xHC 探索・初期化・起動・接続済みポート設定。MSI 割り込みの設定もここで行われます。

---

## 3. メインループ（定常動作）

```mermaid
flowchart TD
    A["メインウィンドウのカウンタを描画<br/>(window_cnt をウィンドウに書き込み)"] --> B["redraw_layers_id(main_window_id)"]
    B --> C["割り込み禁止 (cli)"]
    C --> D{"INTERRUPTMESSAGE_QUEUE に<br/>メッセージあり?"}
    D -- あり --> E["割り込み許可 (sti)"]
    E --> F["usb::xhci::ProcessEvents()<br/>xHCI のイベントを全て処理"]
    F --> A
    D -- なし --> G["sti; hlt<br/>次の割り込みまで CPU 停止"]
    G --> A
```

- キューの確認から `hlt` までの間に割り込みが入って取りこぼすのを防ぐため、**割り込み禁止のまま確認 → なければ `sti; hlt`**（`sti` 直後の 1 命令は割り込まれないので安全）という手順です。
- カウンタは割り込みで起こされるたびに増えます（`hlt` で止まっている間は増えません）。

### マウス操作が画面に反映されるまで

```mermaid
sequenceDiagram
    participant HW as USB マウス / xHC
    participant ISR as 割り込みハンドラ<br/>(interrupts.rs)
    participant Q as INTERRUPTMESSAGE_QUEUE
    participant Main as メインループ
    participant USB as C++ USB ドライバ
    participant Cur as MouseCursor
    participant LM as LayerManager

    HW->>ISR: MSI 割り込み (ベクタ 0x40)
    ISR->>Q: InterruptXhci を push
    ISR->>HW: Local APIC に EOI (0xFEE000B0 へ書込み)
    Main->>Q: pop (割り込み禁止中)
    Main->>USB: usb::xhci::ProcessEvents()
    USB->>Cur: MouseObserver(buttons, dx, dy) を呼ぶ
    Cur->>LM: move_layer(マウスレイヤー, 新座標)
    Cur->>LM: draw_area(移動前後の矩形)
    LM->>LM: バックバッファに合成 → 画面へコピー
```

割り込みハンドラ内では**キューに積むだけ**で、重い処理（USB のイベント処理・描画）はメインループ側で行います。

---

## 4. 描画の仕組み

### 4.1 層構造

```mermaid
flowchart TD
    Config["FrameBufferConfig<br/>(解像度・ピクセル形式・VRAM アドレス)"] --> FB["FrameBuffer"]
    FB --> LM["LayerManager<br/>画面(FrameBuffer)とバックバッファを所有"]
    LM --> L["Layer × N<br/>(id, 位置, z, window)"]
    L --> W["Window<br/>(Rc&lt;RefCell&lt;Window&gt;&gt;, 内部に FrameBuffer のシャドウバッファ)"]
    W --> Draw["PixelDraw トレイト<br/>write / width / height"]
    Draw --> Font["font.rs: write_ascii / write_string<br/>(hankaku.bin の 8x16 フォント)"]
    Draw --> Gfx["graphics.rs: fill_rectrange / draw_rectrange"]
```

| モジュール | 内容 |
|---|---|
| `frame_buffer_config.rs` | ブートローダから渡される設定の型（`PixelFormat`: RGB / BGR） |
| `frame_buffer.rs` | `FrameBuffer`。VRAM または確保したバッファ。`copy_area`（矩形コピー）、`move`（領域移動）等 |
| `graphics.rs` | `PixelDraw` トレイト、`PixelColor`、`Vector2D`、`Rectangle`、矩形描画 |
| `font.rs` | `hankaku.bin` を使った文字描画。`write_ascii` / `write_string` |
| `window.rs` | `Window`（描画対象のバッファ。透明色 `set_transparent_color` 対応）、`draw_window`（タイトルバー付き枠の描画） |
| `layer.rs` | `Layer` / `LayerManager`。重なり順を管理して合成 |
| `console.rs` | 80×25 のテキストコンソール。`printk!` マクロの出力先 |
| `mousecursor.rs` | マウスカーソルの形状・位置管理 |

### 4.2 `LayerManager` の描画メソッド

| メソッド | 動作 |
|---|---|
| `draw_all()` | 重なり順（背面→前面）に全レイヤーを**直接画面へ**描画。 |
| `draw_area(area)` | 画面上の矩形 `area` だけを、各レイヤーから**バックバッファ**に合成してから画面へコピー（ちらつき防止）。 |
| `draw_id(id)` | レイヤー `id` の領域について、そのレイヤーと**前面側のレイヤー**を合成して画面へコピー。 |
| `up_down(id, z)` | 重なり順を設定（z が小さいほど背面、負で非表示）。 |
| `move_layer(id, pos)` | レイヤー位置を変更（再描画は呼び出し側で行う）。 |

現在のレイヤー構成（背面 → 前面）:

| z | レイヤー |
|---|---|
| 0 | 背景（デスクトップ） |
| 1 | コンソール |
| 2 | メインウィンドウ（"Hello window" とカウンタ） |
| 3 | マウスカーソル |

### 4.3 `printk!` の流れ

```mermaid
flowchart LR
    P["printk!(...)"] --> C["console::printk"]
    C --> G["CONSOLE.with(...)<br/>(Global&lt;Option&lt;Console&gt;&gt;)"]
    G --> PS["Console::put_string<br/>1文字ずつ Window に描画 + 内部バッファに記録"]
    PS --> NL{"最下行で改行?"}
    NL -- はい --> SC["Window 内を 1 行分上へ移動(スクロール)"]
    NL -- いいえ --> R
    SC --> R["redraw(layer_id) = redraw_layers_id<br/>→ 画面更新"]
```

`Console` 初期化前（`console::init` 前）の `printk!` は無視されます。

---

## 5. メモリ管理

```mermaid
flowchart TD
    MM["UEFI MemoryMap"] --> Init["起動時に走査"]
    Init --> BM["BitMapMemoryManager<br/>1 bit = 1 フレーム(4KiB) のビットマップ"]
    BM -->|"allocate(n フレーム)"| Heap["allocator::init_heap()<br/>128MiB を確保"]
    Heap --> LLA["linked_list_allocator (global_allocator)"]
    LLA --> Use["Box / Vec / Rc / format! など"]
```

- `BitMapMemoryManager`（`memory_manager.rs`）: `allocate` は先頭から連続した空きフレームを探して確保、`free` で解放、`mark_allocated` で使用中を登録、`set_memory_range` で管理範囲を設定します。最大 128GiB まで管理できます。
- `FrameBuffer::new` は `frame_buffer` が null のときだけ `Vec` で領域を確保します（バックバッファ・ウィンドウ用）。VRAM は実アドレスをそのまま使います。

---

## 6. 割り込みと排他制御

| 項目 | 内容 |
|---|---|
| `interrupts.rs` | IDT の初期化。ベクタ `0x40`（xHCI）のハンドラが `INTERRUPTMESSAGE_QUEUE` に push し、EOI を書きます。 |
| `queue.rs` | 256 要素のリングバッファ `Queue<T>`（満杯なら push は失敗） |
| `timer.rs` | Local APIC タイマー（初期化・計測開始・経過取得・停止）。マウス移動処理の所要時間計測用 |
| `segment.rs` / `paging.rs` | GDT 設定 / 恒等ページテーブル |

### `Global<T>`（`global.rs`）

`static mut` の代わりに使うグローバル変数ラッパーです。

```rust
static COUNTER: Global<u32> = Global::new(0);
COUNTER.with(|c| *c += 1);
```

- `with` の間は**割り込みを禁止**（終了時に元の状態へ復帰）→ 割り込みハンドラとの競合を防止。
- 同じ `Global` に対する**再入はパニック**（`&mut` の重複を防ぐ）。
- シングルコア前提です。

主なグローバル変数:

| 変数 | 型 | 用途 |
|---|---|---|
| `MEMORY_MANAGER` | `Global<BitMapMemoryManager>` | フレーム管理 |
| `LAYER_MANAGER` | `Global<Option<LayerManager>>` | 画面合成 |
| `CONSOLE` | `Global<Option<Console>>` | `printk!` の出力先 |
| `MOUSE_CURSOR` | `Global<MouseCursor>` | カーソル位置・状態 |
| `INTERRUPTMESSAGE_QUEUE` | `Global<Queue<InterruptMessage>>` | 割り込み → メインループの通知 |
| `IDT` / `GDT` / ページテーブル | `Global<...>` | CPU が参照するテーブル |

> 注意: `with` の中から、同じ `Global` を使う処理（例: `CONSOLE.with` の中での `printk!`）を呼ぶと再入でパニックします。

---

## 7. USB（C++ ドライバの利用）

USB ドライバ本体は本家 MikanOS の C++ コードで、Rust からは**本家と同じ名前**で呼べます（`kernel/src/usb/mod.rs`, `kernel/src/pci.rs`）。

| 本家 (C++) | Rust |
|---|---|
| `pci::ScanAllBus()` | `pci::ScanAllBus()` |
| `usb::HIDMouseDriver::default_observer = f` | `usb::HIDMouseDriver::set_default_observer(f)` |
| `usb::HIDKeyboardDriver::default_observer = f` | `usb::HIDKeyboardDriver::set_default_observer(f)` |
| `usb::xhci::Initialize()` | `usb::xhci::Initialize()` |
| `usb::xhci::ProcessEvents()` | `usb::xhci::ProcessEvents()` |

```mermaid
flowchart LR
    R["Rust: usb/mod.rs, pci.rs"] -->|"extern C"| B["bridge/rust_bridge.cpp"]
    B --> X["usb::xhci::Initialize / ProcessEvents"]
    B --> P["pci::ScanAllBus など"]
    B --> O["HIDMouseDriver::default_observer<br/>HIDKeyboardDriver::default_observer"]
```

`usb::xhci::Initialize()` の内部処理（C++ 側, `usb/xhci/xhci.cpp`）:

```mermaid
flowchart TD
    I1["PCI デバイス一覧から xHC を探す<br/>(クラス 0x0c/0x03/0x30, Intel 製を優先)"] --> I2["MSI 割り込み設定<br/>(宛先 = BSP の Local APIC, ベクタ 0x40)"]
    I2 --> I3["BAR0 から MMIO ベースアドレスを取得"]
    I3 --> I4["Controller を生成"]
    I4 --> I5["Intel なら SwitchEhci2Xhci()"]
    I5 --> I6["Controller::Initialize() → Run()"]
    I6 --> I7["接続済みの各ポートに ConfigurePort()"]
```

その後、マウスが動くたびに登録したコールバック（`mousecursor::MouseObserver`）が `(buttons, dx, dy)` で呼ばれ、カーソルが移動します。
C++ 側の `Log()` は何も出力しません（デバッグ出力は未接続）。

---

## 8. ファイル別 早見表

| ファイル | 主な内容 |
|---|---|
| `main.rs` | `KernelMain`（スタック切替）、`KernelMainNewStack`（初期化とメインループ）、`redraw_layers_*`、パニックハンドラ |
| `global.rs` | `Global<T>` |
| `segment.rs` | GDT 初期化 |
| `paging.rs` | 恒等マッピングのページテーブル設定 |
| `memory_map.rs` | UEFI メモリマップの型 (`MemoryMap`, `MemoryDescriptor`, `MemoryType`) |
| `memory_manager.rs` | ビットマップによるフレーム管理 |
| `allocator.rs` | ヒープの初期化と `#[global_allocator]` |
| `interrupts.rs` | IDT・xHCI 割り込みハンドラ・メッセージキュー |
| `queue.rs` | リングバッファ |
| `timer.rs` | Local APIC タイマー |
| `frame_buffer_config.rs` / `frame_buffer.rs` | 画面バッファ |
| `graphics.rs` / `font.rs` | 描画プリミティブ / フォント |
| `window.rs` / `layer.rs` | ウィンドウ / レイヤー合成 |
| `console.rs` | テキストコンソールと `printk!` |
| `mousecursor.rs` | マウスカーソル |
| `pci.rs` / `usb/mod.rs` | C++ の PCI / USB ドライバの Rust インターフェース |
