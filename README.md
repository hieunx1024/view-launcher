# view-launcher 🚀

Trình khởi chạy ứng dụng và tệp tin siêu tối giản, tốc độ cực hạn (<1ms) viết bằng **Rust**. Hoạt động mượt mà trên cả **Linux** (Wayland/X11) và **Windows** thông qua giao diện dòng lệnh TUI (`ratatui` + `crossterm`).

---

## ✨ Tính năng cốt lõi
- ⚡ **Khởi động tức thì (<1ms):** Cơ chế quét tệp nền bất đồng bộ (background thread scan) không gây giật lag UI.
- 🔍 **Tìm kiếm thông minh:** Tìm kiếm mờ (fuzzy search) cho cả Ứng dụng, Tệp tin & Thư mục.
- 🇻🇳 **Hỗ trợ Tiếng Việt:** Tìm kiếm tiếng Việt không dấu tự động (ví dụ gõ `tai lieu` tìm ra `Tài liệu`).
- 🔄 **Bật/Tắt thông minh (Toggle):** Nhấn phím tắt để mở, nhấn lại để tự động đóng (dùng TCP Loopback).
- 🎨 **Tùy biến cao:** Cấu hình màu sắc, độ sâu tìm kiếm qua tệp `config.toml`.

---

## 🛠️ Hướng dẫn cài đặt

Bạn chỉ cần biên dịch một lần để tạo ra tệp chạy duy nhất, không phụ thuộc vào bất kỳ thư viện ngoài nào.

```bash
# 1. Biên dịch bản release tối ưu
cargo build --release
```
Tệp nhị phân sau khi biên dịch nằm tại: `target/release/view-launcher` (hoặc `.exe` trên Windows).

---

## 🖥️ Hướng dẫn chạy và Thiết lập phím tắt

### 1. Trên Linux (Sway / i3 / Hyprland)

Chép tệp chạy vào thư mục hệ thống cục bộ:
```bash
cp target/release/view-launcher ~/.local/bin/
```

Mở tệp cấu hình của Window Manager (ví dụ `~/.config/sway/config`) và thêm phím tắt `Ctrl + Space`:
```plaintext
# Thiết lập cửa sổ nổi cho launcher
for_window [app_id="floating_launcher"] floating enable, resize set 700 450, move position center

# Gán phím tắt gọi launcher trong terminal mặc định (ở đây ví dụ là kitty)
bindsym ctrl+space exec $term --app-id floating_launcher -e view-launcher
```

---

### 2. Trên Windows

Biên dịch dự án trên Windows sẽ tạo ra file **`view-launcher.exe`** (dung lượng ~1.5MB). Bạn có thể chép file này vào bất kỳ thư mục nào trong máy.

**🚀 Tính năng Cài đặt Tự động (Mới):**
Ứng dụng được lập trình để **tự động cấu hình hệ thống** ngay trong lần chạy đầu tiên:
- **Tự khởi chạy (Startup):** Tự động tạo lối tắt trong thư mục khởi động của Windows (`shell:startup`).
- **Phím tắt toàn cục (Global Hotkey):** Tự động liên kết tổ hợp phím **`Ctrl + Alt + Space`** để gọi nhanh Windows Terminal chứa launcher từ bất kỳ màn hình nào!

Bạn chỉ cần kích hoạt nhấp đúp chạy `view-launcher.exe` một lần, toàn bộ hệ thống phím tắt và Startup sẽ được đăng ký ngầm hoàn toàn tự động.

---

### 3. Tùy chọn nâng cao (Gán phím tắt `Ctrl + Space` trên Windows)

Nếu bạn muốn sử dụng phím tắt cực ngắn như **`Ctrl + Space`** (giống hệt trên Linux Sway) thay vì phím tắt mặc định hệ thống `Ctrl + Alt + Space`, bạn có thể sử dụng công cụ **AutoHotkey** (miễn phí, siêu nhẹ):

1. Cài đặt AutoHotkey, tạo một file script `launcher.ahk` có nội dung sau:
   ```autohotkey
   ^Space::
   Run, wt.exe --title "floating_launcher" view-launcher.exe
   return
   ```
2. Lưu file script này vào thư mục Khởi động tự động (`shell:startup`) để nó tự chạy ngầm cùng Windows mỗi khi mở máy.

---

## ⚙️ Cấu hình tùy biến (`config.toml`)

Ứng dụng tự động nạp cấu hình tùy biến của bạn tại:
- **Linux:** `~/.config/view-launcher/config.toml`
- **Windows:** `%APPDATA%\view-launcher\config.toml`

**Mẫu cấu hình tối giản:**
```toml
[theme]
query_color = "cyan"
selection_bg = "#2d3748"
selection_fg = "white"
app_badge_color = "cyan"
file_badge_color = "yellow"
border_color = "#4a5568"

[search]
max_depth = 3
ignored_dirs = [".git", ".cargo", ".cache", "node_modules", "target"]
```
