# Driver de teste de janelas e de varios monitores no Windows. Dot-source: . .\windows-monitores.ps1
# Regra de seguranca: tecla ou clique so sai se o primeiro plano for do pid iniciado aqui.
# Antes de tudo: Backup-Settings (e confira o hash no fim: Restore-Settings devolve os dois).
# Pano de fundo: Start-Backdrop cobre cada monitor com uma cor solida (ver windows-pano-de-fundo.ps1),
# para a captura do Takeshot pegar so isso e a tela do usuario nunca entrar numa imagem.
# Os PNG de Shot-Rect/Shot-Monitors saem em shots\e2e. Depois do primeiro arrasto numa captura em
# tela cheia a copia de tela (CopyFromScreen) fica congelada: confie no estado do app (qual janela
# salva, onde abre o dialogo), nao em pixels.
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class M {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
  // "hwnd|titulo|L|T|largura|altura|topmost" das janelas visiveis do pid com pelo menos minSize de lado
  public static List<string> ForPid(uint pid, int minSize) {
    var res = new List<string>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p != pid || !IsWindowVisible(h)) return true;
      RECT r; GetWindowRect(h, out r);
      if (r.R - r.L < minSize || r.B - r.T < minSize) return true;
      var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
      int ex = GetWindowLong(h, -20);
      res.Add(h.ToInt64() + "|" + sb + "|" + r.L + "|" + r.T + "|" + (r.R - r.L) + "|" + (r.B - r.T) + "|" + (((ex & 8) != 0) ? "topmost" : "normal"));
      return true;
    }, IntPtr.Zero);
    return res;
  }
}
"@
[void][M]::SetProcessDPIAware()

$script:Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$script:Dir = Join-Path $script:Root 'shots'
$script:Out = Join-Path $script:Dir 'e2e'
New-Item -ItemType Directory -Force $script:Out | Out-Null
$script:Settings = Join-Path $env:APPDATA 'takeshot\settings.json'
$script:Backup = Join-Path $script:Dir 'settings.backup.json'
$script:Vk = @{ ctrl = 0xA2; shift = 0xA0; alt = 0xA4; esc = 0x1B; enter = 0x0D; s = 0x53; c = 0x43; e = 0x45; t = 0x54 }
$script:AppPid = 0
$script:BackdropPid = 0
$script:Settle = 500   # ms que o app tem para redesenhar depois de soltar o botao

function Screens { [System.Windows.Forms.Screen]::AllScreens | ForEach-Object { "{0} primary={1} {2},{3} {4}x{5}" -f $_.DeviceName, $_.Primary, $_.Bounds.X, $_.Bounds.Y, $_.Bounds.Width, $_.Bounds.Height } }

function Set-TestSettings([hashtable]$h) {
  # Sobrescreve o settings.json do usuario so com o que o teste precisa (o backup e restaurado no fim)
  # Sem BOM: o app le o arquivo com serde_json, que recusa BOM e cai nos padroes em silencio.
  [System.IO.File]::WriteAllText($script:Settings, ($h | ConvertTo-Json), (New-Object System.Text.UTF8Encoding $false))
}
function Backup-Settings { Copy-Item -Force $script:Settings $script:Backup; (Get-FileHash $script:Settings).Hash }
function Restore-Settings { Copy-Item -Force $script:Backup $script:Settings; (Get-FileHash $script:Settings).Hash + ' ' + (Get-FileHash $script:Backup).Hash }

function Start-Backdrop {
  $p = Start-Process powershell.exe -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'windows-pano-de-fundo.ps1')) -PassThru
  $script:BackdropPid = $p.Id
  Start-Sleep -Seconds 4
  "backdrop pid=$($p.Id)"
}
function Stop-Backdrop { if ($script:BackdropPid) { Stop-Process -Id $script:BackdropPid -ErrorAction SilentlyContinue; $script:BackdropPid = 0 } }

function Wins([int]$min = 100) {
  [M]::ForPid([uint32]$script:AppPid, $min) | ForEach-Object {
    $f = $_.Split('|')
    [pscustomobject]@{ Hwnd = [int64]$f[0]; Title = $f[1]; L = [int]$f[2]; T = [int]$f[3]; W = [int]$f[4]; H = [int]$f[5]; Z = $f[6] }
  }
}
function Wins-Text([int]$min = 100) { (Wins $min | ForEach-Object { "  [{0}] '{1}' at {2},{3} {4}x{5} {6}" -f $_.Hwnd, $_.Title, $_.L, $_.T, $_.W, $_.H, $_.Z }) -join "`n" }
function Alive { try { $null = Get-Process -Id $script:AppPid -ErrorAction Stop; $true } catch { $false } }

function Fg-Info {
  $h = [M]::GetForegroundWindow(); $p = 0
  [void][M]::GetWindowThreadProcessId($h, [ref]$p)
  $r = New-Object M+RECT; [void][M]::GetWindowRect($h, [ref]$r)
  $sb = New-Object System.Text.StringBuilder 256; [void][M]::GetWindowText($h, $sb, 256)
  "foreground pid=$p (app=$script:AppPid) '$sb' at $($r.L),$($r.T) $($r.R - $r.L)x$($r.B - $r.T)"
}
function Assert-Foreground {
  $p = 0; [void][M]::GetWindowThreadProcessId([M]::GetForegroundWindow(), [ref]$p)
  if ($p -ne $script:AppPid) { throw "ABORTADO: o primeiro plano e o pid $p, nao o app ($script:AppPid). Nada foi enviado." }
}

function Start-App([string]$exe, [string]$argLine, [int]$cx, [int]$cy) {
  [void][M]::SetCursorPos($cx, $cy)
  $p = Start-Process -FilePath $exe -ArgumentList $argLine -PassThru
  $script:AppPid = $p.Id
  Start-Sleep -Seconds 4
  "app pid=$($p.Id) args=$argLine cursor=$cx,$cy"
}
function Stop-App { if ($script:AppPid) { Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $script:AppPid } | ForEach-Object { Stop-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }; Stop-Process -Id $script:AppPid -ErrorAction SilentlyContinue; $script:AppPid = 0 } }

function Shot-Rect([string]$name, [int]$x, [int]$y, [int]$w, [int]$h) {
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $path = Join-Path $script:Out "$name.png"
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  $path
}
function Shot-Monitors([string]$prefix) {
  $i = 0
  foreach ($s in [System.Windows.Forms.Screen]::AllScreens) { $i++; Shot-Rect ("{0}-m{1}" -f $prefix, $i) $s.Bounds.X $s.Bounds.Y $s.Bounds.Width $s.Bounds.Height | Out-Null }
}
function Px([int]$x, [int]$y) {
  $bmp = New-Object System.Drawing.Bitmap 1, 1
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size 1, 1))
  $c = $bmp.GetPixel(0, 0); $g.Dispose(); $bmp.Dispose()
  "{0},{1},{2}" -f $c.R, $c.G, $c.B
}

function Hwnd-Of([string]$m) {   # R = janela do monitor primario (L >= 0), L = a da esquerda
  $w = Wins | Where-Object { $_.W -ge 1000 } | Where-Object { if ($m -eq 'R') { $_.L -ge 0 } else { $_.L -lt 0 } } | Select-Object -First 1
  if (-not $w) { throw "sem janela de captura no monitor $m" }
  [IntPtr]$w.Hwnd
}
function Focus-Hwnd([IntPtr]$h) {
  [M]::keybd_event(0xA4, 0, 0, [UIntPtr]::Zero); [M]::keybd_event(0xA4, 0, 2, [UIntPtr]::Zero)   # Alt solto destrava o foco
  [void][M]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 500
}
function Top-At([int]$x, [int]$y) {   # janela de topo (raiz) que recebe um clique nesse ponto
  $pt = New-Object M+POINT; $pt.X = $x; $pt.Y = $y
  $h = [M]::GetAncestor([M]::WindowFromPoint($pt), 2)
  $p = 0; [void][M]::GetWindowThreadProcessId($h, [ref]$p)
  $sb = New-Object System.Text.StringBuilder 256; [void][M]::GetWindowText($h, $sb, 256)
  "{0} '{1}' pid={2}" -f $h.ToInt64(), $sb, $p
}
function Move-To([int]$x, [int]$y) { [void][M]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 120 }
function Drag([int]$x1, [int]$y1, [int]$x2, [int]$y2) {
  Assert-Foreground
  Move-To $x1 $y1
  [M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150
  $steps = 12
  foreach ($i in 1..$steps) { Move-To ($x1 + [int](($x2 - $x1) * $i / $steps)) ($y1 + [int](($y2 - $y1) * $i / $steps)); Start-Sleep -Milliseconds 40 }
  Start-Sleep -Milliseconds 150
  [M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds $script:Settle
}
function Click([int]$x, [int]$y) {
  Move-To $x $y
  [M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80; [M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400
}
function Key-Down([string]$k) { $vk = $script:Vk[$k]; [M]::keybd_event([byte]$vk, [byte][M]::MapVirtualKey($vk, 0), 0, [UIntPtr]::Zero) }
function Key-Up([string]$k) { $vk = $script:Vk[$k]; [M]::keybd_event([byte]$vk, [byte][M]::MapVirtualKey($vk, 0), 2, [UIntPtr]::Zero) }
function Chord([string[]]$keys) {
  Assert-Foreground
  foreach ($k in $keys) { Key-Down $k; Start-Sleep -Milliseconds 100 }
  Start-Sleep -Milliseconds 120
  for ($i = $keys.Count - 1; $i -ge 0; $i--) { Key-Up $keys[$i]; Start-Sleep -Milliseconds 40 }
  Start-Sleep -Milliseconds 700
}
