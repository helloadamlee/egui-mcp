param(
    [string]$Message
)

$pipeName = "\\.\pipe\egui_mcp_client"

try {
    # Create named pipe client
    $pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "egui_mcp_client", [System.IO.Pipes.PipeDirection]::InOut)
    
    # Connect to the pipe
    Write-Host "Connecting to pipe..."
    $pipe.Connect(5000)  # 5 second timeout
    Write-Host "Connected!"
    
    # Send the message
    $writer = New-Object System.IO.StreamWriter($pipe)
    $writer.AutoFlush = $true
    $writer.WriteLine($Message)
    
    # Read response
    $reader = New-Object System.IO.StreamReader($pipe)
    $response = $reader.ReadLine()
    Write-Host "Response: $response"
    
    # Cleanup
    $reader.Close()
    $writer.Close()
    $pipe.Close()
}
catch {
    Write-Host "Error: $_"
}
