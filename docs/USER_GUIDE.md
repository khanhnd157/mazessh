# Maze SSH — User Guide

Hướng dẫn sử dụng ứng dụng Maze SSH Desktop cho việc quản lý SSH Identity trong Git Workflows.

## Mục lục

1. [Giới thiệu](#giới-thiệu)
2. [Cài đặt](#cài-đặt)
3. [Giao diện chính](#giao-diện-chính)
4. [Quản lý Profile](#quản-lý-profile)
5. [Chuyển đổi Profile](#chuyển-đổi-profile)
6. [Repo Mapping](#repo-mapping)
7. [SSH Config](#ssh-config)
8. [Bảo mật](#bảo-mật)
9. [Phím tắt](#phím-tắt)
10. [Xử lý sự cố](#xử-lý-sự-cố)

---

## Giới thiệu

Maze SSH giúp developer quản lý nhiều SSH Identity (GitHub, GitLab, Gitea, Bitbucket) trên cùng một máy tính. Thay vì chỉnh sửa `~/.ssh/config` thủ công, bạn chỉ cần click để chuyển đổi giữa các tài khoản.

### Maze SSH làm gì khi bạn switch profile?

1. Load SSH key vào Windows SSH Agent (`ssh-add`)
2. Set biến môi trường `GIT_SSH_COMMAND` cho tất cả terminal mới
3. Cập nhật `git config user.name` và `user.email`
4. Ghi file env tại `~/.maze-ssh/env` để terminal hiện tại source

Kết quả: mọi thao tác `git push`, `git pull`, `git clone` đều dùng đúng SSH key.

---

## Cài đặt

### Yêu cầu hệ thống

- Windows 10/11 (hoặc macOS, Linux)
- OpenSSH đã cài sẵn (Windows 10+ có sẵn)
- Git đã cài

### Tải và cài đặt

Tải installer từ [GitHub Releases](https://github.com/khanhnd157/mazessh/releases):

| Hệ điều hành | File |
| ------------- | ---- |
| Windows | `Maze.SSH_x64-setup.exe` hoặc `Maze.SSH_x64_en-US.msi` |
| macOS Intel | `Maze.SSH_x64.dmg` |
| macOS Apple Silicon | `Maze.SSH_aarch64.dmg` |
| Linux (Debian/Ubuntu) | `Maze.SSH_amd64.deb` |
| Linux (Fedora/RHEL) | `Maze.SSH-x86_64.rpm` |
| Linux (Universal) | `Maze.SSH_amd64.AppImage` |

### Sau khi cài đặt

Ứng dụng tự động:

- Quét SSH keys có sẵn trong `~/.ssh/`
- Khởi động Windows SSH Agent service nếu chưa chạy
- Hiển thị trong system tray

---

## Giao diện chính

```
┌──────────────────────────────────────────────────────────┐
│ [Logo] Maze SSH | ● Profile Name  Provider  [Switch] ... │ ← Titlebar
├─────────┬──────────────────────────────────────────┬─────┤
│ PROFILES│  Profiles  │ Repo Mappings │ SSH Config │ ... │ ← Tabs
│         ├────────────────────────────────────────────────┤
│ ● Prof1 │                                                │
│ ○ Prof2 │          Profile Detail / Tab Content          │ ← Main
│         │                                                │
├─────────┴────────────────────────────────────────────────┤
│ Activity Log                                             │ ← Bottom
│ Git: username <email>                                    │
└──────────────────────────────────────────────────────────┘
```

### Titlebar

- **Trạng thái**: hiển thị profile đang active (chấm xanh nhấp nháy) hoặc "No active profile"
- **Switch**: dropdown chọn nhanh profile
- **Deactivate**: tắt profile hiện tại
- **Lock**: khóa ứng dụng (khi đã cài PIN)
- **Theme**: chuyển Dark/Light
- **Window controls**: Minimize, Maximize, Close (ẩn xuống tray)

### Sidebar

Danh sách tất cả profiles. Click vào profile để xem chi tiết. Nút **+ New** để tạo profile mới.

### Tabs

- **Profiles**: xem và quản lý profile chi tiết
- **Repo Mappings**: gán repository cho profile
- **SSH Config**: xem, ghi, rollback SSH config
- **Settings**: bảo mật, PIN, timeout

### Bottom Bar

- **Activity Log**: lịch sử thao tác (switch, test, lock...)
- **Git identity**: hiển thị `user.name <user.email>` hiện tại

---

## Quản lý Profile

### Tạo profile mới

1. Click **+ New** ở sidebar
2. Điền thông tin:
   - **Profile Name**: tên hiển thị (ví dụ: "Work GitHub")
   - **Provider**: chọn GitHub, GitLab, Gitea, hoặc Bitbucket
   - **Email**: email liên kết với tài khoản Git
   - **Git Username**: username Git (hiển thị trong commits)
   - **SSH Private Key**: đường dẫn đến private key
     - Ứng dụng tự quét `~/.ssh/` và hiển thị các key tìm thấy
     - Click vào key để chọn nhanh
   - **Host Alias**: tên alias cho SSH config (tự tạo từ Profile Name)
   - **Hostname**: địa chỉ server (tự điền theo Provider)
3. Click **Create Profile**

### Chỉnh sửa profile

1. Chọn profile ở sidebar
2. Click **Edit** trong trang chi tiết
3. Sửa thông tin → **Save Changes**

### Xóa profile

1. Chọn profile ở sidebar
2. Click **Delete**
3. Xác nhận trong dialog → profile và repo mappings liên quan sẽ bị xóa

### Xem thông tin chi tiết

Trang profile detail hiển thị:

- **Host Alias** và **Hostname**: thông tin kết nối SSH
- **SSH User** và **Port**: mặc định `git` và `22`
- **Git Username** và **Key Type**: thông tin identity
- **SSH Private Key**: đường dẫn key (hover để copy)
- **Key Fingerprint**: SHA256 hash và loại key (ED25519, RSA...)
- **Mapped Repositories**: danh sách repo đã gán cho profile này

### Test kết nối

Click **Test Connection** để kiểm tra SSH key có kết nối được đến server không:

- **Thành công**: hiện thông báo xanh với username đã xác thực
- **Thất bại**: hiện thông báo đỏ với chi tiết lỗi

### Export / Import

Trong tab **Settings**:

- **Export to Clipboard**: copy tất cả profiles dạng JSON (không bao gồm passphrase)
- **Import from Clipboard**: paste JSON để import profiles mới (bỏ qua trùng tên)

---

## Chuyển đổi Profile

### Cách 1: Switch từ Titlebar (nhanh nhất)

1. Click **Switch** trên titlebar
2. Chọn profile từ dropdown
3. Ứng dụng tự động: load key → set env → sync git identity

### Cách 2: Activate từ Profile Detail

1. Chọn profile ở sidebar
2. Click **Activate**

### Cách 3: Phím tắt

Dùng **Ctrl+L** để lock, **Ctrl+1-4** để chuyển tab.

### Khi switch profile thì sao?

- **Terminal mới**: tự động dùng đúng key (qua biến môi trường `GIT_SSH_COMMAND`)
- **Terminal đang mở**: cần chạy `source ~/.maze-ssh/env` hoặc mở terminal mới
- **ssh-add -l**: sẽ hiển thị đúng key đang active
- **git push/pull**: sẽ dùng đúng identity

### Deactivate

Click **Deactivate** trên titlebar hoặc dùng CLI `maze-ssh-cli off`:

- Xóa key khỏi SSH agent
- Xóa biến môi trường `GIT_SSH_COMMAND`
- Không ảnh hưởng đến SSH keys trên đĩa

---

## Repo Mapping

Tự động chuyển profile dựa trên thư mục repository.

### Tạo mapping

1. Chuyển sang tab **Repo Mappings**
2. Click **Add Mapping**
3. Nhập đường dẫn repository (ứng dụng tự xác nhận có phải git repo không)
4. Chọn profile
5. Chọn scope:
   - **Local**: chỉ set `git config` cho repo này (khuyến nghị)
   - **Global**: set `git config --global`
6. Click **Create Mapping**

### Cài đặt Git Hook

Trên mỗi mapping card, hover sẽ thấy icon **Git Branch**. Click để cài pre-push hook:

- Hook kiểm tra `git config user.email` trước mỗi `git push`
- Nếu email không khớp với profile → chặn push và hiển thị cảnh báo
- Chỉ xóa hook nếu do Maze SSH tạo

### Xóa mapping

Hover vào mapping card → click icon **Trash** → xác nhận.

---

## SSH Config

Tab **SSH Config** quản lý file `~/.ssh/config`.

### Preview

Xem SSH config sẽ được tạo từ tất cả profiles:

```
# === BEGIN MAZE-SSH MANAGED ===
Host github-work
  HostName github.com
  User git
  IdentityFile C:\Users\you\.ssh\gh_ed25519_work
  IdentitiesOnly yes
# === END MAZE-SSH MANAGED ===
```

### Write Config

Click **Write Config** để ghi vào `~/.ssh/config`:

- Tự động tạo backup trước khi ghi
- Chỉ thay đổi phần giữa markers `BEGIN/END MAZE-SSH MANAGED`
- Nội dung bạn tự viết ngoài markers được giữ nguyên

### Current

Xem nội dung hiện tại của `~/.ssh/config`.

### Backups

Xem danh sách các bản backup với thời gian và kích thước. Click **Rollback** để khôi phục bản backup bất kỳ (bản hiện tại sẽ được backup trước khi rollback).

---

## Bảo mật

### Thiết lập PIN

1. Chuyển sang tab **Settings**
2. Trong phần **PIN Protection**, click **Set PIN**
3. Nhập PIN (tối thiểu 4 ký tự) và xác nhận
4. PIN được hash bằng Argon2 và lưu trong Windows Credential Manager

### Khóa ứng dụng

- **Thủ công**: click icon **Lock** trên titlebar hoặc nhấn **Ctrl+L**
- **Tự động**: cấu hình timeout trong Settings (5, 15, 30, hoặc 60 phút không thao tác)
- **Khi minimize**: bật "Lock when minimized to tray" trong Settings

Khi khóa:

- Màn hình lock che toàn bộ giao diện
- SSH agent keys được xóa
- Tất cả thao tác bị chặn cho đến khi nhập đúng PIN

### Giới hạn nhập sai PIN

- Tối đa 5 lần nhập sai liên tiếp
- Sau 5 lần sai → chờ 60 giây trước khi thử lại

### Agent Key Timeout

Cấu hình trong Settings → **Agent Key Timeout**:

- Sau thời gian cấu hình, SSH keys tự động bị xóa khỏi agent
- Độc lập với việc khóa ứng dụng
- Profile tự động deactivate khi keys hết hạn

### Đổi / Xóa PIN

Trong Settings → **PIN Protection**:

- **Change PIN**: nhập PIN cũ + PIN mới
- **Remove PIN**: nhập PIN để xác nhận → tắt tính năng khóa

### Audit Log

Tất cả thao tác bảo mật được ghi vào `~/.maze-ssh/audit.log`:

- Lock / Unlock (thành công và thất bại)
- Thay đổi PIN
- Thay đổi settings
- Agent keys hết hạn

Xem trong Settings → **Audit Log** → **View Log**.

---

## Phím tắt

| Phím | Chức năng |
| ---- | --------- |
| **Ctrl+1** | Chuyển tab Profiles |
| **Ctrl+2** | Chuyển tab Repo Mappings |
| **Ctrl+3** | Chuyển tab SSH Config |
| **Ctrl+4** | Chuyển tab Settings |
| **Ctrl+L** | Khóa ứng dụng |
| **Escape** | Đóng dialog / dropdown |

---

## Xử lý sự cố

### "ssh-add -l" không hiển thị key sau khi switch

Terminal hiện tại cần được refresh. Mở terminal mới hoặc chạy:

```bash
source ~/.maze-ssh/env
```

### SSH Agent service không khởi động

Mở PowerShell với quyền Admin và chạy:

```powershell
Set-Service ssh-agent -StartupType Manual
Start-Service ssh-agent
```

### Push sai tài khoản

1. Kiểm tra profile đang active: xem titlebar hoặc chạy `maze-ssh-cli current`
2. Switch sang profile đúng
3. Kiểm tra git identity: `git config user.email`
4. Cài git hook để ngăn chặn: tab Repo Mappings → hover mapping → click icon Git Branch

### SSH config bị hỏng sau khi ghi

1. Tab SSH Config → **Backups**
2. Chọn bản backup gần nhất → **Rollback**
3. Nội dung bên ngoài markers `MAZE-SSH MANAGED` luôn được bảo toàn

### Ứng dụng bị khóa và quên PIN

PIN được lưu trong Windows Credential Manager. Để reset:

1. Mở **Credential Manager** trong Windows (Control Panel → Credential Manager)
2. Chọn tab **Windows Credentials**
3. Tìm entry `maze-ssh / pin-hash`
4. Xóa entry đó
5. Khởi động lại ứng dụng — PIN sẽ được reset

### System tray không hiển thị

Ứng dụng minimize xuống tray khi đóng cửa sổ. Click icon Maze SSH trong system tray để mở lại. Nếu không thấy icon, kiểm tra khay ẩn (hidden icons) trên taskbar.

---

## Dữ liệu

Tất cả dữ liệu lưu tại `~/.maze-ssh/`:

| File | Nội dung |
| ---- | -------- |
| `profiles.json` | Thông tin profiles (không chứa private key, chỉ đường dẫn) |
| `active.txt` | ID profile đang active |
| `repo_mappings.json` | Mapping repo → profile |
| `settings.json` | Cài đặt bảo mật (timeout, lock-on-minimize) |
| `env` | File env cho shell sourcing |
| `audit.log` | Nhật ký bảo mật |

### Bảo mật dữ liệu

- **Private keys**: KHÔNG được lưu trong ứng dụng, chỉ lưu đường dẫn
- **Passphrases**: lưu trong Windows Credential Manager (mã hóa bởi OS)
- **PIN**: hash Argon2, lưu trong Windows Credential Manager
- **Profiles**: JSON plaintext — chỉ chứa metadata, không có secrets
