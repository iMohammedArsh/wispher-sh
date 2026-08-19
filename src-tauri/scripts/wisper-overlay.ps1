param([string]$StateFile)

Add-Type -AssemblyName PresentationFramework

$window = New-Object Windows.Window
$window.WindowStyle = 'None'; $window.ResizeMode = 'NoResize'; $window.AllowsTransparency = $true
$window.Background = [Windows.Media.Brushes]::Transparent; $window.Topmost = $true
$window.ShowInTaskbar = $false; $window.ShowActivated = $false; $window.Focusable = $false
$window.Width = 36; $window.Height = 18

$border = New-Object Windows.Controls.Border
$border.CornerRadius = [Windows.CornerRadius]::new(10); $border.BorderThickness = [Windows.Thickness]::new(0)
$border.Background = [Windows.Media.SolidColorBrush]::new([Windows.Media.Color]::FromRgb(4, 4, 4))
$root = New-Object Windows.Controls.StackPanel
$root.Orientation = 'Horizontal'; $root.VerticalAlignment = 'Center'; $root.HorizontalAlignment = 'Center'
$border.Child = $root; $window.Content = $border
$border.Cursor = [Windows.Input.Cursors]::Hand
$border.Add_MouseLeftButtonUp({
  [Console]::Out.WriteLine('toggle'); [Console]::Out.Flush(); $_.Handled = $true
})
$waves = @()
$currentState = ''

function Set-Position {
  $area = [Windows.SystemParameters]::WorkArea
  $window.Left = $area.Left + 18
  $window.Top = $area.Bottom - $window.Height - 18
}

function Set-Size([double]$width, [double]$height) {
  $window.Width = $width; $window.Height = $height
  Set-Position
}

function Set-State([string]$state) {
  if ($state -eq $script:currentState) { return }
  $script:currentState = $state
  $root.Children.Clear(); $script:waves = @()
  if ($state -eq 'recording') {
    Set-Size 130 34
    $mic = New-Object Windows.Controls.TextBlock
    $mic.FontFamily = New-Object Windows.Media.FontFamily('Segoe MDL2 Assets'); $mic.Text = [char]0xE720
    $mic.FontSize = 13; $mic.Foreground = [Windows.Media.Brushes]::White; $mic.VerticalAlignment = 'Center'
    $root.Children.Add($mic) | Out-Null
    $divider = New-Object Windows.Shapes.Rectangle
    $divider.Width = 1; $divider.Height = 14; $divider.Margin = [Windows.Thickness]::new(9,0,9,0)
    $divider.Fill = [Windows.Media.SolidColorBrush]::new([Windows.Media.Color]::FromRgb(43, 43, 43))
    $root.Children.Add($divider) | Out-Null
    $wavePanel = New-Object Windows.Controls.StackPanel
    $wavePanel.Orientation = 'Horizontal'; $wavePanel.VerticalAlignment = 'Center'; $wavePanel.HorizontalAlignment = 'Center'
    0..10 | ForEach-Object { $bar = New-Object Windows.Shapes.Rectangle; $bar.Width = 2; $bar.Height = 4; $bar.RadiusX = 2; $bar.RadiusY = 2; $bar.Margin = [Windows.Thickness]::new(2,0,2,0); $bar.Fill = [Windows.Media.SolidColorBrush]::new([Windows.Media.Color]::FromRgb(169, 226, 187)); $bar.VerticalAlignment = 'Center'; $script:waves += $bar; $wavePanel.Children.Add($bar) | Out-Null }
    $root.Children.Add($wavePanel) | Out-Null
  } elseif ($state -eq 'processing') {
    Set-Size 36 18
  } else {
    Set-Size 36 18
    $idleMic = New-Object Windows.Controls.TextBlock
    $idleMic.FontFamily = New-Object Windows.Media.FontFamily('Segoe MDL2 Assets'); $idleMic.Text = [char]0xE720
    $idleMic.FontSize = 9; $idleMic.Foreground = [Windows.Media.SolidColorBrush]::new([Windows.Media.Color]::FromRgb(143, 158, 147))
    $idleMic.VerticalAlignment = 'Center'; $root.Children.Add($idleMic) | Out-Null
  }
  Set-Position
}

function Set-Level([double]$level) {
  if ($script:waves.Count -eq 0) { return }
  # Speech RMS levels are intentionally small. A square-root response makes quiet
  # speech readable while preserving a clear difference between silence and voice.
  $normalized = [Math]::Min(1, [Math]::Sqrt([Math]::Max(0, $level)) * 4.8)
  $peak = 4 + ($normalized * 22)
  for ($index = 0; $index -lt $script:waves.Count; $index++) {
    $shape = 0.25 + (0.75 * [Math]::Sin(($index + 1) * 1.31) * [Math]::Sin(($index + 1) * 1.31))
    $script:waves[$index].Height = [Math]::Max(3, $peak * $shape)
  }
}

$window.Add_Loaded({ Set-State 'idle' })
$window.Show()

# Poll from the WPF dispatcher itself. This avoids the unreliable WSL-to-Windows
# stdin bridge and means the overlay continues to reflect dictation state even when
# the main workspace is never opened.
$timer = New-Object Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromMilliseconds(45)
$lastSnapshot = ''
$timer.Add_Tick({
  if ([string]::IsNullOrWhiteSpace($StateFile) -or -not (Test-Path -LiteralPath $StateFile)) { return }
  try {
    $snapshot = [System.IO.File]::ReadAllText($StateFile).Trim()
    if ($snapshot -eq $lastSnapshot) { return }
    $lastSnapshot = $snapshot
    $parts = $snapshot.Split('|', 2)
    if ($parts.Count -ne 2) { return }
    Set-State $parts[0]
    if ($parts[0] -eq 'recording') { Set-Level ([double]$parts[1]) }
  } catch { }
})
$timer.Start()
[Windows.Threading.Dispatcher]::Run()
