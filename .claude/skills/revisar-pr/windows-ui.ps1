# Conduz o exe do Takeshot em teste no Windows, com teclas e cliques reais (SendInput).
# Uso: dot-source num powershell, depois chame as funcoes (uma chamada de PowerShell pode ter varias):
#   . .\.claude\skills\revisar-pr\windows-ui.ps1
#   Start-App "<exe>" "--page ajustes --lang en --theme takeui --mode light --captures-dir <repo>\docs\images"
#   Focus-App; ClickAt 87 196; Chord @('ctrl','shift','9'); Shot 'passo-1'
# Regra de seguranca: tecla ou clique so sai se a janela em primeiro plano for do processo que
# este script iniciou; senao a funcao aborta e nada e enviado. O processo do app tem uma janela
# host invisivel ("Takeshot resident", fora da tela): por isso as janelas visiveis sao enumeradas.
# Sempre inicie com --captures-dir apontando para docs/images: a Home mostra a pasta do usuario.
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class W {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
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
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  public static List<string> ForPid(uint pid) {   // "hwnd|titulo|largura|altura" das janelas visiveis e grandes
    var res = new List<string>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p != pid || !IsWindowVisible(h)) return true;
      RECT r; GetWindowRect(h, out r);
      if (r.R - r.L < 100 || r.B - r.T < 100) return true;
      var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
      res.Add(h.ToInt64() + "|" + sb + "|" + (r.R - r.L) + "|" + (r.B - r.T));
      return true;
    }, IntPtr.Zero);
    return res;
  }
}
"@
[void][W]::SetProcessDPIAware()

$script:Pidfile = Join-Path $env:TEMP 'takeshot-review-app.pid'
$script:Out = Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\shots')).Path 'e2e'
New-Item -ItemType Directory -Force $script:Out | Out-Null

# Teclas por nome (virtual-key). Digitos: $script:Vk['5'] = 0x35.
$script:Vk = @{ ctrl = 0xA2; shift = 0xA0; alt = 0xA4; esc = 0x1B; f9 = 0x78; enter = 0x0D; '9' = 0x39 }

function Get-AppProc { Get-Process -Id ([int](Get-Content $script:Pidfile)) -ErrorAction Stop }
function Get-Wins { [W]::ForPid([uint32](Get-AppProc).Id) }

function Get-Hwnd {
  $w = Get-Wins | Select-Object -First 1
  if (-not $w) { throw 'nenhuma janela visivel do app (escondida? abra o exe de novo: a segunda abertura a traz de volta)' }
  [IntPtr][int64]($w.Split('|')[0])
}

function Start-App([string]$exe, [string]$argLine) {
  $p = Start-Process -FilePath $exe -ArgumentList $argLine -PassThru
  Set-Content $script:Pidfile $p.Id
  Start-Sleep -Seconds 5
  "pid=$($p.Id) janelas: " + ((Get-Wins) -join ' ; ')
}

function Get-Rect { $r = New-Object W+RECT; [void][W]::GetWindowRect((Get-Hwnd), [ref]$r); $r }

function Assert-Foreground {
  $fpid = 0
  [void][W]::GetWindowThreadProcessId([W]::GetForegroundWindow(), [ref]$fpid)
  if ($fpid -ne (Get-AppProc).Id) { throw "ABORTADO: o primeiro plano e o pid $fpid, nao o app. Nada foi enviado." }
}

function Focus-App {
  $h = Get-Hwnd
  [void][W]::ShowWindow($h, 9)
  [W]::keybd_event(0xA4, 0, 0, [UIntPtr]::Zero); [W]::keybd_event(0xA4, 0, 2, [UIntPtr]::Zero)   # Alt solto destrava o foco
  [void][W]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 500
  Assert-Foreground
}

function Shot([string]$name) {   # PNG da janela do app em shots/e2e
  $r = Get-Rect; $w = $r.R - $r.L; $h = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $path = Join-Path $script:Out "$name.png"
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  "$path ($w x $h)"
}

function ClickAt([int]$x, [int]$y) {   # coordenadas relativas ao canto da janela
  Assert-Foreground
  $r = Get-Rect
  [void][W]::SetCursorPos($r.L + $x, $r.T + $y); Start-Sleep -Milliseconds 150
  [W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80; [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 400
}

function Key-Down([string]$k) { $vk = $script:Vk[$k]; [W]::keybd_event([byte]$vk, [byte][W]::MapVirtualKey($vk, 0), 0, [UIntPtr]::Zero) }
function Key-Up([string]$k)   { $vk = $script:Vk[$k]; [W]::keybd_event([byte]$vk, [byte][W]::MapVirtualKey($vk, 0), 2, [UIntPtr]::Zero) }

function Chord([string[]]$keys, [int]$gapMs = 120, [int]$holdMs = 150) {   # aperta na ordem, solta na ordem inversa
  Assert-Foreground
  foreach ($k in $keys) { Key-Down $k; Start-Sleep -Milliseconds $gapMs }
  Start-Sleep -Milliseconds $holdMs
  for ($i = $keys.Count - 1; $i -ge 0; $i--) { Key-Up $keys[$i]; Start-Sleep -Milliseconds 40 }
  Start-Sleep -Milliseconds 400
}

function Stop-App {   # encerra so o pid iniciado aqui e os filhos dele (capturas); nunca por nome
  $app = (Get-AppProc).Id
  Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $app } | ForEach-Object { Stop-Process -Id $_.ProcessId }
  Stop-Process -Id $app; Start-Sleep -Seconds 1
  "encerrado pid=$app"
}
