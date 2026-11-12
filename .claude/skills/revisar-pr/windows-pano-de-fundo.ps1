# Pano de fundo do teste: uma janela sem borda, sempre no topo, de cor solida em cada monitor,
# para que a captura do Takeshot pegue so isto e nunca a tela do usuario.
# Monitor primario = vermelho, o outro = azul. Sai sozinho em 15 minutos; o chamador mata o pid.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class P { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[void][P]::SetProcessDPIAware()
$forms = @()
$n = 0
foreach ($s in [System.Windows.Forms.Screen]::AllScreens) {
  $n++
  $f = New-Object System.Windows.Forms.Form
  $f.FormBorderStyle = 'None'
  $f.StartPosition = 'Manual'
  $f.Bounds = $s.Bounds
  $f.TopMost = $true
  $f.ShowInTaskbar = $false
  if ($s.Primary) { $f.BackColor = [System.Drawing.Color]::FromArgb(220, 40, 40) } else { $f.BackColor = [System.Drawing.Color]::FromArgb(40, 60, 220) }
  $l = New-Object System.Windows.Forms.Label
  $l.Text = "FUNDO DO TESTE $n  " + $s.DeviceName
  $l.ForeColor = [System.Drawing.Color]::White
  $l.Font = New-Object System.Drawing.Font('Segoe UI', 40)
  $l.AutoSize = $true
  $l.Location = New-Object System.Drawing.Point(60, 60)
  $f.Controls.Add($l)
  $forms += $f
}
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 900000
$timer.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$timer.Start()
foreach ($f in $forms) { $f.Show() }
[System.Windows.Forms.Application]::Run()
